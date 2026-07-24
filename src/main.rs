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
    Build,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build => build()?,
    }
    Ok(())
}

fn build() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let root =
        project::find_root(&cwd).context("no aster.toml found in current or parent directories")?;

    let config = project::parse_config(&root.join("aster.toml"))
        .map_err(|e| anyhow::anyhow!("failed to parse aster.toml: {e}"))?;

    let result = pipeline::build(&root, config)?;
    if result.has_errors {
        bail!("some files failed to compile");
    }
    Ok(())
}
