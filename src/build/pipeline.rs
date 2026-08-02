use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::EcoString;
use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};
use typst::utils::LazyHash;

use crate::build::output::OutputPublication;
use crate::build::transform;
use crate::build::world::TypstSession;
use crate::engine::content::{self, ContentEntry};
use crate::engine::route::RoutePath;
use crate::foundation::Project;
use crate::foundation::config::AsterConfig;

mod route_plan;

use self::route_plan::RoutePlan;
use super::BuildWarning;

/// The complete build outcome. No build stage decides terminal formatting or
/// exit status; the CLI renders this value after the pipeline finishes.
pub struct BuildOutcome {
    /// Published page paths, in deterministic route order.
    pub outputs: Vec<PathBuf>,
    /// Non-fatal diagnostics collected during the build.
    pub warnings: Vec<BuildWarning>,
    /// Total build and publication time.
    pub elapsed: Duration,
}

/// A reusable build session bound to one Aster project.
pub struct BuildSession {
    session: TypstSession,
}

impl BuildSession {
    /// Create a reusable session for `project`.
    pub fn new(project: Project) -> Self {
        Self {
            session: TypstSession::new(project),
        }
    }

    /// Build and publish the complete project output tree.
    pub fn build(&mut self) -> Result<BuildOutcome> {
        let config = AsterConfig::load(&self.session.project().config_file())
            .context("failed to parse aster.toml")?;
        self.session.reset();

        let session = &self.session;
        let outcome = (|| {
            let started = Instant::now();
            let project = session.project().clone();
            let mut warnings = Vec::new();
            let mut publication = OutputPublication::new(&project);

            let mut css = transform::CssProcessor::new(session.project_files());
            let mut image = transform::ImageProcessor::new();
            let (mut highlight, highlight_warnings) = transform::HighlightProcessor::new(
                &config.highlight,
                session.project_files(),
                &mut publication,
            )?;
            warnings.extend(highlight_warnings);
            let mut processors: [&mut dyn transform::Processor; 3] =
                [&mut css, &mut image, &mut highlight];

            let protocol = load_content(session).context("failed to load content collections")?;
            let base_inputs = content::with_protocol(config.dict, protocol)?;
            let base_library = session.library(base_inputs.clone());
            let plan = RoutePlan::build(session, &base_inputs, &base_library)?;
            let (jobs, route_warnings) = plan.into_parts();
            warnings.extend(route_warnings);

            for job in jobs {
                let library = if job.params.is_empty() {
                    base_library.clone()
                } else {
                    session.library(content::with_route_params(&base_inputs, &job.params)?)
                };
                render_page(
                    session,
                    &mut publication,
                    &job.template,
                    &job.output,
                    &library,
                    &mut processors,
                    &mut warnings,
                )
                .with_context(|| format!("failed to build {}", job.output))?;
            }

            let published = publication.publish()?;
            Ok(BuildOutcome {
                outputs: published.pages,
                warnings,
                elapsed: started.elapsed(),
            })
        })();
        comemo::evict(10);
        outcome
    }

    /// Iterate over the dependencies observed by the latest build attempt.
    pub fn dependencies(&mut self) -> impl Iterator<Item = PathBuf> + '_ {
        self.session.dependencies()
    }

    /// Return the project bound to this session.
    pub fn project(&self) -> &Project {
        self.session.project()
    }
}

fn render_page(
    session: &TypstSession,
    publication: &mut OutputPublication,
    template: &VirtualPath,
    output: &RoutePath,
    library: &LazyHash<Library>,
    processors: &mut [&mut dyn transform::Processor],
    warnings: &mut Vec<BuildWarning>,
) -> Result<()> {
    let (mut document, compiled_warnings) = session.compile_page(template, library)?;
    warnings.extend(compiled_warnings);

    let mut page = publication.page(template, output);
    transform::process_document(&mut document, &mut page, processors)?;
    let html = typst_html::html(&document, &typst_html::HtmlOptions::default())
        .map_err(|error| anyhow::anyhow!("HTML encoding failed: {error:?}"))?;
    page.add_html(html)
}

fn load_content(session: &TypstSession) -> Result<typst::foundations::Value> {
    let mut entries = Vec::new();

    for path in session.content_files()? {
        let content_relative = Path::new(path.get_without_slash())
            .strip_prefix("content")
            .context("content path is outside /content")?;
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
