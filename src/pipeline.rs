use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Str, Value};

/// Result of a complete Aster project build.
pub struct BuildResult {
    /// Whether any compilation errors occurred.
    pub has_errors: bool,
    /// Paths of all successfully written output files.
    pub outputs: Vec<PathBuf>,
}

/// Return a complete `_aster` protocol value with empty collections.
fn empty_aster() -> Value {
    Value::Dict(Dict::from_iter([
        (Str::from("protocol"), Value::Int(1)),
        (Str::from("collections"), Value::Dict(Dict::new())),
    ]))
}

/// Execute the full Aster build lifecycle.
///
/// 1. Load content collections (Phase 1)
/// 2. Assemble final Typst inputs (config + `_aster` protocol)
/// 3. Compile all page templates and write output (Phase 2)
/// 4. Report success / failure
pub fn build(root: &Path, config: Dict) -> Result<BuildResult> {
    // --- Phase 1: content collections ---
    let content_dir = crate::project::content_dir(root);
    let aster_value = if content_dir.is_dir() {
        match crate::content::load_collections(&content_dir, root, config.clone()) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("warning: failed to load content collections: {err}");
                empty_aster()
            }
        }
    } else {
        empty_aster()
    };

    // --- Assemble final inputs (config + _aster) ---
    let final_inputs = {
        let mut data: Vec<(Str, Value)> = config.into_iter().collect();
        data.push((Str::from("_aster"), aster_value));
        Dict::from_iter(data)
    };

    // --- Phase 2: pages ---
    let src_dir = crate::project::src_dir(root);
    if !src_dir.is_dir() {
        bail!("src/ directory not found in project");
    }

    let entries = crate::project::find_typ_files(&src_dir).context("failed to scan src/")?;
    if entries.is_empty() {
        bail!("no .typ files found in src/");
    }

    let mut result = BuildResult {
        has_errors: false,
        outputs: Vec::new(),
    };

    for entry in &entries {
        let output =
            crate::project::page_output_path(entry, root).expect("file must be under src/");

        match crate::compile::run(entry, root, final_inputs.clone()) {
            Ok(html) => {
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create directory {}", parent.display())
                    })?;
                }
                std::fs::write(&output, &html)
                    .with_context(|| format!("failed to write {}", output.display()))?;
                result.outputs.push(output);
            }
            Err(_) => {
                result.has_errors = true;
            }
        }
    }

    Ok(result)
}
