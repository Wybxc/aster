use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::World;
use typst::foundations::{Dict, Str, Value};
use typst::syntax::Source;
use typst::utils::LazyHash;
use typst_html::HtmlOptions;

use crate::config::AsterConfig;
use crate::project::ProjectRoot;
use crate::utils::Asset;
use crate::{compile, diag, route, transform};

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
pub fn build(
    project: &ProjectRoot,
    aster_config: AsterConfig,
) -> Result<(Vec<PathBuf>, Vec<anyhow::Error>)> {
    let builder = compile::CompileContext::new(project);

    // Shared asset collector — registers all generated files, flushed at end.
    let mut all_assets = crate::utils::AssetCollector::new();

    // Pre-resolve highlight theme colours (non-fatal on failure).
    // Computation and I/O are separate: compute returns content, pipeline
    // registers it in the collector for batch writing.
    let hl_css_path = transform::highlight::compute_highlight_css(&aster_config.highlight, project)
        .unwrap_or_else(|e| {
            diag::emit_warning(&format!("failed to resolve highlight CSS: {e:#}"));
            None
        })
        .map(|(css_content, filename)| {
            all_assets.add(&project.output_dir(), "hl", "css", css_content.into_bytes());
            filename
        });

    // --- Phase 1: content collections ---
    let aster_value = if project.content_dir().is_dir() {
        let lib = builder.page_library(aster_config.dict.clone());
        crate::content::load_collections(project, &builder, &lib)
            .map_err(|err| anyhow::anyhow!("failed to load content collections: {err}"))?
    } else {
        empty_aster()
    };

    // Base Typst inputs — reused for both static and dynamic pages.
    let mut base_inputs: Vec<(Str, Value)> = aster_config.dict.into_iter().collect();
    base_inputs.push((Str::from("_aster"), aster_value));

    let page_library = builder.page_library(Dict::from_iter(base_inputs.clone()));

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
                for (name, value) in params {
                    inputs.push((Str::from(name), Value::Str(Str::from(value))));
                }
                let library = builder.page_library(Dict::from_iter(inputs));
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
            // Pre‑load the template source for cache‑key computation.
            let world = builder.world_with_source(&job.template, project, &job.library);
            let source = world
                .source(world.main)
                .map_err(|e| anyhow::anyhow!("failed to load source: {e}"))?;

            let suffix = output
                .strip_prefix(project.output_dir())
                .unwrap_or(output)
                .to_string_lossy()
                .into_owned();
            let hl_text = hl_css_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            let (raw, page_assets) = cached_page(
                &builder,
                source,
                &job.library,
                project,
                output,
                &hl_css_path,
            )?;

            for asset in page_assets {
                all_assets.add_path(asset.path, asset.content);
            }

            crate::utils::write_file(output, raw.as_bytes())?;

            outputs.push(output.clone());
            let out_rel = output.strip_prefix(project.output_dir()).unwrap_or(output);
            diag::emit_page(&out_rel.to_string_lossy());
            Ok(())
        };

        if let Err(e) = page() {
            errors.push(e);
        }
    }

    // Flush all generated assets (duplicate content → single file).
    if let Err(e) = all_assets.flush() {
        diag::emit_warning(&format!("failed to write assets: {e:#}"));
    }

    diag::emit_summary(outputs.len(), &render_start);
    Ok((outputs, errors))
}

/// Per‑build page cache: `(source_hash, lib_hash, suffix) → HTML + assets`.
static PAGE_CACHE: LazyLock<Mutex<HashMap<u128, (String, Vec<Asset>)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Compute a single page, using an in‑memory cache keyed by the content
/// hash of the source and library.
fn cached_page(
    builder: &compile::CompileContext,
    source: Source,
    library: &LazyHash<Library>,
    project: &ProjectRoot,
    page_path: &PathBuf,
    hl_css_path: &Option<PathBuf>,
) -> Result<(String, Vec<Asset>)> {
    // Build a composite key from everything that affects the output.
    let source_hash = seahash::hash(source.text().as_bytes()) as u128;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    library.hash(&mut h);
    let lib_hash = h.finish() as u128;
    let suffix = page_path
        .strip_prefix(project.output_dir())
        .unwrap_or(page_path)
        .to_string_lossy();
    let key = source_hash ^ lib_hash.rotate_left(64) ^ seahash::hash(suffix.as_bytes()) as u128;

    if let Some(cached) = PAGE_CACHE.lock().unwrap().get(&key) {
        return Ok(cached.clone());
    }

    let result = render_page(builder, source, library, project, page_path, hl_css_path)?;
    PAGE_CACHE.lock().unwrap().insert(key, result.clone());
    Ok(result)
}

/// The actual (uncached) page rendering.
fn render_page(
    builder: &compile::CompileContext,
    source: Source,
    library: &LazyHash<Library>,
    project: &ProjectRoot,
    page_path: &PathBuf,
    hl_css_path: &Option<PathBuf>,
) -> Result<(String, Vec<Asset>)> {
    let mut doc = builder
        .document_with_source(source, library)
        .map_err(|_| anyhow::anyhow!("compilation failed"))?;

    let pctx = transform::ProcessingContext::new(
        page_path.clone(),
        hl_css_path.clone(),
        project.src_dir(),
        project.output_dir(),
    );
    let page_assets = transform::process_document(&mut doc, &pctx)?;

    let raw = typst_html::html(&doc, &HtmlOptions::default())
        .map_err(|_| anyhow::anyhow!("HTML encoding failed"))?;

    Ok((raw, page_assets))
}
