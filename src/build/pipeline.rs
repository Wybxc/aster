use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::build::output::OutputPublication;
use crate::build::transform::DocumentTransform;
use crate::foundation::ProjectLayout;
use crate::foundation::config::AsterConfig;

mod route_plan;

use self::route_plan::{plan_generators, plan_pages};
use super::{BuildSession, BuildWarning, postprocess, world};

/// The complete build outcome. No build stage decides terminal formatting or
/// exit status; the CLI renders this value after the pipeline finishes.
pub struct BuildOutcome {
    /// Root directory containing the complete published site.
    pub output_dir: PathBuf,
    /// Published page paths, in deterministic route order.
    pub pages: Vec<PathBuf>,
    /// Published generator output paths, in deterministic route order.
    pub generated: Vec<PathBuf>,
    /// Non-fatal diagnostics collected during the build.
    pub warnings: Vec<BuildWarning>,
    /// Total build and publication time.
    pub elapsed: Duration,
}

impl BuildSession {
    /// Build and publish the complete project output tree.
    pub fn build(&mut self) -> Result<BuildOutcome> {
        let started = Instant::now();
        self.reset();
        let outcome = (|| {
            let stage = tracing::info_span!("configure", message = "configured project").entered();
            let config_file = self.project().config_file();
            let config_path = self.project().config_path();
            let content = self
                .project_files()
                .read(&config_path)
                .context("failed to read aster.toml")?;
            let config = AsterConfig::parse(content.as_slice(), &config_file)
                .context("failed to parse aster.toml")?;
            let layout = ProjectLayout::new(&config).context("invalid project layout")?;
            let output_dir = self.project().realize(layout.output());
            tracing::debug!(
                project = %self.project().root().display(),
                output = %output_dir.display(),
                "configured project"
            );
            for path in layout.watch_paths() {
                self.files
                    .watch(path)
                    .context("failed to inspect configured watch paths")?;
            }
            drop(stage);

            let stage = tracing::info_span!("fonts", message = "loaded fonts").entered();
            world::configure_fonts(self, &config.typst.fonts, &layout)?;
            drop(stage);

            let session = &*self;
            let stage = tracing::info_span!("prepare", message = "prepared build").entered();
            let project = session.project().clone();
            let mut warnings = Vec::new();
            let mut publication = OutputPublication::new(&project, &layout)?;
            publication
                .add_public_tree(session.project_files(), layout.public())
                .context("failed to collect public files")?;

            let (mut transform, transform_warning) = DocumentTransform::new(
                session.project_files(),
                project.root(),
                &config.assets,
                &config.css,
                &config.highlight,
            )?;
            warnings.extend(transform_warning);

            let runtime = super::content::load(session.project_files(), layout.content())
                .context("failed to load content collections")?;
            drop(stage);

            let stage = tracing::info_span!("plan", message = "planned routes").entered();
            let (pages, page_warnings) = plan_pages(session, &layout, &runtime)?;
            warnings.extend(page_warnings);
            let page_paths = pages
                .iter()
                .map(|job| job.output.page_url_path())
                .collect::<Vec<_>>();
            tracing::debug!(
                pages = pages.len(),
                "planned {} page{}",
                pages.len(),
                if pages.len() == 1 { "" } else { "s" }
            );
            let runtime = runtime.with_page_routes(&page_paths);
            drop(stage);

            let stage = tracing::info_span!("render", message = "rendered pages").entered();
            let mut site_pages = Vec::with_capacity(pages.len());
            for job in &pages {
                let path = job.output.page_url_path();
                let route = tracing::info_span!("page", route = %path, message = "rendered page");
                let _route = route.enter();
                let route_runtime = runtime.for_route(path, &job.params);
                let stage = tracing::debug_span!(
                    "compile",
                    source = %job.template.get_with_slash(),
                    message = "compiled"
                )
                .entered();
                let (document, compiled_warnings) =
                    world::compile_document(session, &job.template, &route_runtime)?;
                warnings.extend(compiled_warnings);
                drop(stage);
                let page = transform
                    .render(
                        document,
                        publication.page(&job.template, &job.output),
                        config.output.pretty,
                    )
                    .with_context(|| format!("failed to build {}", job.output))?;
                site_pages.push(page);
            }
            drop(stage);

            let stage = tracing::info_span!("generate", message = "ran generators").entered();
            let runtime = runtime.with_site(&site_pages);
            let (generators, generator_warnings) =
                plan_generators(session, &layout, &runtime, &pages)?;
            warnings.extend(generator_warnings);
            tracing::debug!(
                generators = generators.len(),
                "planned {} generator{}",
                generators.len(),
                if generators.len() == 1 { "" } else { "s" }
            );
            for job in &generators {
                let path = job.output.url_path();
                let route = tracing::info_span!(
                    "generator",
                    route = %path,
                    message = "ran generator"
                );
                let _route = route.enter();
                let route_runtime = runtime.for_route(path, &job.params);
                let stage = tracing::debug_span!(
                    "compile",
                    source = %job.template.get_with_slash(),
                    message = "compiled"
                )
                .entered();
                let (content, compiled_warnings) =
                    world::evaluate_generator(session, &job.template, &route_runtime)
                        .context("invalid generator output")?;
                warnings.extend(compiled_warnings);
                drop(stage);
                publication
                    .add_generator_output(job.output.clone(), content)
                    .with_context(|| format!("failed to generate {}", job.output))?;
            }
            drop(stage);

            let stage = tracing::info_span!("publish", message = "published output").entered();
            let mut staged = publication.stage()?;
            postprocess::run(&config.postprocess, project.root(), &mut staged)?;
            let published = staged.publish()?;
            drop(stage);
            Ok(BuildOutcome {
                output_dir: published.output_dir,
                pages: published.pages,
                generated: published.generated,
                warnings,
                elapsed: started.elapsed(),
            })
        })();
        comemo::evict(10);
        outcome
    }
}
