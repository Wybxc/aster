use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use typst::ecow::{EcoString, EcoVec};
use typst::foundations::{Label, Selector, Value};
use typst::introspection::{Introspector, MetadataElem};
use typst::syntax::VirtualPath;

/// Parameter assignments for one generated page.
pub type ParamSet = BTreeMap<EcoString, EcoString>;

/// A validated, portable path in Aster's virtual output tree.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RoutePath(VirtualPath);

impl RoutePath {
    /// Validate a relative output path and place it in the virtual output root.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        use std::path::Component;

        let path = path.as_ref();
        ensure!(!path.as_os_str().is_empty(), "route path cannot be empty");

        for component in path.components() {
            match component {
                Component::Normal(value) => {
                    let value = value.to_str().context("route path is not valid UTF-8")?;
                    ensure!(valid_segment(value), "non-portable route segment `{value}`");
                }
                Component::CurDir => bail!("route path cannot contain `.`"),
                Component::ParentDir => bail!("route path cannot contain `..`"),
                Component::RootDir | Component::Prefix(_) => {
                    bail!("route path must be relative")
                }
            }
        }

        let path = VirtualPath::virtualize(Path::new(""), path)
            .context("route path is not a valid virtual path")?;
        ensure!(!path.is_root(), "route path cannot be empty");
        Ok(Self(path))
    }

    /// Return the normalized path within the virtual output root.
    pub fn as_virtual_path(&self) -> &VirtualPath {
        &self.0
    }

    /// Whether two output paths cannot coexist on a portable filesystem.
    pub fn conflicts_with(&self, other: &Self) -> bool {
        let mut left = self
            .as_virtual_path()
            .get_without_slash()
            .split('/')
            .map(str::to_lowercase);
        let mut right = other
            .as_virtual_path()
            .get_without_slash()
            .split('/')
            .map(str::to_lowercase);

        loop {
            match (left.next(), right.next()) {
                (Some(left), Some(right)) if left == right => {}
                (Some(_), Some(_)) => return false,
                _ => return true,
            }
        }
    }
}

impl Ord for RoutePath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.get_with_slash().cmp(other.0.get_with_slash())
    }
}

impl PartialOrd for RoutePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for RoutePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.get_without_slash())
    }
}

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

    pub fn generate(&self, params: &ParamSet) -> Result<RoutePath> {
        let mut output = self.generate_path(params)?;
        output.as_mut_os_string().push(".html");
        RoutePath::new(output)
    }

    /// Generate an exact output path without appending a page extension.
    pub fn generate_endpoint(&self, params: &ParamSet) -> Result<RoutePath> {
        RoutePath::new(self.generate_path(params)?)
    }

    fn generate_path(&self, params: &ParamSet) -> Result<PathBuf> {
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
        for segment in &self.segments {
            if let [Part::Spread(name)] = segment.as_slice() {
                let value = params.get(name).expect("validated parameter exists");
                ensure!(
                    !value.is_empty(),
                    "spread parameter `{name}` cannot be empty"
                );
                let pieces = value.split('/').collect::<Vec<_>>();
                for piece in pieces {
                    ensure!(valid_segment(piece), "invalid route segment `{piece}`");
                    output.push(piece);
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
                            valid_segment(value),
                            "invalid value for route parameter `{name}`"
                        );
                        text.push_str(value);
                    }
                    Part::Spread(_) => unreachable!("spread is validated as standalone"),
                }
            }
            ensure!(valid_segment(&text), "invalid route segment `{text}`");
            output.push(text);
        }
        Ok(output)
    }
}

fn valid_segment(value: &str) -> bool {
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

/// Extract parameter assignments declared through `<aster-route>` metadata.
pub fn extract_params(introspector: &dyn Introspector) -> Result<EcoVec<ParamSet>> {
    let selector =
        Selector::Label(Label::construct("aster-route".into()).expect("route label is non-empty"));
    let mut result = EcoVec::new();
    for element in introspector.query(&selector) {
        let Some(metadata) = element.to_packed::<MetadataElem>() else {
            continue;
        };
        let declaration = metadata.value.clone();
        let Value::Array(items) = declaration else {
            bail!("route metadata must be an array of dictionaries");
        };
        for item in items {
            let Value::Dict(dict) = item else {
                bail!("route metadata must be an array of dictionaries");
            };
            if dict.is_empty() {
                bail!("route metadata must not contain an empty parameter set");
            }
            let mut params = ParamSet::new();
            for (name, value) in dict.iter() {
                let Value::Str(value) = value else {
                    bail!("route parameter `{}` must be a string", name.as_str());
                };
                params.insert(name.as_str().into(), value.as_str().into());
            }
            result.push(params);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            route
                .generate(&params)
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/blog/hello.html"
        );
        assert!(route.generate(&ParamSet::new()).is_err());
        assert!(
            route
                .generate(&ParamSet::from([("slug".into(), "../escape".into())]))
                .is_err()
        );
        assert!(
            route
                .generate(&ParamSet::from([
                    ("slug".into(), "hello".into()),
                    ("other".into(), "value".into()),
                ]))
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
                .as_virtual_path()
                .get_with_slash(),
            "/docs/a/b.html"
        );
        let plain = parse("docs/[path].typ").unwrap();
        assert!(
            plain
                .generate(&ParamSet::from([("path".into(), "a/b".into())]))
                .is_err()
        );
    }

    #[test]
    fn preserves_dots_and_rejects_nonportable_segments() {
        let route = parse("[slug].typ").unwrap();
        assert_eq!(
            route
                .generate(&ParamSet::from([("slug".into(), "v1.2".into())]))
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/v1.2.html"
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
    fn template_paths_determine_page_outputs() {
        assert_eq!(
            parse("index.typ")
                .unwrap()
                .generate(&ParamSet::new())
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/index.html"
        );
        assert_eq!(
            parse("about.typ")
                .unwrap()
                .generate(&ParamSet::new())
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/about.html"
        );

        let route = parse("blog/[slug]/index.typ").unwrap();
        assert_eq!(
            route
                .generate(&ParamSet::from([("slug".into(), "hello".into())]))
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/blog/hello/index.html"
        );

        let route = parse("[slug].typ").unwrap();
        assert_eq!(
            route
                .generate(&ParamSet::from([("slug".into(), "index".into())]))
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/index.html"
        );

        assert_eq!(
            parse("feed/[slug].xml.typ")
                .unwrap()
                .generate_endpoint(&ParamSet::from([("slug".into(), "latest".into())]))
                .unwrap()
                .as_virtual_path()
                .get_with_slash(),
            "/feed/latest.xml"
        );
    }

    #[test]
    fn detects_portable_and_ancestor_output_collisions() {
        let route = |path| RoutePath::new(path).unwrap();

        assert!(route("Case/index.html").conflicts_with(&route("case/index.html")));
        assert!(route("foo/index.html").conflicts_with(&route("foo/index.html/bar/index.html")));
        assert!(!route("foo/index.html").conflicts_with(&route("foobar/index.html")));
    }

    #[test]
    fn route_path_establishes_portable_virtual_path_invariant() {
        let path = RoutePath::new("docs/index.html").unwrap();
        assert_eq!(path.as_virtual_path().get_with_slash(), "/docs/index.html");

        for invalid in [
            "",
            ".",
            "../index.html",
            "docs/../index.html",
            "/index.html",
            "bad:name.html",
            "CON.html",
        ] {
            assert!(
                RoutePath::new(invalid).is_err(),
                "{invalid} must be rejected"
            );
        }
    }
}
