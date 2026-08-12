use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::{eco_format, eco_vec};
use typst::syntax::VirtualPath;
use typst::utils::LazyHash;

use crate::build::{BuildSession, BuildWarning, files, world};
use crate::engine::endpoint;
use crate::engine::route::{self, ParamSet, RoutePath};
use crate::foundation::ProjectLayout;

pub struct PlannedRoute {
    pub kind: PlannedRouteKind,
    pub template: VirtualPath,
    pub output: RoutePath,
    pub params: ParamSet,
}

#[derive(Clone, Copy)]
pub enum PlannedRouteKind {
    Page,
    Endpoint,
}

/// Discover, classify, and validate every route template before rendering.
pub fn plan_routes(
    session: &BuildSession,
    layout: &ProjectLayout,
    base_library: &LazyHash<Library>,
) -> Result<(Vec<PlannedRoute>, Vec<BuildWarning>)> {
    let templates = files::list_typst_files(session.project_files(), layout.pages(), true)?;
    let mut routes = Vec::new();
    let mut warnings = Vec::new();

    for template in templates {
        let relative = Path::new(template.get_without_slash())
            .strip_prefix(Path::new(layout.pages().get_without_slash()))
            .context("route template is outside configured pages directory")?;
        let probe = tracing::debug_span!(
            "probe",
            template = %relative.display(),
            message = %format_args!("probed template {}", relative.display())
        )
        .entered();
        let pattern = route::parse_template(relative)
            .with_context(|| format!("invalid route template {}", relative.display()))?;
        let (document, compiled_warnings) =
            world::compile_document(session, &template, base_library)
                .with_context(|| format!("failed to probe {}", relative.display()))?;
        let introspector = document.introspector().as_ref();
        let is_endpoint = endpoint::is_declared(introspector)
            .with_context(|| format!("invalid endpoint declaration in {}", relative.display()))?;
        let kind = if is_endpoint {
            PlannedRouteKind::Endpoint
        } else {
            PlannedRouteKind::Page
        };
        let param_sets = if pattern.is_dynamic() {
            warnings.extend(compiled_warnings);
            let params = route::extract_params(introspector)
                .with_context(|| format!("invalid route metadata in {}", relative.display()))?;
            if params.is_empty() {
                warnings.push(BuildWarning::new(eco_format!(
                    "{} has a dynamic route pattern but no <aster-route> metadata",
                    relative.display()
                )));
            }
            params
        } else {
            eco_vec![ParamSet::new()]
        };

        for params in param_sets {
            let output = match kind {
                PlannedRouteKind::Page => pattern.generate(&params)?,
                PlannedRouteKind::Endpoint => pattern.generate_endpoint(&params)?,
            };
            routes.push(PlannedRoute {
                kind,
                template: template.clone(),
                output,
                params,
            });
        }
        drop(probe);
    }

    routes.sort_by(|left, right| {
        left.output.cmp(&right.output).then_with(|| {
            left.template
                .get_with_slash()
                .cmp(right.template.get_with_slash())
        })
    });
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
    Ok((routes, warnings))
}
