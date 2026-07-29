use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Str, Value};

/// Highlight theme configuration from `aster.toml`.
pub struct HighlightConfig {
    pub themes: Themes,
}

pub struct Themes {
    pub light: String,
    pub dark: String,
}

/// Parse highlight-specific config from `aster.toml`.
pub fn parse_highlight(path: &Path) -> Result<Option<HighlightConfig>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let table: toml::Table = content
        .parse()
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let Some(highlight) = table.get("highlight") else {
        return Ok(None);
    };
    let themes = highlight
        .get("themes")
        .and_then(|v| v.as_table())
        .context("expected [highlight.themes] section")?;
    let light = themes
        .get("light")
        .and_then(|v| v.as_str())
        .map(String::from)
        .context("expected [highlight.themes].light")?;
    let dark = themes
        .get("dark")
        .and_then(|v| v.as_str())
        .map(String::from)
        .context("expected [highlight.themes].dark")?;
    Ok(Some(HighlightConfig {
        themes: Themes { light, dark },
    }))
}

/// Parse `aster.toml` at the given path and return a [`Dict`] suitable for
/// `sys.inputs`.
pub fn parse_config(path: &Path) -> Result<Dict> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let table: toml::Table = content
        .parse()
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let value = toml::Value::Table(table);
    match toml_to_typst(value) {
        Value::Dict(d) => Ok(d),
        _ => bail!("unexpected value type from toml conversion"),
    }
}

/// Convert a parsed `toml::Value` into a typst [`Value`].
fn toml_to_typst(value: toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::Str(Str::from(s)),
        toml::Value::Integer(i) => Value::Int(i),
        toml::Value::Float(f) => Value::Float(f),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(dt) => Value::Str(Str::from(dt.to_string())),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_typst).collect()),
        toml::Value::Table(table) => Value::Dict(
            table
                .into_iter()
                .map(|(k, v)| (Str::from(k), toml_to_typst(v)))
                .collect(),
        ),
    }
}
