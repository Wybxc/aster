use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use typst::foundations::{Content, Value};
use typst::introspection::MetadataElem;

use crate::project::ProjectRoot;

/// Parameter assignments for a single generated page.
pub type ParamSet = Vec<(String, String)>;

/// Extract route declarations from compiled content.
///
/// Looks for `#metadata(((key: val, ...), ...)) <route>` elements.
/// The metadata value is an array of dicts, each dict representing one page's
/// parameter bindings.
pub fn extract(content: &Content) -> Vec<ParamSet> {
    let mut result = Vec::new();
    let _ = content.traverse(&mut |element| -> ControlFlow<()> {
        if element.label().map_or(false, |l| *l.resolve() == *"route")
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
        path_str = path_str.replace(&format!("[{}]", name), value);
    }
    project.output_dir().join(&path_str).with_extension("html")
}
