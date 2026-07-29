use std::ffi::OsStr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use typst::foundations::{Content, Value};
use typst::introspection::MetadataElem;

use crate::project::ProjectRoot;

/// Parameter assignments for a single generated page.
pub type ParamSet = Vec<(String, String)>;

// ---------------------------------------------------------------------------
// Template path parsing
// ---------------------------------------------------------------------------

/// A pre-parsed component of a template path.
#[derive(Debug, Clone)]
pub enum Component {
    /// Static text used as-is (directory or filename).
    Text(String),
    /// Contains `[param]` patterns (stem only, no extension).
    Pattern(String),
    /// A `[...param]` spread component.
    Spread(String),
}

/// Pre-parse a template path into structured components.
///
/// Strips file extensions, detects `[...]` spread patterns, and extracts
/// all `[name]` slots for later substitution.
pub fn parse_template(path: &Path) -> Vec<Component> {
    use std::path::Component as PathComp;

    let mut components = Vec::new();
    let comps: Vec<PathComp<'_>> = path.components().collect();

    for (i, comp) in comps.iter().enumerate() {
        let PathComp::Normal(os) = comp else { continue };
        let s = os.to_string_lossy();
        let is_last = i == comps.len() - 1;

        // Check spread: `[...name]` or `[...name].ext`
        if let Some(param) = spread_name(&s) {
            components.push(Component::Spread(param));
            continue;
        }

        // For file components, strip the extension.
        let text = if is_last {
            Path::new(&*s)
                .file_stem()
                .unwrap_or(os)
                .to_string_lossy()
                .to_string()
        } else {
            s.to_string()
        };

        if text.contains('[') {
            components.push(Component::Pattern(text));
        } else {
            components.push(Component::Text(text));
        }
    }

    components
}

/// Extract parameter names from a template path (convenience for probe detection).
pub fn parse_params(path: &Path) -> Vec<String> {
    let mut params = Vec::new();
    for comp in parse_template(path) {
        match comp {
            Component::Pattern(s) | Component::Spread(s) => {
                // Extract param names from the text.
                let mut pos = 0;
                while let Some(open) = s[pos..].find('[') {
                    let open = pos + open;
                    if let Some(close) = s[open..].find(']') {
                        let close = open + close;
                        if close > open + 1 {
                            let raw = &s[open + 1..close];
                            let param = raw.strip_prefix("..").unwrap_or(raw);
                            if !param.is_empty() {
                                params.push(param.to_string());
                            }
                        }
                        pos = close + 1;
                    } else {
                        break;
                    }
                }
            }
            Component::Text(_) => {}
        }
    }
    params
}

fn spread_name(s: &str) -> Option<String> {
    let stem = Path::new(s).file_stem().unwrap_or(OsStr::new(s));
    let t = stem.to_string_lossy();
    t.strip_prefix("[...")
        .and_then(|rest| rest.strip_suffix("]"))
        .map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// Route extraction
// ---------------------------------------------------------------------------

/// Extract route declarations from compiled content.
pub fn extract(content: &Content) -> Vec<ParamSet> {
    let mut result = Vec::new();
    let _ = content.traverse(&mut |element| -> ControlFlow<()> {
        if element.label().is_some_and(|l| *l.resolve() == *"route")
            && let Some(meta) = element.to_packed::<MetadataElem>()
            && let Value::Array(arr) = &meta.value
        {
            for item in arr {
                if let Value::Dict(dict) = item {
                    let mut params = ParamSet::new();
                    for (k, v) in dict.iter() {
                        if let Value::Str(s) = v {
                            params.push((k.to_string(), s.to_string()));
                        }
                    }
                    if !params.is_empty() {
                        result.push(params);
                    }
                }
            }
        }
        ControlFlow::Continue(())
    });
    result
}

// ---------------------------------------------------------------------------
// Output path generation
// ---------------------------------------------------------------------------

/// Compute the output path for a route-generated page.
///
/// Uses the pre-parsed template path to expand each component:
/// - `Text` parts are used as-is
/// - `Pattern` parts have `[name]` replaced with param values
/// - `Spread` parts expand the param value by `/` into multiple segments
/// - The final component gets a `.html` extension
pub fn output_path(project: &ProjectRoot, template: &[Component], params: &ParamSet) -> PathBuf {
    let mut parts: Vec<String> = Vec::new();

    for comp in template {
        match comp {
            Component::Text(t) => parts.push(t.clone()),
            Component::Pattern(s) => parts.push(fill_params(s, params)),
            Component::Spread(name) => {
                if let Some((_, value)) = params.iter().find(|(k, _)| *k == *name) {
                    for part in value.split('/') {
                        parts.push(part.to_string());
                    }
                }
            }
        }
    }

    let last = parts.len().saturating_sub(1);
    let mut output = project.output_dir();
    for (i, part) in parts.iter().enumerate() {
        if i == last {
            output = output.join(part).with_extension("html");
        } else {
            output = output.join(part);
        }
    }
    output
}

fn fill_params(s: &str, params: &ParamSet) -> String {
    let mut result = String::new();
    let mut pos = 0;
    while let Some(open) = s[pos..].find('[') {
        let abs_open = pos + open;
        result.push_str(&s[pos..abs_open]);
        if let Some(close) = s[abs_open..].find(']') {
            let name = &s[abs_open + 1..abs_open + close];
            if let Some((_, value)) = params.iter().find(|(k, _)| *k == name) {
                result.push_str(value);
            }
            pos = abs_open + close + 1;
        } else {
            result.push_str(&s[abs_open..]);
            pos = s.len();
            break;
        }
    }
    if pos < s.len() {
        result.push_str(&s[pos..]);
    }
    result
}
