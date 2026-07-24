mod compile;
mod project;
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

    // Parse aster.toml and expose its contents as sys.inputs.
    let config_path = root.join("aster.toml");
    let inputs = world::parse_config(&config_path)
        .map_err(|e| anyhow::anyhow!("failed to parse aster.toml: {e}"))?;

    let src_dir = root.join("src");
    if !src_dir.is_dir() {
        bail!("src/ directory not found in project");
    }

    let entries = project::find_typ_files(&src_dir).context("failed to scan src/")?;

    if entries.is_empty() {
        bail!("no .typ files found in src/");
    }

    let mut has_errors = false;

    for entry in &entries {
        let relative = entry
            .strip_prefix(&src_dir)
            .expect("file must be under src/");
        let output = root.join("dist").join(relative).with_extension("html");

        match compile::run(entry, &root, inputs.clone()) {
            Ok(html) => {
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create directory {}", parent.display())
                    })?;
                }
                std::fs::write(&output, &html)
                    .with_context(|| format!("failed to write {}", output.display()))?;
            }
            Err(_) => {
                has_errors = true;
            }
        }
    }

    if has_errors {
        bail!("some files failed to compile");
    }

    Ok(())
}
