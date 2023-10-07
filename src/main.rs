use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use semver::VersionReq;

use crate::{registry::Registry, serde::PackageJson};

mod registry;
mod serde;

type ListPkgs = HashMap<String, HashSet<Option<VersionReq>>>;

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
        } => fetch(
            packages,
            output,
            dev_dependencies,
            peer_dependencies,
            optional_dependencies,
            registry,
            compress,
        ),
        Subcommands::Publish { packages, registry } => todo!(),
    }
}

fn fetch(
    args: Vec<String>,
    output: String,
    dev: bool,
    peer: bool,
    optional: bool,
    registry: Option<String>,
    compress: bool,
) -> Result<()> {
    let mut pkgs: ListPkgs = HashMap::new();

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
                    Some(VersionReq::parse(&version)?)
                });
        }
        if dev {
            for (name, version) in pkg_json.dev_dependencies {
                pkgs.entry(name)
                    .or_default()
                    .insert(if version == "latest" {
                        None
                    } else {
                        Some(VersionReq::parse(&version)?)
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
                        Some(VersionReq::parse(&version)?)
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
                        Some(VersionReq::parse(&version)?)
                    });
            }
        }
    }

    let registry = Registry::new()?;

    for (name, version) in pkgs {
        for v in version {
            let (pkg, version, dist) = registry.fetch_package_dist(name.clone(), v)?;
            println!("{}@{}", pkg, version);
            println!("  {}", dist.tarball);
            let f_path = registry.download_dist(dist.shasum, dist.tarball)?;
            println!("Tarball saved to {}", f_path);
        }
    }

    todo!()
}

fn publish(pkgs: Vec<String>, registry: String) {
    todo!()
}

/// Split a package string into a tuple of package name and version
fn split_package_string(package: String) -> Result<(String, Option<VersionReq>)> {
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
        Some(VersionReq::parse(splitted[1])?)
    };

    return Ok((splitted[0].to_string(), version));
}
