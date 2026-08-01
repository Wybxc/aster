use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use typst::Library;
use typst::utils::LazyHash;

use crate::build::output::{AssetPath, OutputPath, OutputPublication};
use crate::build::transform;
use crate::build::world::TypstSession;
use crate::engine::{content, route};
use crate::foundation::config::AsterConfig;

/// The complete build outcome. No build stage decides terminal formatting or
/// exit status; the CLI renders this value after the pipeline finishes.
pub struct BuildOutcome {
    pub outputs: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub elapsed: Duration,
}

impl BuildOutcome {
    pub fn report(&self) {
        for warning in &self.warnings {
            crate::cli::diag::emit_warning(warning);
        }
        crate::cli::diag::emit_summary(self.outputs.len(), self.elapsed);
    }
}

pub struct BuildDriver {
    session: TypstSession,
    started: bool,
}

impl BuildDriver {
    pub fn new(project: crate::foundation::project::ProjectRoot) -> Self {
        Self {
            session: TypstSession::new(project),
            started: false,
        }
    }

    pub fn build(&mut self, config: AsterConfig) -> Result<BuildOutcome> {
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

    pub fn dependencies(&mut self) -> Vec<PathBuf> {
        self.session.dependencies()
    }
}

fn build_once(session: &TypstSession, config: AsterConfig) -> Result<BuildOutcome> {
    let started = Instant::now();
    let project = session.project().clone();
    let mut warnings = Vec::new();
    let mut publication = OutputPublication::new(&project);

    let highlight_css = match transform::compute_highlight_css(
        &config.highlight,
        &project,
        session.project_files(),
    ) {
        Ok(Some(css)) => Some(publication.add_asset("hl", "css", css.into_bytes())?),
        Ok(None) => None,
        Err(error) => {
            warnings.push(format!("failed to resolve highlight CSS: {error:#}"));
            None
        }
    };

    let protocol = content::load(session).context("failed to load content collections")?;
    let base_inputs = content::install(config.dict, protocol)?;
    let base_library = session.library(base_inputs.clone());
    let plan = route::RoutePlan::build(session, &base_inputs, &base_library)?;
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
    template: &std::path::Path,
    output: &OutputPath,
    library: &LazyHash<Library>,
    highlight_css: Option<&AssetPath>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let compiled = session.compile_page(template, library)?;
    warnings.extend(compiled.warnings);

    let mut document = compiled.document;
    let mut page = publication.page(template, output)?;
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
