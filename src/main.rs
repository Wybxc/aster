mod compile;
mod content;
mod html;
mod project;
mod world;

use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Parser;
use typst::foundations::{Dict, Str, Value};

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

    // --- Phase 0: parse config ---
    let config_path = root.join("aster.toml");
    let config_inputs = world::parse_config(&config_path)
        .map_err(|e| anyhow::anyhow!("failed to parse aster.toml: {e}"))?;

    // --- Phase 1: content collections ---
    let collections_value = load_collections(&root, config_inputs.clone());

    // Build the `_aster` payload.
    let aster_payload = Dict::from_iter([
        (Str::from("protocol"), Value::Int(1)),
        (Str::from("collections"), collections_value),
    ]);

    // Final inputs: config + _aster.
    let mut final_inputs_data: Vec<(Str, Value)> = config_inputs.clone().into_iter().collect();
    final_inputs_data.push((Str::from("_aster"), Value::Dict(aster_payload)));
    let final_inputs = Dict::from_iter(final_inputs_data);

    // --- Phase 2: pages ---
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

        match compile::run(entry, &root, final_inputs.clone()) {
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

/// Load all content collections from `content/`.  Returns an empty dict when
/// the directory doesn't exist.
fn load_collections(root: &Path, config_inputs: Dict) -> Value {
    let content_dir = root.join("content");
    if !content_dir.is_dir() {
        return Value::Dict(Dict::new());
    }

    match content::load_collections(&content_dir, root, config_inputs) {
        Ok(dict) => Value::Dict(dict),
        Err(err) => {
            // Print collection errors and continue with empty collections.
            eprintln!("warning: failed to load content collections: {err}");
            Value::Dict(Dict::new())
        }
    }
}
