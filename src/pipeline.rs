use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Str, Value};
use typst::utils::LazyHash;
use typst_html::{HtmlDocument, HtmlOptions};

use crate::project::ProjectRoot;
use crate::{compile, route, transform, world};

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
/// 1. Load content collections (Phase 1) — uses config inputs
/// 2. Assemble base Typst inputs (config + `_aster` protocol)
/// 3. Probe dynamic templates (`[slug].typ`) for route declarations
/// 4. Compile all pages — static once, dynamic once per route entry
/// 5. Serialize and write output
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

    // Base Typst inputs — reused for both static and dynamic pages.
    let mut base_inputs: Vec<(Str, Value)> = config.into_iter().collect();
    base_inputs.push((Str::from("_aster"), aster_value));

    let page_library = LazyHash::new(world::build_library(Dict::from_iter(base_inputs.clone())));

    if !project.src_dir().is_dir() {
        bail!("src/ directory not found in project");
    }

    let mut result = BuildResult {
        has_errors: false,
        outputs: Vec::new(),
    };

    // --- Probe phase: extract routes from [slug] templates ---
    let mut static_entries: Vec<PathBuf> = Vec::new();
    let mut route_entries: Vec<(PathBuf, Vec<route::ParamSet>)> = Vec::new();

    for entry in project
        .walk_src()
        .filter(|p| p.extension().is_some_and(|ext| ext == "typ"))
    {
        let has_slug_param = entry
            .file_stem()
            .and_then(|s| s.to_str())
            .map_or(false, |s| s.contains('[') && s.contains(']'));

        if has_slug_param {
            // Probe: compile to Content, extract route metadata.
            match builder.content(&entry, project, &page_library) {
                Ok(content) => {
                    let routes = route::extract(&content);
                    if routes.is_empty() {
                        eprintln!(
                            "warning: {} has `[slug]` pattern but no `<route>` metadata",
                            entry.display()
                        );
                    } else {
                        route_entries.push((entry, routes));
                    }
                }
                Err(e) => {
                    bail!("failed to probe {}: {e:#}", entry.display());
                }
            }
        } else {
            static_entries.push(entry);
        }
    }

    // --- Render phase ---
    let mut page_docs: Vec<(PathBuf, HtmlDocument)> = Vec::new();

    // Static pages: compile once per template.
    for entry in &static_entries {
        let output = project
            .page_output_path(entry)
            .expect("file must be under src/");

        match builder.document(entry, project, &page_library) {
            Ok(mut doc) => {
                let ctx = transform::ProcessingContext::new(project, output.clone());
                if let Err(err) = transform::process_document(&mut doc, &ctx) {
                    eprintln!("error: post-processing failed: {err:#}");
                    result.has_errors = true;
                }
                page_docs.push((output, doc));
            }
            Err(_) => {
                result.has_errors = true;
            }
        }
    }

    // Dynamic pages: compile once per route entry.
    for (template, param_sets) in &route_entries {
        for params in param_sets {
            let mut route_inputs = base_inputs.clone();
            for (name, value) in params {
                route_inputs.push((
                    Str::from(name.as_str()),
                    Value::Str(Str::from(value.as_str())),
                ));
            }
            let route_library = LazyHash::new(world::build_library(Dict::from_iter(route_inputs)));

            let output = route::output_path(project, template, params);

            match builder.document(template, project, &route_library) {
                Ok(mut doc) => {
                    let ctx = transform::ProcessingContext::new(project, output.clone());

                    if let Err(err) = transform::process_document(&mut doc, &ctx) {
                        eprintln!("error: post-processing failed: {err:#}");
                        result.has_errors = true;
                    }
                    page_docs.push((output, doc));
                }
                Err(_) => {
                    result.has_errors = true;
                }
            }
        }
    }

    // --- Serialize ---
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
