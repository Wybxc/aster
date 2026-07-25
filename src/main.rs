mod compile;
mod content;
mod highlight;
mod pipeline;
mod project;

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
    let root = match project_dir {
        Some(dir) => {
            let dir = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir()
                    .context("failed to get current directory")?
                    .join(dir)
            };
            if !dir.join("aster.toml").exists() {
                bail!("no aster.toml found in {:?}", dir);
            }
            dir
        }
        None => {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            project::find_root(&cwd)
                .context("no aster.toml found in current or parent directories")?
        }
    };

    let config = project::parse_config(&root.join("aster.toml"))
        .map_err(|e| anyhow::anyhow!("failed to parse aster.toml: {e}"))?;

    let result = pipeline::build(&root, config)?;
    if result.has_errors {
        bail!("some files failed to compile");
    }
    Ok(())
}
