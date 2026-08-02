use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

mod cli;

use crate::cli::{build, diag, init, watch};

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
        Commands::Watch { project_dir } => watch::run(project_dir)?,
    }
    Ok(ExitCode::SUCCESS)
}
