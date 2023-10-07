use std::{
    fs::{create_dir, File},
    io::{self, Write},
};

use anyhow::{anyhow, Result};
use reqwest::header::{ACCEPT, USER_AGENT};
use semver::VersionReq;
use serde_json::Value;
use sha1::{Digest, Sha1};
use url::Url;

use crate::serde::{Dist, PackageRsp};

const REGISTRY_URL: &str = "https://registry.npmjs.org";

pub struct Registry {
    client: reqwest::blocking::Client,
}

impl Registry {
    pub fn new() -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(ACCEPT, "application/vnd.npm.install-v1+json".parse()?);
        headers.insert(USER_AGENT, "npm-offline@v0.1.0".parse()?);

        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .default_headers(headers)
                .build()?,
        })
    }

    pub fn fetch_package_dist(
        &self,
        package: String,
        version: Option<VersionReq>,
    ) -> Result<(String, String, Dist)> {
        let rsp = self
            .client
            .get(format!("{}/{}", REGISTRY_URL, package))
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

        let body: PackageRsp = rsp.json()?;
        if version.is_none() {
            let res = body.dist_tags.get("latest");
            // * Note: shouldn't enter here => https://github.com/npm/registry/blob/master/docs/responses/package-metadata.md#full-metadata-format
            if res.is_none() {
                return Err(anyhow!("no latest version found in dist-tag response"));
            }

            for (vrs, ver) in body.versions {
                if &vrs == res.unwrap() {
                    return Ok((package, vrs, ver.dist));
                }
            }

            return Err(anyhow!("no version found for latest dist-tag"));
        }

        let version = version.unwrap();
        let mut found_version: Option<String> = None;
        for (vrs, ver) in body.versions {
            let matched = version.matches(&semver::Version::parse(&vrs)?);
            if !matched && found_version.is_some() {
                return Ok((package, found_version.unwrap(), ver.dist));
            }

            if matched {
                found_version = Some(vrs);
            }
        }

        Err(anyhow!("no version found for version requirement"))
    }

    pub fn download_dist(&self, tarball_sum: String, url: String) -> Result<String> {
        let parsed_url = Url::parse(&url)?;
        let filename = parsed_url
            .path_segments()
            .ok_or(anyhow!("failed to parse path segements from url: {}", url))?
            .last()
            .ok_or(anyhow!("failed to get last path segment in url: {}", url))?;

        let res = self.client.get(url).send()?;

        let dir = std::env::temp_dir().join("npm-offline");
        if !dir.exists() {
            create_dir(&dir)?;
        }
        let file = dir.join(filename);

        let mut hasher = Sha1::new();
        if file.exists() {
            let mut f = File::open(&file)?;
            io::copy(&mut f, &mut hasher)?;

            if base16ct::lower::encode_string(&hasher.finalize()) == tarball_sum {
                println!("{} already exists", filename);
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
