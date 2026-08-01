use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use typst::Library;
use typst::foundations::Dict;
use typst::syntax::VirtualPath;
use typst::utils::LazyHash;

use crate::build::world::TypstSession;
use crate::engine::content;
use crate::engine::route::{self, ParamSet, RoutePath};

/// A deterministic, collision-free page plan.
pub(super) struct RoutePlan {
    jobs: Vec<PlannedRoute>,
    warnings: Vec<String>,
}

pub(super) struct PlannedRoute {
    pub template: PathBuf,
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
        let project = session.project();
        let templates = session.source_files()?;
        let source_root = project.src_dir();
        let mut jobs = Vec::new();
        let mut warnings = Vec::new();

        for template in templates {
            let virtual_path = VirtualPath::virtualize(&source_root, &template)
                .context("source template is outside src/")?;
            let relative = Path::new(virtual_path.get_without_slash());
            let pattern = route::parse_template(relative)
                .with_context(|| format!("invalid route template {}", relative.display()))?;
            if pattern.is_dynamic() {
                let evaluated = session
                    .evaluate(&template, base_library)
                    .with_context(|| format!("failed to probe {}", relative.display()))?;
                warnings.extend(evaluated.warnings);
                let routes = route::extract(&evaluated.content)
                    .with_context(|| format!("invalid route metadata in {}", relative.display()))?;
                if routes.is_empty() {
                    warnings.push(format!(
                        "{} has a dynamic route pattern but no <route> metadata",
                        relative.display()
                    ));
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
                validate_static_output(relative)?;
                jobs.push(PlannedRoute {
                    output: RoutePath::from_template(relative)?,
                    template,
                    params: ParamSet::new(),
                });
            }
        }

        jobs.sort_by(|left, right| {
            left.output
                .cmp(&right.output)
                .then_with(|| left.template.cmp(&right.template))
        });
        for (index, left) in jobs.iter().enumerate() {
            for right in &jobs[index + 1..] {
                let left_key = portable_output_key(&left.output);
                let right_key = portable_output_key(&right.output);
                if output_paths_collide(&left_key, &right_key) {
                    bail!(
                        "templates {} and {} generate conflicting outputs {} and {}",
                        left.template.display(),
                        right.template.display(),
                        left.output.as_path().display(),
                        right.output.as_path().display()
                    );
                }
            }
        }
        Ok(Self { jobs, warnings })
    }

    pub fn into_parts(self) -> (Vec<PlannedRoute>, Vec<String>) {
        (self.jobs, self.warnings)
    }
}

fn validate_static_output(template: &Path) -> Result<()> {
    let mut components = template.components().peekable();
    while let Some(component) = components.next() {
        let mut value = component.as_os_str().to_string_lossy().into_owned();
        if components.peek().is_none() {
            value = Path::new(&value)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
        }
        ensure!(
            route::valid_segment(&value),
            "non-portable static route segment `{value}`"
        );
    }
    Ok(())
}

fn portable_output_key(output: &RoutePath) -> Vec<String> {
    output
        .as_path()
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

fn is_component_prefix(left: &[String], right: &[String]) -> bool {
    left.len() < right.len() && right.starts_with(left)
}

fn output_paths_collide(left: &[String], right: &[String]) -> bool {
    left == right || is_component_prefix(left, right) || is_component_prefix(right, left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_portable_and_ancestor_output_collisions() {
        let key = |path| portable_output_key(&RoutePath::new(path).unwrap());

        assert!(output_paths_collide(&key("Case.html"), &key("case.html")));
        assert!(output_paths_collide(
            &key("foo.html"),
            &key("foo.html/bar.html")
        ));
        assert!(!output_paths_collide(&key("foo.html"), &key("foobar.html")));
    }

    #[test]
    fn rejects_nonportable_static_paths() {
        for template in ["CON.typ", "bad:name.typ", "trailing./page.typ"] {
            assert!(validate_static_output(Path::new(template)).is_err());
        }
        assert!(validate_static_output(Path::new("docs/v1.2.typ")).is_ok());
    }
}
