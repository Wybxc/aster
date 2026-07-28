use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use typst::foundations::{Content, Value};
use typst::introspection::MetadataElem;

use crate::project::ProjectRoot;

/// Parameter assignments for a single generated page.
pub type ParamSet = Vec<(String, String)>;

/// Extract parameter names from a template path.
///
/// Extracts all `[name]` and `[...name]` (spread/rest) patterns from each
/// path component's stem, matching Astro's convention. Supports multiple
/// params within a single component, e.g. `[lang]-[version].typ` → `["lang", "version"]`.
pub fn parse_params(path: &Path) -> Vec<String> {
    let mut params = Vec::new();
    for comp in path.components() {
        let os = comp.as_os_str();
        let name = Path::new(os).file_stem().unwrap_or(os).to_string_lossy();
        let mut pos = 0;
        let s = name.as_ref();
        while let Some(open) = s[pos..].find('[') {
            let open = pos + open;
            if let Some(close) = s[open..].find(']') {
                let close = open + close;
                if close > open + 1 {
                    let raw = &s[open + 1..close];
                    // Strip leading `..` for spread params (`[...slug]` → `slug`).
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
    params
}

/// Extract route declarations from compiled content.
///
/// Looks for `#metadata(((key: val, ...), ...)) <route>` elements.
/// The metadata value is an array of dicts, each dict representing one page's
/// parameter bindings.
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

/// Compute the output path for a route-generated page.
///
/// Replaces `[param]` placeholders in the template path with actual values.
pub fn output_path(project: &ProjectRoot, template: &Path, params: &ParamSet) -> PathBuf {
    let relative = template
        .strip_prefix(project.src_dir())
        .expect("template must be under src/");
    let mut path_str = relative.to_string_lossy().to_string();
    for (name, value) in params {
        // Replace spread form first to avoid partial matches.
        path_str = path_str.replace(&format!("[...{}]", name), value);
        path_str = path_str.replace(&format!("[{}]", name), value);
    }
    project.output_dir().join(&path_str).with_extension("html")
}
