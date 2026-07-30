use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::World;
use typst::comemo::{Track, Tracked, memoize};
use typst::foundations::{Dict, Str, Value};
use typst::utils::LazyHash;

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
            // Create a CompileWorld with the template source pre‑loaded,
            // then pass it as a tracked World — changes to the source
            // content invalidate comemo's cache automatically.
            let world = builder.world_with_source(&job.template, project, &job.library);

            let suffix = output
                .strip_prefix(project.output_dir())
                .unwrap_or(output)
                .to_string_lossy()
                .into_owned();
            let hl_str = hl_css_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let root_str = project.root().to_string_lossy().into_owned();

            let (raw, page_assets) = compute_page_output(
                &suffix,
                &hl_str,
                &root_str,
                (&world as &dyn World).track(),
                &job.library,
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;

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

// (memoization handled by #[comemo::memoize])

/// Memoized page rendering.
///
/// `world` is a tracked Typst [`World`] — changes to source content,
/// fonts, or files invalidate the cache through comemo's constraint
/// tracking.  `suffix`, `hl_css_path`, and `root_path` are hashed into
/// the cache key.
#[memoize]
fn compute_page_output(
    suffix: &str,
    hl_css_path: &str,
    root_path: &str,
    world: Tracked<dyn World + '_>,
    _library: &LazyHash<Library>,
) -> Result<(String, Vec<Asset>), String> {
    let main = world.main();
    let _source = world
        .source(main)
        .map_err(|e| format!("failed to load source: {e}"))?;

    let warned = typst::compile::<typst_html::HtmlDocument>(&*world);

    let mut doc = match warned.output {
        Ok(d) => d,
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e:?}");
            }
            return Err("compilation failed".into());
        }
    };

    let root = std::path::Path::new(root_path);
    let pctx = crate::transform::ProcessingContext::new(
        std::path::PathBuf::from(suffix),
        if hl_css_path.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(hl_css_path))
        },
        root.join("src"),
        root.join("dist"),
    );
    let page_assets = crate::transform::process_document(&mut doc, &pctx)
        .map_err(|e| format!("transform failed: {e}"))?;

    let raw = typst_html::html(&doc, &typst_html::HtmlOptions::default())
        .map_err(|e| format!("HTML encoding failed: {e:?}"))?;

    Ok((raw, page_assets))
}
