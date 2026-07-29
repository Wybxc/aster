use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use typst::foundations::{Content, Value};
use typst::introspection::MetadataElem;

use crate::project::ProjectRoot;

/// Parameter assignments for a single generated page.
pub type ParamSet = Vec<(String, String)>;

// ---------------------------------------------------------------------------
// Route template — pre-parsed, validated
// ---------------------------------------------------------------------------

/// A single part within a route segment.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// Static text preserved verbatim.
    Static(String),
    /// A named parameter `[name]`.
    Param(String),
    /// A spread parameter `[...name]` — value may expand into multiple segments.
    Spread(String),
}

/// A pre-parsed and validated route template.
///
/// Segments correspond to path components, each consisting of parts
/// that are either static text or parameter slots.
#[derive(Debug, Clone)]
pub struct RouteTemplate {
    segments: Vec<Vec<Part>>,
}

/// Errors that can occur during template parsing.
#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error("unbalanced brackets in route path")]
    UnbalancedBrackets,
    #[error("empty brackets `[]` in route path")]
    EmptyBrackets,
    #[error("consecutive brackets `][` in route path")]
    ConsecutiveBrackets,
    #[error("rest parameter `[...` in segment {0} must be a standalone segment")]
    SpreadNotStandalone(usize),
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a template path (relative to `src_dir`) into a validated [`RouteTemplate`].
pub fn parse_template(path: &Path) -> Result<RouteTemplate, RouteError> {
    use std::path::Component;

    let mut segments = Vec::new();

    for comp in path.components() {
        let Component::Normal(os) = comp else {
            continue;
        };
        let s = os.to_string_lossy();

        // Strip file extension for the last component (file).
        let is_last = is_last_component(path, comp);
        let text = if is_last {
            Path::new(&*s).file_stem().unwrap_or(os).to_string_lossy()
        } else {
            s
        };

        let parts = parse_component(&text)?;
        if parts.is_empty() {
            continue;
        }

        // Validate spread standalone.
        for (i, part) in parts.iter().enumerate() {
            if matches!(part, Part::Spread(_)) && parts.len() > 1 {
                return Err(RouteError::SpreadNotStandalone(segments.len()));
            }
            if i > 0 && matches!(part, Part::Param(_)) && matches!(parts[i - 1], Part::Param(_)) {
                return Err(RouteError::ConsecutiveBrackets);
            }
        }

        segments.push(parts);
    }

    Ok(RouteTemplate { segments })
}

fn is_last_component(path: &Path, comp: std::path::Component<'_>) -> bool {
    let mut it = path.components();
    while let Some(c) = it.next() {
        if std::ptr::eq(&c, &comp) {
            return it.next().is_none();
        }
    }
    false
}

/// Split a route component string into parts at `[...]` boundaries.
fn parse_component(s: &str) -> Result<Vec<Part>, RouteError> {
    let mut parts = Vec::new();
    let mut pos = 0;

    while pos < s.len() {
        if s[pos..].starts_with('[') {
            // Scan for matching ']'
            let open_at = pos;
            pos += 1;
            let close_at = match s[pos..].find(']') {
                Some(i) => pos + i,
                None => return Err(RouteError::UnbalancedBrackets),
            };

            let inner = &s[open_at + 1..close_at];

            if inner.is_empty() {
                return Err(RouteError::EmptyBrackets);
            }

            if let Some(name) = inner.strip_prefix("...") {
                if name.is_empty() {
                    return Err(RouteError::EmptyBrackets);
                }
                parts.push(Part::Spread(name.to_string()));
            } else {
                parts.push(Part::Param(inner.to_string()));
            }

            pos = close_at + 1;
        } else {
            // Scan static text until next '[' or end.
            let start = pos;
            pos = match s[pos..].find('[') {
                Some(i) => pos + i,
                None => s.len(),
            };
            parts.push(Part::Static(s[start..pos].to_string()));
        }
    }

    Ok(parts)
}

// ---------------------------------------------------------------------------
// Parameter name extraction
// ---------------------------------------------------------------------------

/// Extract all parameter names from a template path (convenience for probe).
pub fn parse_params(path: &Path) -> Vec<String> {
    let Ok(tpl) = parse_template(path) else {
        return vec![];
    };
    let mut params = Vec::new();
    for seg in &tpl.segments {
        for part in seg {
            match part {
                Part::Param(name) | Part::Spread(name) => params.push(name.clone()),
                Part::Static(_) => {}
            }
        }
    }
    params
}

// ---------------------------------------------------------------------------
// Route extraction from compiled content
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

impl RouteTemplate {
    /// Compute the output path for a given set of params.
    ///
    /// Each segment is assembled by substituting param values for their slots,
    /// then joined with `/`. The last segment gets a `.html` extension.
    pub fn generate(&self, project: &ProjectRoot, params: &ParamSet) -> PathBuf {
        let mut out_parts: Vec<String> = Vec::new();

        for seg in &self.segments {
            let mut seg_text = String::new();
            for part in seg {
                match part {
                    Part::Static(s) => seg_text.push_str(s),
                    Part::Param(name) | Part::Spread(name) => {
                        if let Some((_, value)) = params.iter().find(|(k, _)| k == name) {
                            seg_text.push_str(value);
                        }
                    }
                }
            }
            // Spread: the param itself may contain '/', expanding into sub-segments.
            // We handle this by splitting after assembling the segment text.
            for sub in seg_text.split('/') {
                out_parts.push(sub.to_string());
            }
        }

        // Build the output path.
        let last = out_parts.len().saturating_sub(1);
        let mut output = project.output_dir();
        for (i, part) in out_parts.iter().enumerate() {
            if i == last {
                output = output.join(part).with_extension("html");
            } else {
                output = output.join(part);
            }
        }
        output
    }
}

// Retained for backward compatibility: delegates to new RouteTemplate-based path.
pub fn output_path(project: &ProjectRoot, tpl: &RouteTemplate, params: &ParamSet) -> PathBuf {
    tpl.generate(project, params)
}
