use clap::{Parser, Subcommand};
use commands::{download, publish};

mod commands;
mod errors;
mod macros;
mod registry;
mod serde;
mod version;
mod utils;

/// Download NodeJS dependencies from an npm registry for offline use
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Remote registry to use
    #[arg(short, long)]
    registry: Option<String>,

    #[command(subcommand)]
    subcommands: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Download tarballs dependencies from an npm registry for a given package
    Download {
        /// List of packages with their version (express@4.18.2) or list of "package.json" files.
        /// Space separated
        #[arg(required = true)]
        packages: Vec<String>,

        /// Output directory for all tarballs
        #[arg(short = 'o', long, default_value_t = String::from("./packages"))]
        output: String,

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
    },

    Resolve {
        packages: Vec<String>,
    },
}

fn main() {
    let args = Cli::parse();

    let remote_registry = args.registry;

    let res = match args.subcommands {
        Subcommands::Download {
            packages,
            output,
            dev_dependencies,
            peer_dependencies,
            optional_dependencies,
            compress,
            dispatch_sub_dependencies,
        } => download(
            packages,
            output,
            dev_dependencies,
            peer_dependencies,
            optional_dependencies,
            remote_registry,
            compress,
            dispatch_sub_dependencies,
        ),
        Subcommands::Publish { packages } => publish(packages, remote_registry),
        Subcommands::Resolve { packages: _ } => todo!(),
    };

    if let Err(e) = res {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
