mod compile;
mod config;
mod content;
mod diag;
mod init;
mod output;
mod pipeline;
mod project;
mod route;
mod transform;
mod utils;
mod watch;

use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

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
        Commands::Init { path } => init::run(path)?.report(),
        Commands::Build { project_dir } => build(project_dir)?.report(),
        Commands::Watch { project_dir } => watch::run(resolve_project(project_dir)?)?,
    }
    Ok(ExitCode::SUCCESS)
}

fn build(project_dir: Option<std::path::PathBuf>) -> Result<pipeline::BuildOutcome> {
    let project = resolve_project(project_dir)?;
    let aster_config =
        config::AsterConfig::load(&project.config_file()).context("failed to parse aster.toml")?;

    pipeline::BuildDriver::new(project).build(aster_config)
}

fn resolve_project(project_dir: Option<std::path::PathBuf>) -> Result<project::ProjectRoot> {
    match project_dir {
        Some(dir) => {
            let dir = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir()
                    .context("failed to get current directory")?
                    .join(dir)
            };
            project::ProjectRoot::new(dir)
        }
        None => {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            project::ProjectRoot::find(&cwd)
                .context("no aster.toml found in current or parent directories")
        }
    }
}
