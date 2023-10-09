use std::{
    collections::HashMap,
    fs::{create_dir, File},
    io::{self, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use semver::VersionReq;
use serde_json::Value;
use sha1::{Digest, Sha1};
use url::Url;

use crate::{
    serde::{PackageRsp, Version},
    version::{display_multi, parser_multi_requirements},
};

const REGISTRY_URL: &str = "https://registry.npmjs.org";

pub struct Registry {
    client: reqwest::blocking::Client,
    registry: String,
}

impl Registry {
    pub fn new(registry: Option<String>) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(ACCEPT, "application/vnd.npm.install-v1+json".parse()?);
        headers.insert(USER_AGENT, "npm-offline@v0.1.0".parse()?);

        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .default_headers(headers)
                .build()?,
            registry: registry.unwrap_or_else(|| REGISTRY_URL.to_string()),
        })
    }

    /// Fetch package version and its dependencies from registry.
    /// First entry in returned vector is the top-level package
    pub fn fetch_package_deps(
        &self,
        package: String,
        version_req: Option<Vec<VersionReq>>,
        dev: bool,
        peer: bool,
        optional: bool,
        dispatch: bool,
    ) -> Result<HashMap<String, HashMap<String, Version>>> {
        let rsp = self
            .client
            .get(format!("{}/{}", self.registry, package))
            .send()?;

        if !rsp.status().is_success() {
            let body: serde_json::Value = rsp.json()?;
            return Err(anyhow!(
                "failed to fetch package manifest: {}",
                body.as_object()
                    .unwrap_or(&serde_json::Map::new())
                    .get("error")
                    .unwrap_or(&Value::String("no error in body".to_string()))
            ));
        }

        let mut pkgs: HashMap<String, HashMap<String, Version>> = HashMap::new();
        let rsp: PackageRsp = rsp.json()?;
        let v = if let Some(version_req) = version_req {
            let mut found = None;
            for (tag, v) in rsp.versions {
                let parsed_v = semver::Version::parse(&tag)?;
                let matched = version_req
                    .iter()
                    .find(|req| req.matches(&parsed_v))
                    .is_some();
                if !matched && found.is_some() {
                    break;
                }

                if matched {
                    found = Some(v);
                }
            }

            if let Some(version) = found {
                version
            } else {
                return Err(anyhow!(
                    "no version found for {package}@{}",
                    display_multi(version_req)
                ));
            }
        } else {
            let tag = rsp
                .dist_tags
                .get("latest")
                .ok_or(anyhow!("latest dist-tag not found for {package}"))?;

            if let Some(v) = rsp.versions.get(tag) {
                v.to_owned()
            } else {
                return Err(anyhow!("{package}@latest alias {package}@{tag} not found"));
            }
        };

        let mut pkg_map = HashMap::new();
        pkg_map.insert(v.version.clone(), v.clone());
        pkgs.insert(package, pkg_map);

        // pkgs.push(version.clone());
        let mut deps = HashMap::new();
        deps.extend(v.dependencies);
        if dev {
            deps.extend(v.dev_dependencies);
        }
        if peer {
            deps.extend(v.peer_dependencies);
        }
        if optional {
            deps.extend(v.optional_dependencies);
        }

        for (dep, version) in deps {
            if let Ok(requirements) = parser_multi_requirements(&version) {
                let sub_deps = self.fetch_package_deps(
                    dep,
                    Some(requirements),
                    dev && dispatch,
                    peer && dispatch,
                    optional && dispatch,
                    dispatch,
                )?;
                manuel_extend(sub_deps, &mut pkgs);
            } else {
                println!("{dep}@{version}: failed to parse requirement version. Skipping...");
            }
        }

        Ok(pkgs)
    }

    pub fn download_dist(&self, tarball_sum: String, url: String, output: &str) -> Result<String> {
        let parsed_url = Url::parse(&url)?;
        let filename = parsed_url
            .path_segments()
            .ok_or(anyhow!("failed to parse path segments from url: {}", url))?
            .last()
            .ok_or(anyhow!("failed to get last path segment in url: {}", url))?;

        let res = self.client.get(url).send()?;

        let dir = Path::new(output);
        if !dir.exists() {
            create_dir(&dir)?;
        }
        let file = dir.join(filename);

        let mut hasher = Sha1::new();
        if file.exists() {
            let mut f = File::open(&file)?;
            io::copy(&mut f, &mut hasher)?;

            if base16ct::lower::encode_string(&hasher.finalize()) == tarball_sum {
                return Ok(file
                    .to_str()
                    .ok_or(anyhow!("failed to convert file path to string"))?
                    .to_string());
            }
        }

        let data = res.bytes()?;

        let mut f = File::create(&file)?;
        f.write_all(&data)?;
        Ok(file
            .to_str()
            .ok_or(anyhow!("failed to convert file path to string"))?
            .to_string())
    }
}

pub fn manuel_extend(
    src: HashMap<String, HashMap<String, Version>>,
    dst: &mut HashMap<String, HashMap<String, Version>>,
) {
    for (k, v) in src {
        if dst.contains_key(&k) {
            let versions = dst.get_mut(&k).unwrap();
            versions.extend(v);
        } else {
            dst.insert(k, v);
        }
    }
}
