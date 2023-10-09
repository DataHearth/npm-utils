use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::Duration,
};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use indicatif::ProgressBar;
use semver::VersionReq;
use version::parser_multi_requirements;

use crate::{
    registry::{manuel_extend, Registry},
    serde::PackageJson,
};

mod registry;
mod serde;
mod version;

/// Download NodeJS dependencies from an npm registry for offline use
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    subcommands: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Fetch tarballs dependencies from an npm registry for a given package
    Fetch {
        /// List of packages with their version (express@4.18.2) or list of "package.json" files.
        /// Space separated
        #[arg(required = true)]
        packages: Vec<String>,

        /// Output directory for all tarballs
        #[arg(short = 'o', long, default_value_t = String::from("./packages"))]
        output: String,

        /// Remote registry to use
        #[arg(short, long)]
        registry: Option<String>,

        /// Include devDependencies
        #[arg(short = 'd', long)]
        dev_dependencies: bool,

        /// Include peerDependencies
        #[arg(long)]
        peer_dependencies: bool,

        /// Include optionalDependencies
        #[arg(long)]
        optional_dependencies: bool,

        #[arg(long)]
        dispatch_sub_dependencies: bool,

        /// Compress tarballs into a single one. Output path will be "output" the flag with ".tar.gz" extension
        #[arg(short, long)]
        compress: bool,
    },

    /// Publish tarballs dependencies to an npm registry
    Publish {
        /// List of tarballs path to publish. Path can be a directory. Space separated
        packages: Vec<String>,

        /// Remote registry to use
        #[arg(short, long)]
        registry: String,
    },
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.subcommands {
        Subcommands::Fetch {
            packages,
            output,
            dev_dependencies,
            peer_dependencies,
            optional_dependencies,
            registry,
            compress,
            dispatch_sub_dependencies,
        } => fetch(
            packages,
            output,
            dev_dependencies,
            peer_dependencies,
            optional_dependencies,
            registry,
            compress,
            dispatch_sub_dependencies,
        ),
        Subcommands::Publish {
            packages: _,
            registry: _,
        } => todo!(),
    }
}

fn fetch(
    args: Vec<String>,
    output: String,
    dev: bool,
    peer: bool,
    optional: bool,
    registry: Option<String>,
    _compress: bool,
    dispatch: bool,
) -> Result<()> {
    let mut pkgs: HashMap<String, HashSet<Option<Vec<VersionReq>>>> = HashMap::new();

    for arg in args {
        let mut path = PathBuf::from(arg.clone());
        if !path.exists() {
            let (name, version) = split_package_string(arg)?;
            pkgs.entry(name).or_default().insert(version);

            // TODO: get package manifest from registry and retrieve all dependencies
            continue;
        }
        if path.is_dir() {
            path = path.join("package.json");
            if !path.exists() {
                return Err(anyhow!(
                    "package.json not found in directory {}",
                    path.display()
                ));
            }
        }

        let pkg_json: PackageJson = serde_json::from_str(&fs::read_to_string(path)?)?;
        for (name, version) in pkg_json.dependencies {
            pkgs.entry(name)
                .or_default()
                .insert(if version == "latest" {
                    None
                } else {
                    Some(parser_multi_requirements(&version)?)
                });
        }
        if dev {
            for (name, version) in pkg_json.dev_dependencies {
                pkgs.entry(name)
                    .or_default()
                    .insert(if version == "latest" {
                        None
                    } else {
                        Some(parser_multi_requirements(&version)?)
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
                        Some(parser_multi_requirements(&version)?)
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
                        Some(parser_multi_requirements(&version)?)
                    });
            }
        }
    }

    let registry = Registry::new(registry)?;

    let mut tbd = HashMap::new();

    for (name, version) in pkgs {
        for v in version {
            let pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message(format!("{name}: Resolving dependencies"));

            manuel_extend(
                registry.fetch_package_deps(name.clone(), v, dev, peer, optional, dispatch)?,
                &mut tbd,
            );

            pb.finish_with_message(format!("{name}: Resolved"));
        }
    }

    let pb = ProgressBar::new(tbd.len() as u64);
    pb.set_message("Downloading packages...");

    tbd.iter().for_each(|(package, versions)| {
        // println!("Downloading: {name}@{version}", name = v.0, version = v.1)
        versions.iter().for_each(|(tag, manifest)| {
            pb.inc(1);
            let x = registry.download_dist(
                manifest.dist.shasum.to_owned(),
                manifest.dist.tarball.to_owned(),
                &output,
            );

            if x.is_ok() {
                // pb.println(format!("{package}@{tag}: Downloaded at {}", x.unwrap()));
            } else {
                pb.println(format!(
                    "{package}@{tag}: Failed to download => {}",
                    x.unwrap_err()
                ));
            }
        });
    });

    pb.finish_with_message("Packages downloaded!");

    Ok(())
}

#[allow(dead_code)]
fn publish(_pkgs: Vec<String>, _registry: String) {
    todo!()
}

/// Split a package string into a tuple of package name and version
fn split_package_string(package: String) -> Result<(String, Option<Vec<VersionReq>>)> {
    let mut splitted = package.split('@').collect::<Vec<&str>>();
    if splitted.len() > 3 {
        return Err(anyhow!(
            "package name can only contains a maximum of 2 '@'. Found {}",
            splitted.len() - 1
        ));
    } else if splitted.len() == 2 {
        splitted.remove(0);
    }

    let version = if splitted[1] == "latest" {
        None
    } else {
        Some(parser_multi_requirements(splitted[1])?)
    };

    return Ok((splitted[0].to_string(), version));
}
