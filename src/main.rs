mod cli;
mod compile;
mod project;
mod world;

use anyhow::{Context, Result, bail};
use clap::Parser;
use termcolor::{ColorChoice, StandardStream};
use typst_kit::diagnostics::DiagnosticFormat;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build => build()?,
    }
    Ok(())
}

fn build() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;

    let root = project::find_root(&cwd)
        .context("no aster.toml found in current or parent directories")?;

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

        match compile::run(entry, &root) {
            Ok(html) => {
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("failed to create directory {}", parent.display()))?;
                }
                std::fs::write(&output, &html)
                    .with_context(|| format!("failed to write {}", output.display()))?;
            }
            Err(err) => {
                has_errors = true;
                let relative = relative.with_extension("html");
                eprintln!("error: failed to build {}", relative.display());
                let mut writer = StandardStream::stderr(ColorChoice::Auto);
                if typst_kit::diagnostics::emit(
                    &mut writer,
                    &err.world,
                    &err.diagnostics,
                    DiagnosticFormat::Human,
                )
                .is_err()
                {
                    for diag in &err.diagnostics {
                        eprintln!("error: {diag:?}");
                    }
                }
            }
        }
    }

    if has_errors {
        bail!("some files failed to compile");
    }

    Ok(())
}
