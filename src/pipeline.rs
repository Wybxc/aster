use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::foundations::{Dict, Str, Value};
use typst::utils::LazyHash;
use typst_html::HtmlOptions;

use crate::project::ProjectRoot;
use crate::{compile, diag, route, transform, world};

/// Return a complete `_aster` protocol value with empty collections.
fn empty_aster() -> Value {
    Value::Dict(Dict::from_iter([
        (Str::from("protocol"), Value::Int(1)),
        (Str::from("collections"), Value::Dict(Dict::new())),
    ]))
}

/// Execute the full Aster build lifecycle.
///
/// Returns `(output_paths, per_page_errors)` on success. The caller is
/// responsible for printing errors and deciding whether they are fatal.
pub fn build(project: &ProjectRoot, config: Dict) -> Result<(Vec<PathBuf>, Vec<anyhow::Error>)> {
    let builder = compile::CompileContext::new(project);

    // --- Phase 1: content collections ---
    let aster_value = if project.content_dir().is_dir() {
        let lib = LazyHash::new(world::build_library(config.clone()));
        crate::content::load_collections(project, &builder, &lib)
            .map_err(|err| anyhow::anyhow!("failed to load content collections: {err}"))?
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

    // --- Probe phase ---
    struct RenderJob {
        template: PathBuf,
        library: LazyHash<Library>,
    }

    let mut render_queue: indexmap::IndexMap<PathBuf, RenderJob> = indexmap::IndexMap::new();

    let mut enqueue = |template: PathBuf, output: PathBuf, library: LazyHash<Library>| {
        if render_queue
            .insert(output.clone(), RenderJob { template, library })
            .is_some()
        {
            diag::emit_warning(&format!(
                "duplicate output path `{}` — skipping",
                output.display()
            ));
        }
    };

    for entry in project
        .walk_src()
        .filter(|p| p.extension().is_some_and(|ext| ext == "typ"))
    {
        let relative = entry
            .strip_prefix(project.src_dir())
            .expect("entry under src/");
        let tpl = route::parse_template(relative).expect("invalid route template");

        if route::parse_params(relative).is_empty() {
            let output = project
                .page_output_path(&entry)
                .expect("file must be under src/");
            enqueue(entry, output, page_library.clone());
        } else {
            let routes = route::extract(
                &builder
                    .content(&entry, project, &page_library)
                    .with_context(|| {
                        format!("failed to probe {}: compilation failed", entry.display())
                    })?,
            );
            if routes.is_empty() {
                diag::emit_warning(&format!(
                    "{} has `[slug]` pattern but no `<route>` metadata",
                    entry.display()
                ));
            }

            for params in routes {
                let output = route::output_path(project, &tpl, &params);
                let mut inputs = base_inputs.clone();
                for (name, value) in &params {
                    inputs.push((
                        Str::from(name.as_str()),
                        Value::Str(Str::from(value.as_str())),
                    ));
                }
                let library = LazyHash::new(world::build_library(Dict::from_iter(inputs)));
                enqueue(entry.clone(), output, library);
            }
        }
    }

    // --- Render & serialize ---
    let render_start = std::time::Instant::now();
    let mut outputs: Vec<PathBuf> = Vec::new();
    let mut errors: Vec<anyhow::Error> = Vec::new();

    for (output, job) in &render_queue {
        let mut page = || -> Result<()> {
            let mut doc = builder
                .document(&job.template, project, &job.library)
                .map_err(|_| anyhow::anyhow!("compilation failed"))?;

            transform::process_document(
                &mut doc,
                &transform::ProcessingContext::new(project, output.clone()),
            )?;

            let raw = typst_html::html(&doc, &HtmlOptions::default())
                .map_err(|_| anyhow::anyhow!("HTML encoding failed"))?;

            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            std::fs::write(output, &raw)
                .with_context(|| format!("failed to write {}", output.display()))?;

            outputs.push(output.clone());
            let rel = output.strip_prefix(project.output_dir()).unwrap_or(&output);
            diag::emit_page(&rel.to_string_lossy());
            Ok(())
        };

        if let Err(e) = page() {
            errors.push(e);
        }
    }

    diag::emit_summary(outputs.len(), &render_start);
    Ok((outputs, errors))
}
