use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use typst::Library;
use typst::ecow::EcoString;
use typst::foundations::{Content, Dict, Value};
use typst::introspection::MetadataElem;
use typst::utils::LazyHash;

use crate::compile::TypstSession;
use crate::content;
use crate::output::OutputPath;

/// Parameter assignments for one generated page.
pub type ParamSet = BTreeMap<EcoString, EcoString>;

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Static(EcoString),
    Param(EcoString),
    Spread(EcoString),
}

/// A pre-parsed and validated route template.
#[derive(Debug, Clone)]
pub struct RouteTemplate {
    segments: Vec<Vec<Part>>,
    parameters: BTreeSet<EcoString>,
}

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("unbalanced brackets in route path")]
    UnbalancedBrackets,
    #[error("empty brackets `[]` in route path")]
    EmptyBrackets,
    #[error("consecutive brackets `][` in route path")]
    ConsecutiveBrackets,
    #[error("spread parameter in segment {0} must be a standalone segment")]
    SpreadNotStandalone(usize),
    #[error("duplicate route parameter `{0}`")]
    DuplicateParameter(EcoString),
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum RouteMetadataError {
    #[error("route metadata must be an array of dictionaries")]
    InvalidShape,
    #[error("route parameter `{0}` must be a string")]
    NonStringParameter(EcoString),
    #[error("route metadata must not contain an empty parameter set")]
    EmptyParameterSet,
}

pub fn parse_template(path: &Path) -> Result<RouteTemplate, RouteError> {
    use std::path::Component;

    let normal: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect();
    let mut segments = Vec::with_capacity(normal.len());
    let mut parameters = BTreeSet::new();

    for (index, os) in normal.iter().enumerate() {
        let text = if index + 1 == normal.len() {
            Path::new(os).file_stem().unwrap_or(os).to_string_lossy()
        } else {
            os.to_string_lossy()
        };
        let parts = parse_component(&text)?;
        if parts.is_empty() {
            continue;
        }

        for (part_index, part) in parts.iter().enumerate() {
            if matches!(part, Part::Spread(_)) && parts.len() > 1 {
                return Err(RouteError::SpreadNotStandalone(segments.len()));
            }
            if part_index > 0
                && matches!(part, Part::Param(_))
                && matches!(parts[part_index - 1], Part::Param(_))
            {
                return Err(RouteError::ConsecutiveBrackets);
            }
            if let Part::Param(name) | Part::Spread(name) = part
                && !parameters.insert(name.clone())
            {
                return Err(RouteError::DuplicateParameter(name.clone()));
            }
        }
        segments.push(parts);
    }

    Ok(RouteTemplate {
        segments,
        parameters,
    })
}

fn parse_component(text: &str) -> Result<Vec<Part>, RouteError> {
    let mut parts = Vec::new();
    let mut pos = 0;

    while pos < text.len() {
        if text[pos..].starts_with('[') {
            let open_at = pos;
            pos += 1;
            let close_at = text[pos..]
                .find(']')
                .map(|index| pos + index)
                .ok_or(RouteError::UnbalancedBrackets)?;
            let inner = &text[open_at + 1..close_at];
            if inner.is_empty() {
                return Err(RouteError::EmptyBrackets);
            }
            if let Some(name) = inner.strip_prefix("...") {
                if name.is_empty() {
                    return Err(RouteError::EmptyBrackets);
                }
                parts.push(Part::Spread(name.into()));
            } else {
                parts.push(Part::Param(inner.into()));
            }
            pos = close_at + 1;
        } else {
            let start = pos;
            pos = text[pos..]
                .find('[')
                .map(|index| pos + index)
                .unwrap_or(text.len());
            parts.push(Part::Static(text[start..pos].into()));
        }
    }
    Ok(parts)
}

impl RouteTemplate {
    pub fn is_dynamic(&self) -> bool {
        !self.parameters.is_empty()
    }

    pub fn generate(&self, params: &ParamSet) -> Result<OutputPath> {
        let supplied: BTreeSet<_> = params.keys().cloned().collect();
        let missing: Vec<_> = self.parameters.difference(&supplied).cloned().collect();
        let extra: Vec<_> = supplied.difference(&self.parameters).cloned().collect();
        ensure!(
            missing.is_empty(),
            "missing route parameter(s): {}",
            join_names(&missing)
        );
        ensure!(
            extra.is_empty(),
            "unexpected route parameter(s): {}",
            join_names(&extra)
        );

        let mut output = PathBuf::new();
        for (index, segment) in self.segments.iter().enumerate() {
            if let [Part::Spread(name)] = segment.as_slice() {
                let value = params.get(name).expect("validated parameter exists");
                ensure!(
                    !value.is_empty(),
                    "spread parameter `{name}` cannot be empty"
                );
                let pieces = value.split('/').collect::<Vec<_>>();
                for (piece_index, piece) in pieces.iter().enumerate() {
                    ensure!(
                        valid_parameter_segment(piece),
                        "invalid route segment `{piece}`"
                    );
                    if index + 1 == self.segments.len() && piece_index + 1 == pieces.len() {
                        output.push(format!("{piece}.html"));
                    } else {
                        output.push(piece);
                    }
                }
                continue;
            }

            let mut text = String::new();
            for part in segment {
                match part {
                    Part::Static(value) => text.push_str(value),
                    Part::Param(name) => {
                        let value = params.get(name).expect("validated parameter exists");
                        ensure!(
                            valid_parameter_segment(value),
                            "invalid value for route parameter `{name}`"
                        );
                        text.push_str(value);
                    }
                    Part::Spread(_) => unreachable!("spread is validated as standalone"),
                }
            }
            ensure!(
                valid_parameter_segment(&text),
                "invalid route segment `{text}`"
            );
            if index + 1 == self.segments.len() {
                text.push_str(".html");
            }
            output.push(text);
        }
        OutputPath::new(output)
    }
}

fn valid_parameter_segment(value: &str) -> bool {
    const WINDOWS_DEVICES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = value.split('.').next().unwrap_or(value);
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.ends_with(['.', ' '])
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' | '#' | '%'
                )
        })
        && !WINDOWS_DEVICES
            .iter()
            .any(|device| stem.eq_ignore_ascii_case(device))
}

fn join_names(names: &[EcoString]) -> String {
    names
        .iter()
        .map(EcoString::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[comemo::memoize]
pub fn extract(content: &Content) -> Result<Vec<ParamSet>, RouteMetadataError> {
    let mut declarations = Vec::new();
    let _ = content.traverse(&mut |element| {
        if element
            .label()
            .is_some_and(|label| *label.resolve() == *"route")
            && let Some(metadata) = element.to_packed::<MetadataElem>()
        {
            declarations.push(metadata.value.clone());
        }
        std::ops::ControlFlow::<()>::Continue(())
    });

    let mut result = Vec::new();
    for declaration in declarations {
        let Value::Array(items) = declaration else {
            return Err(RouteMetadataError::InvalidShape);
        };
        for item in items {
            let Value::Dict(dict) = item else {
                return Err(RouteMetadataError::InvalidShape);
            };
            if dict.is_empty() {
                return Err(RouteMetadataError::EmptyParameterSet);
            }
            let mut params = ParamSet::new();
            for (name, value) in dict.iter() {
                let Value::Str(value) = value else {
                    return Err(RouteMetadataError::NonStringParameter(name.as_str().into()));
                };
                params.insert(name.as_str().into(), value.as_str().into());
            }
            result.push(params);
        }
    }
    Ok(result)
}

/// A deterministic, collision-free page plan.
pub struct RoutePlan {
    jobs: Vec<PlannedRoute>,
    warnings: Vec<String>,
}

pub struct PlannedRoute {
    pub template: PathBuf,
    pub output: OutputPath,
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
        let mut jobs = Vec::new();
        let mut warnings = Vec::new();

        for template in templates {
            let relative = template
                .strip_prefix(project.src_dir())
                .context("source template is outside src/")?;
            let route = parse_template(relative)
                .with_context(|| format!("invalid route template {}", relative.display()))?;
            if route.is_dynamic() {
                let evaluated = session
                    .evaluate(&template, base_library)
                    .with_context(|| format!("failed to probe {}", relative.display()))?;
                warnings.extend(evaluated.warnings);
                let routes = extract(&evaluated.content)
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
                        output: route.generate(&params)?,
                        params,
                    });
                }
            } else {
                validate_static_output(relative)?;
                jobs.push(PlannedRoute {
                    output: OutputPath::from_template(relative)?,
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
                if left_key == right_key
                    || is_component_prefix(&left_key, &right_key)
                    || is_component_prefix(&right_key, &left_key)
                {
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
            valid_parameter_segment(&value),
            "non-portable static route segment `{value}`"
        );
    }
    Ok(())
}

fn portable_output_key(output: &OutputPath) -> Vec<String> {
    output
        .as_path()
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

fn is_component_prefix(left: &[String], right: &[String]) -> bool {
    left.len() < right.len() && right.starts_with(left)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectRoot;
    use typst::text::TextElem;

    fn fixture(files: &[&str]) -> (tempfile::TempDir, ProjectRoot) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        for file in files {
            let path = root.join("src").join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        let project = ProjectRoot::new(root.to_owned()).unwrap();
        (temp, project)
    }

    fn build_plan(project: &crate::project::ProjectRoot) -> Result<RoutePlan> {
        let session = TypstSession::new(project.clone());
        let inputs = content::install(Dict::new(), content::empty()).unwrap();
        let library = session.library(inputs.clone());
        RoutePlan::build(&session, &inputs, &library)
    }

    fn write_routes(project: &crate::project::ProjectRoot, template: &str, routes: &str) {
        std::fs::write(
            project.src_dir().join(template),
            format!("#metadata({routes}) <route>"),
        )
        .unwrap();
    }

    fn parse(path: &str) -> Result<RouteTemplate, RouteError> {
        parse_template(Path::new(path))
    }

    #[test]
    fn parses_static_and_dynamic_parts() {
        let route = parse("blog/prefix[slug].typ").unwrap();
        assert!(route.is_dynamic());
        assert_eq!(route.parameters, BTreeSet::from(["slug".into()]));
    }

    #[test]
    fn route_metadata_analysis_is_memoized_by_content() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().display().to_string();
        let content = TextElem::packed(marker.clone());

        assert!(extract(&content).unwrap().is_empty());
        assert!(!comemo::testing::last_was_hit());

        assert!(extract(&content).unwrap().is_empty());
        assert!(comemo::testing::last_was_hit());

        let changed = TextElem::packed(format!("{marker}-changed"));
        assert!(extract(&changed).unwrap().is_empty());
        assert!(!comemo::testing::last_was_hit());
    }

    #[test]
    fn rejects_malformed_and_duplicate_parameters() {
        assert!(matches!(
            parse("[slug.typ"),
            Err(RouteError::UnbalancedBrackets)
        ));
        assert!(matches!(parse("[].typ"), Err(RouteError::EmptyBrackets)));
        assert!(matches!(
            parse("[a][b].typ"),
            Err(RouteError::ConsecutiveBrackets)
        ));
        assert!(matches!(
            parse("prefix[...slug].typ"),
            Err(RouteError::SpreadNotStandalone(0))
        ));
        assert!(matches!(
            parse("[slug]/[slug].typ"),
            Err(RouteError::DuplicateParameter(_))
        ));
    }

    #[test]
    fn generates_validated_dynamic_output() {
        let route = parse("blog/[slug].typ").unwrap();
        let params = ParamSet::from([("slug".into(), "hello".into())]);
        assert_eq!(
            route.generate(&params).unwrap().as_path(),
            Path::new("blog/hello.html")
        );
        assert!(route.generate(&ParamSet::new()).is_err());
        assert!(
            route
                .generate(&ParamSet::from([("slug".into(), "../escape".into())]))
                .is_err()
        );
    }

    #[test]
    fn only_spread_parameters_create_segments() {
        let spread = parse("docs/[...path].typ").unwrap();
        assert_eq!(
            spread
                .generate(&ParamSet::from([("path".into(), "a/b".into())]))
                .unwrap()
                .as_path(),
            Path::new("docs/a/b.html")
        );
        let plain = parse("docs/[path].typ").unwrap();
        assert!(
            plain
                .generate(&ParamSet::from([("path".into(), "a/b".into())]))
                .is_err()
        );
    }

    #[test]
    fn route_plan_is_sorted_and_probes_dynamic_templates_once() {
        let (_temp, project) = fixture(&["z.typ", "blog/[slug].typ", "a.typ"]);
        write_routes(&project, "blog/[slug].typ", "((slug: \"post\"),)");
        let plan = build_plan(&project).unwrap();
        let (jobs, warnings) = plan.into_parts();

        assert!(warnings.is_empty());
        assert_eq!(
            jobs.iter()
                .map(|job| job.output.as_path())
                .collect::<Vec<_>>(),
            vec![
                Path::new("a.html"),
                Path::new("blog/post.html"),
                Path::new("z.html"),
            ]
        );
    }

    #[test]
    fn route_plan_rejects_static_dynamic_collision() {
        let (_temp, project) = fixture(&["post.typ", "[slug].typ"]);
        write_routes(&project, "[slug].typ", "((slug: \"post\"),)");
        assert!(build_plan(&project).is_err());
    }

    #[test]
    fn preserves_dots_and_rejects_nonportable_segments() {
        let route = parse("[slug].typ").unwrap();
        assert_eq!(
            route
                .generate(&ParamSet::from([("slug".into(), "v1.2".into())]))
                .unwrap()
                .as_path(),
            Path::new("v1.2.html")
        );
        for invalid in ["CON", "bad?name", "fragment#name", "trail."] {
            assert!(
                route
                    .generate(&ParamSet::from([("slug".into(), invalid.into())]))
                    .is_err(),
                "{invalid} must be rejected"
            );
        }
    }

    #[test]
    fn route_plan_rejects_portable_and_ancestor_collisions() {
        let (_temp, project) = fixture(&["[slug].typ"]);
        write_routes(
            &project,
            "[slug].typ",
            "((slug: \"Case\"), (slug: \"case\"))",
        );
        assert!(build_plan(&project).is_err());

        let (_temp, project) = fixture(&["foo.typ", "foo.html/bar.typ"]);
        assert!(build_plan(&project).is_err());
    }

    #[test]
    fn route_plan_rejects_nonportable_static_paths() {
        for template in ["CON.typ", "bad:name.typ", "trailing./page.typ"] {
            let (_temp, project) = fixture(&[template]);
            assert!(build_plan(&project).is_err());
        }
    }

    #[test]
    fn route_plan_reports_missing_dynamic_metadata() {
        let (_temp, project) = fixture(&["[slug].typ"]);
        let plan = build_plan(&project).unwrap();
        let (jobs, warnings) = plan.into_parts();
        assert!(jobs.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn rejects_extra_parameters() {
        let route = parse("[slug].typ").unwrap();
        let params = ParamSet::from([
            ("slug".into(), "hello".into()),
            ("other".into(), "value".into()),
        ]);
        assert!(route.generate(&params).is_err());
    }
}
