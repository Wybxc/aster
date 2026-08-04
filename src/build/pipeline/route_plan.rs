use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::eco_format;
use typst::foundations::Dict;
use typst::syntax::VirtualPath;
use typst::utils::LazyHash;

use crate::build::BuildWarning;
use crate::build::world::TypstSession;
use crate::engine::route::{self, ParamSet, RoutePath};
use crate::engine::{content, endpoint};
use crate::foundation::ProjectLayout;

pub(super) struct PlannedRoute {
    pub kind: PlannedRouteKind,
    pub template: VirtualPath,
    pub output: RoutePath,
    pub params: ParamSet,
}

#[derive(Clone, Copy)]
pub(super) enum PlannedRouteKind {
    Page,
    Endpoint,
}

/// Discover, classify, and validate every route template before rendering.
pub(super) fn plan_routes(
    session: &TypstSession,
    layout: &ProjectLayout,
    base_inputs: &Dict,
    base_library: &LazyHash<Library>,
) -> Result<(Vec<PlannedRoute>, Vec<BuildWarning>)> {
    let templates = session.route_templates(layout)?;
    let mut routes = Vec::new();
    let mut warnings = Vec::new();

    for template in templates {
        let relative = Path::new(template.get_without_slash())
            .strip_prefix(Path::new(layout.pages().get_without_slash()))
            .context("route template is outside configured pages directory")?;
        let pattern = route::parse_template(relative)
            .with_context(|| format!("invalid route template {}", relative.display()))?;
        let (evaluated, evaluated_warnings) = session
            .evaluate(&template, base_library)
            .with_context(|| format!("failed to probe {}", relative.display()))?;
        let is_endpoint = endpoint::is_declared(&evaluated)
            .with_context(|| format!("invalid endpoint declaration in {}", relative.display()))?;
        let kind = if is_endpoint {
            PlannedRouteKind::Endpoint
        } else {
            PlannedRouteKind::Page
        };
        let param_sets = if pattern.is_dynamic() {
            warnings.extend(evaluated_warnings);
            let params = route::extract(&evaluated)
                .with_context(|| format!("invalid route metadata in {}", relative.display()))?;
            if params.is_empty() {
                warnings.push(BuildWarning::new(eco_format!(
                    "{} has a dynamic route pattern but no <route> metadata",
                    relative.display()
                )));
            }
            params.into_iter().collect::<Vec<_>>()
        } else {
            vec![ParamSet::new()]
        };

        for params in param_sets {
            if !params.is_empty() {
                content::with_route_params(base_inputs, &params)?;
            }
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
