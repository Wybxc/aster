use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::EcoString;
use typst::syntax::{RootedPath, VirtualRoot};
use typst::utils::LazyHash;

use crate::build::output::OutputPublication;
use crate::build::transform;
use crate::engine::content::ContentEntry;
use crate::engine::{content, endpoint};
use crate::foundation::ProjectLayout;
use crate::foundation::config::AsterConfig;

mod route_plan;

use self::route_plan::{PlannedRoute, PlannedRouteKind, plan_routes};
use super::{BuildSession, BuildWarning, files, world};

/// The complete build outcome. No build stage decides terminal formatting or
/// exit status; the CLI renders this value after the pipeline finishes.
pub struct BuildOutcome {
    /// Root directory containing the complete published site.
    pub output_dir: PathBuf,
    /// Published page paths, in deterministic route order.
    pub outputs: Vec<PathBuf>,
    /// Published generated endpoint paths, in deterministic route order.
    pub endpoints: Vec<PathBuf>,
    /// Non-fatal diagnostics collected during the build.
    pub warnings: Vec<BuildWarning>,
    /// Total build and publication time.
    pub elapsed: Duration,
}

impl BuildSession {
    /// Build and publish the complete project output tree.
    pub fn build(&mut self) -> Result<BuildOutcome> {
        self.reset();
        let outcome = (|| {
            let config_file = self.project().config_file();
            let config_path = self.project().config_path();
            let content = self
                .project_files()
                .read(&config_path)
                .context("failed to read aster.toml")?;
            let config = AsterConfig::parse(content.as_slice(), &config_file)
                .context("failed to parse aster.toml")?;
            let layout = ProjectLayout::new(&config).context("invalid project layout")?;
            for path in layout.watch_paths() {
                self.files
                    .watch(path)
                    .context("failed to inspect configured watch paths")?;
            }
            world::configure_fonts(self, &config.typst.fonts, &layout)?;

            let session = &*self;
            let started = Instant::now();
            let project = session.project().clone();
            let mut warnings = Vec::new();
            let mut publication = OutputPublication::new(&project, &layout)?;
            add_public_files(session, &layout, &mut publication)
                .context("failed to collect public files")?;

            let mut assets = transform::AssetProcessor::new(
                session.project_files(),
                project.root(),
                &config.assets,
                &config.css,
            )?;
            let (mut highlight, highlight_warning) = transform::HighlightProcessor::new(
                &config.highlight,
                session.project_files(),
                &mut publication,
            )?;
            warnings.extend(highlight_warning);

            let protocol =
                load_content(session, &layout).context("failed to load content collections")?;
            let base_inputs = content::inputs(protocol);
            let base_library = world::library(base_inputs.clone());
            let (routes, route_warnings) =
                plan_routes(session, &layout, &base_inputs, &base_library)?;
            warnings.extend(route_warnings);

            for job in routes {
                let library = if job.params.is_empty() {
                    base_library.clone()
                } else {
                    world::library(content::with_route_params(&base_inputs, &job.params)?)
                };
                match job.kind {
                    PlannedRouteKind::Page => render_page(
                        session,
                        &mut publication,
                        &job,
                        &library,
                        config.output.pretty,
                        (&mut assets, &mut highlight),
                        &mut warnings,
                    ),
                    PlannedRouteKind::Endpoint => {
                        render_endpoint(session, &mut publication, &job, &library, &mut warnings)
                    }
                }
                .with_context(|| format!("failed to build {}", job.output))?;
            }

            let published = publication.publish()?;
            Ok(BuildOutcome {
                output_dir: published.output_dir,
                outputs: published.pages,
                endpoints: published.endpoints,
                warnings,
                elapsed: started.elapsed(),
            })
        })();
        comemo::evict(10);
        outcome
    }
}

fn add_public_files(
    session: &BuildSession,
    layout: &ProjectLayout,
    publication: &mut OutputPublication,
) -> Result<()> {
    let files = session.project_files();
    let public_root = Path::new(layout.public().get_without_slash());

    for path in files.list(layout.public(), false)? {
        let relative = Path::new(path.get_without_slash())
            .strip_prefix(public_root)
            .context("public file is outside configured public directory")?;
        let content = files
            .read(&path)
            .with_context(|| format!("failed to read public file {}", path.get_with_slash()))?;
        publication.add_public_file(relative, content)?;
    }
    Ok(())
}

fn render_page(
    session: &BuildSession,
    publication: &mut OutputPublication,
    job: &PlannedRoute,
    library: &LazyHash<Library>,
    pretty: bool,
    processors: (
        &mut transform::AssetProcessor<'_>,
        &mut transform::HighlightProcessor,
    ),
    warnings: &mut Vec<BuildWarning>,
) -> Result<()> {
    let (mut document, compiled_warnings) =
        world::compile_document(session, &job.template, library)?;
    warnings.extend(compiled_warnings);
    let resources = transform::ComponentResources::collect(&document)?;

    let mut page = publication.page(&job.template, &job.output);
    let (assets, highlight) = processors;
    {
        let mut processors: [&mut dyn transform::Processor; 2] = [assets, highlight];
        transform::process_document(&mut document, &mut page, &mut processors)?;
    }
    resources.apply(&mut document, &mut page, assets)?;
    let html = typst_html::html(&document, &typst_html::HtmlOptions { pretty })
        .map_err(|error| anyhow::anyhow!("HTML encoding failed: {error:?}"))?;
    page.add_html(html)
}

fn render_endpoint(
    session: &BuildSession,
    publication: &mut OutputPublication,
    job: &PlannedRoute,
    library: &LazyHash<Library>,
    warnings: &mut Vec<BuildWarning>,
) -> Result<()> {
    let (document, compiled_warnings) = world::compile_document(session, &job.template, library)?;
    warnings.extend(compiled_warnings);
    let content = endpoint::extract(document.introspector().as_ref())
        .context("invalid endpoint declaration")?
        .context("endpoint route did not produce <aster-endpoint>")?;
    publication.add_generated_file(job.output.clone(), content)
}

fn load_content(
    session: &BuildSession,
    layout: &ProjectLayout,
) -> Result<typst::foundations::Value> {
    let mut entries = Vec::new();

    for path in files::list_typst_files(session.project_files(), layout.content(), false)? {
        let content_relative = Path::new(path.get_without_slash())
            .strip_prefix(Path::new(layout.content().get_without_slash()))
            .context("content path is outside configured content directory")?;
        if content_relative.components().count() < 2 {
            bail!(
                "entry {} is not inside a collection; expected content/<collection>/.../<id>.typ",
                path.get_with_slash()
            );
        }

        let mut components = content_relative.components();
        let collection = components
            .next()
            .map(|component| EcoString::from(component.as_os_str().to_string_lossy().as_ref()))
            .context("entry not inside a collection directory")?;
        let id = {
            let mut path = PathBuf::new();
            for component in components {
                path.push(component);
            }
            path.set_extension("");
            EcoString::from(path.to_string_lossy().replace('\\', "/"))
        };

        entries.push(ContentEntry {
            collection,
            id,
            source: RootedPath::new(VirtualRoot::Project, path),
        });
    }

    Ok(content::protocol(entries))
}
