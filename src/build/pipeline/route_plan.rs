use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::eco_format;
use typst::foundations::Dict;
use typst::syntax::VirtualPath;
use typst::utils::LazyHash;

use crate::build::BuildWarning;
use crate::build::world::TypstSession;
use crate::engine::content;
use crate::engine::route::{self, ParamSet, RoutePath};

/// A deterministic, collision-free page plan.
pub(super) struct RoutePlan {
    jobs: Vec<PlannedRoute>,
    warnings: Vec<BuildWarning>,
}

pub(super) struct PlannedRoute {
    pub template: VirtualPath,
    pub output: RoutePath,
    pub params: ParamSet,
}

impl RoutePlan {
    /// Discover, parse, and probe every template exactly once.
    pub fn build(
        session: &TypstSession,
        base_inputs: &Dict,
        base_library: &LazyHash<Library>,
    ) -> Result<Self> {
        let templates = session.source_files()?;
        let mut jobs = Vec::new();
        let mut warnings = Vec::new();

        for template in templates {
            let relative = Path::new(template.get_without_slash())
                .strip_prefix("src")
                .context("source template is outside /src")?;
            let pattern = route::parse_template(relative)
                .with_context(|| format!("invalid route template {}", relative.display()))?;
            if pattern.is_dynamic() {
                let (evaluated, evaluated_warnings) = session
                    .evaluate(&template, base_library)
                    .with_context(|| format!("failed to probe {}", relative.display()))?;
                warnings.extend(evaluated_warnings);
                let routes = route::extract(&evaluated)
                    .with_context(|| format!("invalid route metadata in {}", relative.display()))?;
                if routes.is_empty() {
                    warnings.push(BuildWarning::new(eco_format!(
                        "{} has a dynamic route pattern but no <route> metadata",
                        relative.display()
                    )));
                }
                for params in routes {
                    content::with_route_params(base_inputs, &params)?;
                    jobs.push(PlannedRoute {
                        template: template.clone(),
                        output: pattern.generate(&params)?,
                        params,
                    });
                }
            } else {
                jobs.push(PlannedRoute {
                    output: RoutePath::from_template(relative)?,
                    template,
                    params: ParamSet::new(),
                });
            }
        }

        jobs.sort_by(|left, right| {
            left.output.cmp(&right.output).then_with(|| {
                left.template
                    .get_with_slash()
                    .cmp(right.template.get_with_slash())
            })
        });
        for (index, left) in jobs.iter().enumerate() {
            for right in &jobs[index + 1..] {
                if output_paths_collide(&left.output, &right.output) {
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
        Ok(Self { jobs, warnings })
    }

    pub fn into_parts(self) -> (Vec<PlannedRoute>, Vec<BuildWarning>) {
        (self.jobs, self.warnings)
    }
}

fn portable_output_key(output: &RoutePath) -> impl Iterator<Item = String> + '_ {
    output
        .as_virtual_path()
        .get_without_slash()
        .split('/')
        .map(str::to_lowercase)
}

fn output_paths_collide(left: &RoutePath, right: &RoutePath) -> bool {
    let mut left = portable_output_key(left);
    let mut right = portable_output_key(right);
    loop {
        match (left.next(), right.next()) {
            (Some(left), Some(right)) if left == right => {}
            (Some(_), Some(_)) => return false,
            _ => return true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_portable_and_ancestor_output_collisions() {
        let route = |path| RoutePath::new(path).unwrap();

        assert!(output_paths_collide(
            &route("Case.html"),
            &route("case.html")
        ));
        assert!(output_paths_collide(
            &route("foo.html"),
            &route("foo.html/bar.html")
        ));
        assert!(!output_paths_collide(
            &route("foo.html"),
            &route("foobar.html")
        ));
    }

    #[test]
    fn rejects_nonportable_static_paths() {
        for template in ["CON.typ", "bad:name.typ", "trailing./page.typ"] {
            assert!(RoutePath::from_template(Path::new(template)).is_err());
        }
        assert!(RoutePath::from_template(Path::new("docs/v1.2.typ")).is_ok());
    }
}
