use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use typst::Library;
use typst::utils::LazyHash;

use crate::compile::TypstSession;
use crate::config::AsterConfig;
use crate::output::{AssetPath, OutputPath, OutputPublication};
use crate::project::ProjectRoot;
use crate::{content, route, transform};

/// The complete build outcome. No build stage decides terminal formatting or
/// exit status; the CLI renders this value after the pipeline finishes.
pub struct BuildOutcome {
    pub outputs: Vec<PathBuf>,
    pub warnings: Vec<String>,
    pub elapsed: Duration,
}

pub fn build(project: ProjectRoot, config: AsterConfig) -> Result<BuildOutcome> {
    let started = Instant::now();
    let session = TypstSession::new(project.clone());
    let mut warnings = Vec::new();
    let mut publication = OutputPublication::new(&project);

    let highlight_css =
        match transform::highlight::compute_highlight_css(&config.highlight, &project) {
            Ok(Some(css)) => Some(publication.add_asset("hl", "css", css.into_bytes())?),
            Ok(None) => None,
            Err(error) => {
                warnings.push(format!("failed to resolve highlight CSS: {error:#}"));
                None
            }
        };

    let content_library = session.library(config.dict.clone());
    let loaded =
        content::load(&session, &content_library).context("failed to load content collections")?;
    warnings.extend(loaded.warnings);
    let base_inputs = content::install(config.dict, loaded.protocol)?;
    let base_library = session.library(base_inputs.clone());
    let mut probe_warnings = Vec::new();
    let plan = route::RoutePlan::build(&project, |template| {
        let evaluated = session.evaluate(template, &base_library)?;
        probe_warnings.extend(evaluated.warnings);
        let routes = route::extract(&evaluated.content)
            .with_context(|| format!("invalid route metadata in {}", template.display()))?;
        for params in &routes {
            content::with_route_params(&base_inputs, params)?;
        }
        Ok(routes)
    })?;
    let (jobs, route_warnings) = plan.into_parts();
    warnings.extend(probe_warnings);
    warnings.extend(route_warnings);

    for job in jobs {
        let library = if job.params.is_empty() {
            base_library.clone()
        } else {
            session.library(content::with_route_params(&base_inputs, &job.params)?)
        };
        render_page(
            &session,
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
    transform::process_document(&mut document, &mut page, highlight_css)?;
    let html = typst_html::html(&document, &typst_html::HtmlOptions::default())
        .map_err(|error| anyhow::anyhow!("HTML encoding failed: {error:?}"))?;
    page.add_html(html)
}
