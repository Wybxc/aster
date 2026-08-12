use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::ecow::{eco_format, eco_vec};
use typst::syntax::VirtualPath;

use crate::build::{BuildSession, BuildWarning, files, world};
use crate::engine::content::Runtime;
use crate::engine::route::{self, ParamSet, RoutePath};
use crate::foundation::ProjectLayout;

pub struct PlannedRoute {
    pub template: VirtualPath,
    pub output: RoutePath,
    pub params: ParamSet,
}

#[derive(Clone, Copy)]
enum TemplateKind {
    Page,
    Generator,
}

impl TemplateKind {
    fn root(self, layout: &ProjectLayout) -> &VirtualPath {
        match self {
            Self::Page => layout.pages(),
            Self::Generator => layout.generate(),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Generator => "generator",
        }
    }

    fn required(self) -> bool {
        matches!(self, Self::Page)
    }

    fn output(self, pattern: &route::RouteTemplate, params: &ParamSet) -> Result<RoutePath> {
        match self {
            Self::Page => pattern.generate(params),
            Self::Generator => pattern.generate_file(params),
        }
    }
}

/// Plan HTML pages without evaluating static templates.
pub fn plan_pages(
    session: &BuildSession,
    layout: &ProjectLayout,
    runtime: &Runtime,
) -> Result<(Vec<PlannedRoute>, Vec<BuildWarning>)> {
    plan_templates(session, layout, TemplateKind::Page, runtime)
}

/// Plan exact-path generated files after the rendered site is available.
pub fn plan_generators(
    session: &BuildSession,
    layout: &ProjectLayout,
    runtime: &Runtime,
    pages: &[PlannedRoute],
) -> Result<(Vec<PlannedRoute>, Vec<BuildWarning>)> {
    let (generators, warnings) = plan_templates(session, layout, TemplateKind::Generator, runtime)?;
    validate_outputs(pages, &generators)?;
    Ok((generators, warnings))
}

/// Validate collisions across independently planned page and generator routes.
fn validate_outputs(pages: &[PlannedRoute], generators: &[PlannedRoute]) -> Result<()> {
    let mut routes = pages.iter().chain(generators).collect::<Vec<_>>();
    routes.sort_by(|left, right| left.output.cmp(&right.output));
    for (index, left) in routes.iter().enumerate() {
        for right in &routes[index + 1..] {
            if left.output.conflicts_with(&right.output) {
                bail!(
                    "templates {} and {} generate conflicting outputs {} and {}",
                    left.template.get_with_slash(),
                    right.template.get_with_slash(),
                    left.output,
                    right.output
                );
            }
        }
    }
    Ok(())
}

fn plan_templates(
    session: &BuildSession,
    layout: &ProjectLayout,
    kind: TemplateKind,
    runtime: &Runtime,
) -> Result<(Vec<PlannedRoute>, Vec<BuildWarning>)> {
    let root = kind.root(layout);
    let templates = files::list_typst_files(session.project_files(), root, kind.required())?;
    let mut routes = Vec::new();
    let mut warnings = Vec::new();

    for template in templates {
        let relative = Path::new(template.get_without_slash())
            .strip_prefix(Path::new(root.get_without_slash()))
            .with_context(|| {
                format!(
                    "{} template is outside its configured directory",
                    kind.name()
                )
            })?;
        let pattern = route::parse_template(relative)
            .with_context(|| format!("invalid {} template {}", kind.name(), relative.display()))?;
        let param_sets = if pattern.is_dynamic() {
            let probe = tracing::debug_span!(
                "probe",
                template = %relative.display(),
                message = "probed dynamic template"
            )
            .entered();
            let (document, compiled_warnings) =
                world::compile_document(session, &template, runtime)
                    .with_context(|| format!("failed to probe {}", relative.display()))?;
            warnings.extend(compiled_warnings);
            let params = route::extract_params(document.introspector().as_ref())
                .with_context(|| format!("invalid route metadata in {}", relative.display()))?;
            if params.is_empty() {
                warnings.push(BuildWarning::new(eco_format!(
                    "{} has a dynamic route pattern but no <aster-route> metadata",
                    relative.display()
                )));
            }
            drop(probe);
            params
        } else {
            eco_vec![ParamSet::new()]
        };

        for params in param_sets {
            let output = kind.output(&pattern, &params)?;
            routes.push(PlannedRoute {
                template: template.clone(),
                output,
                params,
            });
        }
    }

    routes.sort_by(|left, right| {
        left.output.cmp(&right.output).then_with(|| {
            left.template
                .get_with_slash()
                .cmp(right.template.get_with_slash())
        })
    });
    validate_outputs(&routes, &[])?;
    Ok((routes, warnings))
}
