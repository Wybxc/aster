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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { project_dir } => build(project_dir)?,
    }
    Ok(())
}

fn build(project_dir: Option<std::path::PathBuf>) -> Result<()> {
    let project = match project_dir {
        Some(dir) => {
            let dir = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir()
                    .context("failed to get current directory")?
                    .join(dir)
            };
            project::ProjectRoot::new(dir)?
        }
        None => {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            project::ProjectRoot::find(&cwd)
                .context("no aster.toml found in current or parent directories")?
        }
    };

    let aster_config =
        config::AsterConfig::load(&project.config_file()).context("failed to parse aster.toml")?;

    let mut driver = pipeline::BuildDriver::new(project.clone());
    let outcome = driver.build(aster_config)?;
    for warning in &outcome.warnings {
        diag::emit_warning(warning);
    }
    for output in &outcome.outputs {
        let relative = output.strip_prefix(project.output_dir()).unwrap_or(output);
        diag::emit_page(&relative.to_string_lossy());
    }
    diag::emit_summary(outcome.outputs.len(), outcome.elapsed);
    Ok(())
}
