use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

mod cli;

use crate::cli::{build, dev, diag, init, watch};

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Create a new Aster project
    Init {
        /// Directory to initialize. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: std::path::PathBuf,
    },
    /// Build the project
    Build {
        /// Project root directory (containing aster.toml).
        /// Defaults to the nearest ancestor with aster.toml from cwd.
        #[arg(short = 'p', long = "project")]
        project_dir: Option<std::path::PathBuf>,
    },
    /// Build and serve the project with automatic browser refresh
    Dev {
        /// Project root directory (containing aster.toml).
        /// Defaults to the nearest ancestor with aster.toml from cwd.
        #[arg(short = 'p', long = "project")]
        project_dir: Option<std::path::PathBuf>,
        /// Address on which to serve the project.
        #[arg(long, default_value = "127.0.0.1")]
        host: std::net::IpAddr,
        /// Port on which to serve the project.
        #[arg(long, default_value_t = 4321)]
        port: u16,
    },
    /// Build the project and rebuild when its inputs change
    Watch {
        /// Project root directory (containing aster.toml).
        /// Defaults to the nearest ancestor with aster.toml from cwd.
        #[arg(short = 'p', long = "project")]
        project_dir: Option<std::path::PathBuf>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(exit) => exit,
        Err(error) => {
            diag::emit_error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Init { path } => init::run(path)?,
        Commands::Build { project_dir } => build::run(project_dir)?,
        Commands::Dev {
            project_dir,
            host,
            port,
        } => dev::run(project_dir, host, port)?,
        Commands::Watch { project_dir } => watch::run(project_dir)?,
    }
    Ok(ExitCode::SUCCESS)
}
