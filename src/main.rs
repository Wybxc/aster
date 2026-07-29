mod compile;
mod config;
mod content;
mod pipeline;
mod project;
mod route;
mod transform;
mod world;

use anyhow::{Context, Result, bail};
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

    let config =
        config::parse_config(&project.config_file()).context("failed to parse aster.toml")?;

    let (outputs, errors) = pipeline::build(&project, config)?;

    for err in &errors {
        world::emit_message(
            &world::NullWorld,
            typst::diag::Severity::Error,
            &format!("{err:#}"),
        );
    }

    if !errors.is_empty() {
        eprintln!("error: {} page(s) failed to compile", errors.len());
        bail!("build failed");
    }

    eprintln!("built {} page(s)", outputs.len());
    Ok(())
}
