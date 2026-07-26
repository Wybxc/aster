use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Str, Value};
use typst::utils::LazyHash;

use crate::compile;

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
/// 1. Load content collections (Phase 1) — uses `config` as inputs
/// 2. Assemble final Typst inputs (config + `_aster` protocol)
/// 3. Compile all page templates and write output (Phase 2) — uses final inputs
/// 4. Report success / failure
pub fn build(root: &Path, config: Dict) -> Result<BuildResult> {
    // World builder — fonts scanned once for the whole project.
    let builder = compile::CompileContext::new(root);

    // --- Phase 1: content collections (config inputs only) ---
    let content_library = LazyHash::new(compile::build_library(config.clone()));

    let content_dir = crate::project::content_dir(root);
    let aster_value = if content_dir.is_dir() {
        match crate::content::load_collections(&content_dir, root, &builder, &content_library) {
            Ok(v) => v,
            Err(err) => bail!("error: failed to load content collections: {err}"),
        }
    } else {
        empty_aster()
    };

    // --- Assemble final inputs (config + _aster) ---
    let page_library = {
        let mut data: Vec<(Str, Value)> = config.into_iter().collect();
        data.push((Str::from("_aster"), aster_value));
        LazyHash::new(compile::build_library(Dict::from_iter(data)))
    };

    // --- Phase 2: pages (config + _aster inputs) ---
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

        match builder.page(entry, root, &page_library) {
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

    // --- Phase 3: CSS assets ---
    crate::css::run(
        &crate::project::src_dir(root),
        &crate::project::output_dir(root),
    )?;

    Ok(result)
}
