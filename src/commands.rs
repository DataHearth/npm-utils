use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::PathBuf,
};

use semver::VersionReq;

use crate::{
    errors::CustomErrors,
    registry::{manuel_extend, Registry},
    serde::PackageJson,
    utils::split_package_string,
    version::parse,
};

/// List of versions of a package
type PackageVersions = BTreeSet<Option<Vec<VersionReq>>>;
type PackageList = BTreeMap<String, PackageVersions>;

pub(super) fn download(
    args: Vec<String>,
    output: String,
    dev: bool,
    peer: bool,
    optional: bool,
    registry: Option<String>,
    _compress: bool,
    dispatch: bool,
) -> Result<(), CustomErrors> {
    let mut pkgs: BTreeMap<String, HashSet<Option<Vec<VersionReq>>>> = BTreeMap::new();

    for arg in args {
        let mut path = PathBuf::from(arg.clone());
        if !path.exists() {
            let (name, version) = split_package_string(arg)?;
            pkgs.entry(name).or_default().insert(version);

            continue;
        }

        if path.is_dir() {
            path = path.join("package.json");
            if !path.exists() {
                return Err(CustomErrors::PackageJsonParse(format!(
                    "package.json not found in directory {}",
                    path.display()
                )));
            }
        }

        let pkg_json: PackageJson = serde_json::from_str(
            &fs::read_to_string(path).map_err(|e| CustomErrors::Fs(e.to_string()))?,
        )
        .map_err(|e| CustomErrors::Fs(e.to_string()))?;
        for (name, version) in pkg_json.dependencies {
            pkgs.entry(name)
                .or_default()
                .insert(if version == "latest" {
                    None
                } else {
                    Some(parse(&version)?)
                });
        }
        if dev {
            for (name, version) in pkg_json.dev_dependencies {
                pkgs.entry(name)
                    .or_default()
                    .insert(if version == "latest" {
                        None
                    } else {
                        Some(parse(&version)?)
                    });
            }
        }
        if peer {
            for (name, version) in pkg_json.peer_dependencies {
                pkgs.entry(name)
                    .or_default()
                    .insert(if version == "latest" {
                        None
                    } else {
                        Some(parse(&version)?)
                    });
            }
        }
        if optional {
            for (name, version) in pkg_json.optional_dependencies {
                pkgs.entry(name)
                    .or_default()
                    .insert(if version == "latest" {
                        None
                    } else {
                        Some(parse(&version)?)
                    });
            }
        }
    }

    let registry = Registry::new(registry)?;

    let mut tbd = HashMap::new();

    for (name, version) in pkgs {
        for v in version {
            println!("{name}: Resolving dependencies...");

            manuel_extend(
                registry.fetch_dependencies(name.clone(), v, dev, peer, optional, dispatch)?,
                &mut tbd,
            );

            println!("{name}: Resolved");
        }
    }

    println!("Downloading {} packages...", tbd.len());

    tbd.iter().for_each(|(package, versions)| {
        versions.iter().for_each(|(tag, manifest)| {
            let x = registry.download_tarball(
                manifest.dist.shasum.to_owned(),
                manifest.dist.tarball.to_owned(),
                &output,
            );

            if let Err(e) = x {
                println!("{package}@{tag}: Failed to download => {e}");
            }
        });
    });

    println!("Packages downloaded!");

    Ok(())
}

#[allow(dead_code)]
pub(super) fn publish(_pkgs: Vec<String>, _registry: Option<String>) -> Result<(), CustomErrors> {
    todo!()
}
