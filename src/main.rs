use std::process::ExitCode;

use anyhow::{Context, Result};
use aster::Project;
use clap::Parser;

mod cli;

use crate::cli::{diag, init, watch};

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
        Commands::Init { path } => {
            let outcome = init::run(path)?;
            diag::emit_initialized(&outcome.project);
        }
        Commands::Build { project_dir } => {
            diag::report_build(&aster::build(resolve_project(project_dir)?)?)
        }
        Commands::Watch { project_dir } => watch::run(resolve_project(project_dir)?)?,
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_project(project_dir: Option<std::path::PathBuf>) -> Result<Project> {
    match project_dir {
        Some(dir) => {
            let dir = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir()
                    .context("failed to get current directory")?
                    .join(dir)
            };
            Project::open(dir)
        }
        None => {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            Project::find(&cwd).context("no aster.toml found in current or parent directories")
        }
    }
}
