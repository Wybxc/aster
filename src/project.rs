use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Str, Value};

/// Search upward from `dir` for an `aster.toml` file.
pub fn find_root(dir: &Path) -> Option<PathBuf> {
    let mut current = Some(dir);
    while let Some(path) = current {
        if path.join("aster.toml").exists() {
            return Some(path.to_owned());
        }
        current = path.parent();
    }
    None
}

/// Collect all `.typ` files under `dir` (iterative, depth-first).
///
/// Propagates I/O errors from individual entries.
pub fn find_typ_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_owned()];

    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "typ") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

// ---------------------------------------------------------------------------
// Project layout
// ---------------------------------------------------------------------------

/// The page templates directory (`<root>/src`).
pub fn src_dir(root: &Path) -> PathBuf {
    root.join("src")
}

/// The content collections directory (`<root>/content`).
pub fn content_dir(root: &Path) -> PathBuf {
    root.join("content")
}

/// The build output directory (`<root>/dist`).
pub fn output_dir(root: &Path) -> PathBuf {
    root.join("dist")
}

/// Compute the output HTML path for a page template.
///
/// Returns `Some(<root>/dist/<relative>.html)` when `page` is inside
/// `src_dir(root)`, or `None` if it isn't.
pub fn page_output_path(page: &Path, root: &Path) -> Option<PathBuf> {
    let src = src_dir(root);
    let relative = page.strip_prefix(&src).ok()?;
    Some(output_dir(root).join(relative).with_extension("html"))
}

// ---------------------------------------------------------------------------
// Project configuration
// ---------------------------------------------------------------------------

/// Parse `aster.toml` at the given path and return a [`Dict`] suitable for
/// `sys.inputs`.
pub fn parse_config(path: &Path) -> Result<Dict> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let table: toml::Table = content
        .parse()
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let value = toml::Value::Table(table);
    match toml_to_typst(&value) {
        Value::Dict(d) => Ok(d),
        _ => bail!("unexpected value type from toml conversion"),
    }
}

/// Convert a parsed `toml::Value` into a typst [`Value`].
fn toml_to_typst(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::Str(Str::from(s.as_str())),
        toml::Value::Integer(i) => Value::Int(*i),
        toml::Value::Float(f) => Value::Float(*f),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(dt) => Value::Str(Str::from(dt.to_string())),
        toml::Value::Array(arr) => Value::Array(arr.iter().map(toml_to_typst).collect()),
        toml::Value::Table(table) => Value::Dict(
            table
                .iter()
                .map(|(k, v)| (Str::from(k.as_str()), toml_to_typst(v)))
                .collect(),
        ),
    }
}
