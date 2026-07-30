mod compile;
mod config;
mod content;
mod diag;
mod output;
mod pipeline;
mod project;
mod route;
mod transform;
mod utils;
mod watch;

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { project_dir } => build(project_dir)?,
        Commands::Watch { project_dir } => watch::run(resolve_project(project_dir)?)?,
    }
    Ok(())
}

fn build(project_dir: Option<std::path::PathBuf>) -> Result<()> {
    let project = resolve_project(project_dir)?;
    let aster_config =
        config::AsterConfig::load(&project.config_file()).context("failed to parse aster.toml")?;

    let mut driver = pipeline::BuildDriver::new(project.clone());
    let outcome = driver.build(aster_config)?;
    report_outcome(&outcome);
    Ok(())
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

pub(crate) fn report_outcome(outcome: &pipeline::BuildOutcome) {
    for warning in &outcome.warnings {
        diag::emit_warning(warning);
    }
    diag::emit_summary(outcome.outputs.len(), outcome.elapsed);
}
