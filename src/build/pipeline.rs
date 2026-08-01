use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::EcoString;
use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};
use typst::utils::LazyHash;

use crate::build::output::{AssetPath, OutputPublication};
use crate::build::transform;
use crate::build::world::TypstSession;
use crate::engine::content::{self, ContentEntry};
use crate::engine::route::RoutePath;
use crate::foundation::Project;
use crate::foundation::config::AsterConfig;

mod route_plan;

use self::route_plan::RoutePlan;

/// The complete build outcome. No build stage decides terminal formatting or
/// exit status; the CLI renders this value after the pipeline finishes.
pub struct BuildOutcome {
    /// Published page paths, in deterministic route order.
    pub outputs: Vec<PathBuf>,
    /// Non-fatal diagnostics collected during the build.
    pub warnings: Vec<String>,
    /// Total build and publication time.
    pub elapsed: Duration,
}

/// A reusable build session bound to one Aster project.
pub struct BuildSession {
    session: TypstSession,
    started: bool,
}

impl BuildSession {
    /// Create a reusable session for `project`.
    pub fn new(project: Project) -> Self {
        Self {
            session: TypstSession::new(project),
            started: false,
        }
    }

    /// Build and publish the complete project output tree.
    pub fn build(&mut self) -> Result<BuildOutcome> {
        let config = AsterConfig::load(&self.session.project().config_file())
            .context("failed to parse aster.toml")?;
        let rebuilding = std::mem::replace(&mut self.started, true);
        if rebuilding {
            self.session.reset();
        }

        let outcome = build_once(&self.session, config);
        if rebuilding {
            comemo::evict(10);
        }
        outcome
    }

    /// Return the dependencies observed by the latest build attempt.
    pub fn dependencies(&mut self) -> Vec<PathBuf> {
        self.session.dependencies()
    }

    /// Return the project bound to this session.
    pub fn project(&self) -> &Project {
        self.session.project()
    }
}

/// Build a project once using a fresh session.
pub fn build(project: Project) -> Result<BuildOutcome> {
    BuildSession::new(project).build()
}

fn build_once(session: &TypstSession, config: AsterConfig) -> Result<BuildOutcome> {
    let started = Instant::now();
    let project = session.project().clone();
    let mut warnings = Vec::new();
    let mut publication = OutputPublication::new(&project);

    let highlight_css =
        match transform::compute_highlight_css(&config.highlight, session.project_files()) {
            Ok(Some(css)) => Some(publication.add_highlight_stylesheet(css.into_bytes())?),
            Ok(None) => None,
            Err(error) => {
                warnings.push(format!("failed to resolve highlight CSS: {error:#}"));
                None
            }
        };

    let protocol = load_content(session).context("failed to load content collections")?;
    let base_inputs = content::install(config.dict, protocol)?;
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
            highlight_css.as_ref(),
            &mut warnings,
        )
        .with_context(|| format!("failed to build {}", job.output.as_path().display()))?;
    }

    let published = publication.publish()?;
    Ok(BuildOutcome {
        outputs: published.pages,
        warnings,
        elapsed: started.elapsed(),
    })
}

fn render_page(
    session: &TypstSession,
    publication: &mut OutputPublication,
    template: &VirtualPath,
    output: &RoutePath,
    library: &LazyHash<Library>,
    highlight_css: Option<&AssetPath>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let compiled = session.compile_page(template, library)?;
    warnings.extend(compiled.warnings);

    let mut document = compiled.document;
    let mut page = publication.page(template, output);
    transform::process_document(
        &mut document,
        &mut page,
        session.project_files(),
        highlight_css,
    )?;
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
