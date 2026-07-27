use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Str, Value};
use typst::utils::LazyHash;
use typst_html::{HtmlDocument, HtmlOptions};

use crate::project::ProjectRoot;
use crate::{compile, transform, world};

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
/// 3. Compile page templates, run the document pipeline (CSS bundling,
///    image extraction, syntax highlighting), and write output (Phase 2)
/// 4. Report success / failure
pub fn build(project: &ProjectRoot, config: Dict) -> Result<BuildResult> {
    // World builder — fonts scanned once for the whole project.
    let builder = compile::CompileContext::new(project);

    // --- Phase 1: content collections (config inputs only) ---
    let content_library = LazyHash::new(world::build_library(config.clone()));

    let aster_value = if project.content_dir().is_dir() {
        match crate::content::load_collections(project, &builder, &content_library) {
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
        LazyHash::new(world::build_library(Dict::from_iter(data)))
    };

    // --- Phase 2: pages (config + _aster inputs) ---
    if !project.src_dir().is_dir() {
        bail!("src/ directory not found in project");
    }

    let entries: Vec<_> = project
        .walk_src()
        .filter(|p| p.extension().is_some_and(|ext| ext == "typ"))
        .collect();
    if entries.is_empty() {
        bail!("no .typ files found in src/");
    }

    let mut result = BuildResult {
        has_errors: false,
        outputs: Vec::new(),
    };

    let src_dir = project.src_dir();
    let output_dir = project.output_dir();
    let mut page_docs: Vec<(PathBuf, HtmlDocument)> = Vec::new();

    for entry in &entries {
        let output = project
            .page_output_path(entry)
            .expect("file must be under src/");

        match builder.document(entry, project, &page_library) {
            Ok(mut doc) => {
                let ctx = transform::ProcessingContext {
                    src_dir: src_dir.clone(),
                    dist_dir: output_dir.clone(),
                };
                if transform::process_document(&mut doc, &ctx).is_err() {
                    result.has_errors = true;
                }
                page_docs.push((output, doc));
            }
            Err(_) => {
                result.has_errors = true;
            }
        }
    }

    for (output, doc) in &page_docs {
        let raw = typst_html::html(doc, &HtmlOptions::default())
            .map_err(|_| anyhow::anyhow!("failed to encode HTML"))?;

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        std::fs::write(output, &raw)
            .with_context(|| format!("failed to write {}", output.display()))?;
        result.outputs.push(output.clone());
    }

    Ok(result)
}
