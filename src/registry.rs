use std::{
    collections::HashMap,
    fs::{create_dir, File},
    io::{self, Write},
    path::Path,
};

use reqwest::header::{HeaderValue, ACCEPT, USER_AGENT};
use semver::VersionReq;
use serde_json::Value;
use sha1::{Digest, Sha1};
use url::Url;

use crate::{
    errors::CustomErrors,
    hashmap, hashmap_ext_cond, headers,
    serde::{PackageRsp, Version},
    utils::find_version,
    version::parse,
};

const REGISTRY_URL: &str = "https://registry.npmjs.org";

pub(super) struct Registry {
    client: reqwest::blocking::Client,
    registry: String,
}

impl Registry {
    pub(super) fn new(registry: Option<String>) -> Result<Self, CustomErrors> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .default_headers(headers!(
                    (ACCEPT, "application/vnd.npm.install-v1+json"),
                    (USER_AGENT, "npm-offline@v0.1.0")
                ))
                .build()
                .map_err(|e| CustomErrors::HttpClient(e.to_string()))?,
            registry: registry.unwrap_or_else(|| REGISTRY_URL.to_string()),
        })
    }

    /// Fetch package version and its dependencies from registry.
    /// First entry in returned vector is the top-level package
    pub(super) fn fetch_dependencies(
        &self,
        package: String,
        version_req: Option<Vec<VersionReq>>,
        dev: bool,
        peer: bool,
        optional: bool,
        dispatch: bool,
    ) -> Result<HashMap<String, HashMap<String, Version>>, CustomErrors> {
        let rsp = self
            .client
            .get(format!("{}/{}", self.registry, package))
            .send()
            .map_err(|e| CustomErrors::PackageManifestFetch(e.to_string()))?;

        if !rsp.status().is_success() {
            return Err(CustomErrors::PackageManifestFetch(
                rsp.json::<serde_json::Value>()
                    .map_err(|e| CustomErrors::BodyParse("JSON".to_string(), e.to_string()))?
                    .as_object()
                    .unwrap_or(&serde_json::Map::new())
                    .get("error")
                    .unwrap_or(&Value::String("no error in body".to_string()))
                    .to_string(),
            ));
        }

        let body = rsp
            .json::<PackageRsp>()
            .map_err(|e| CustomErrors::BodyParse("JSON".to_string(), e.to_string()))?;

        let pkg_version = find_version(
            body.versions,
            version_req,
            body.dist_tags.get("latest").map(|v| v.as_str()),
        )?
        .ok_or(CustomErrors::Version(format!(
            "no version found for {package}@latest"
        )))?;

        let mut deps = hashmap!((
            package,
            hashmap!((pkg_version.version.clone(), pkg_version.clone()))
        ));
        for (dep, version) in hashmap_ext_cond!(
            (true, pkg_version.dependencies),
            (dev, pkg_version.dev_dependencies),
            (peer, pkg_version.peer_dependencies),
            (optional, pkg_version.optional_dependencies)
        ) {
            match parse(&version) {
                Ok(v) => manuel_extend(
                    self.fetch_dependencies(
                        dep,
                        Some(v),
                        dev && dispatch,
                        peer && dispatch,
                        optional && dispatch,
                        dispatch,
                    )?,
                    &mut deps,
                ),
                Err(e) => eprintln!("{dep}@{version}: failed to parse requirement version {e}"),
            };
        }

        Ok(deps)
    }

    /// Download dependency tarball from registry.
    pub(super) fn download_tarball(
        &self,
        tarball_sum: String,
        url: String,
        output: &str,
    ) -> Result<String, CustomErrors> {
        let parsed_url = Url::parse(&url).map_err(|e| CustomErrors::Global(e.to_string()))?;
        let filename = parsed_url
            .path_segments()
            .ok_or(CustomErrors::Global(format!(
                "failed to parse path segments from url: {}",
                url
            )))?
            .last()
            .ok_or(CustomErrors::Global(format!(
                "failed to get last path segment in url: {}",
                url
            )))?;

        let res = self
            .client
            .get(url)
            .send()
            .map_err(|e| CustomErrors::PackageManifestFetch(e.to_string()))?;

        let dir = Path::new(output);
        if !dir.exists() {
            create_dir(&dir).map_err(|e| CustomErrors::Fs(e.to_string()))?;
        }
        let file = dir.join(filename);

        let mut hasher = Sha1::new();
        if file.exists() {
            let mut f = File::open(&file).map_err(|e| CustomErrors::Fs(e.to_string()))?;
            io::copy(&mut f, &mut hasher).map_err(|e| CustomErrors::Fs(e.to_string()))?;

            if base16ct::lower::encode_string(&hasher.finalize()) == tarball_sum {
                return Ok(file
                    .to_str()
                    .ok_or(CustomErrors::Global(
                        "failed to convert file path to string".to_string(),
                    ))?
                    .to_string());
            }

            println!("{}: checksum mismatch, redownloading...", filename);
        }

        let data = res
            .bytes()
            .map_err(|e| CustomErrors::BodyParse("BYTES".to_string(), e.to_string()))?;

        let mut f = File::create(&file).map_err(|e| CustomErrors::Fs(e.to_string()))?;
        f.write_all(&data)
            .map_err(|e| CustomErrors::Fs(e.to_string()))?;

        Ok(file
            .to_str()
            .ok_or(CustomErrors::Global(
                "failed to convert file path to string".to_string(),
            ))?
            .to_string())
    }
}

pub(super) fn manuel_extend(
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
