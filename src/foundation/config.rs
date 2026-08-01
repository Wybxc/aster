use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::foundations::{Dict, Value};

const DEFAULT_LIGHT: &str = "InspiredGitHub";
const DEFAULT_DARK: &str = "base16-eighties.dark";

/// Highlight theme configuration from `aster.toml`.
pub struct HighlightConfig {
    pub themes: Themes,
}

pub struct Themes {
    pub light: String,
    pub dark: String,
}

/// Complete parsed configuration from `aster.toml`.
///
/// Build once, share everywhere — no repeat file I/O.
pub struct AsterConfig {
    /// Typst-friendly config dict for `sys.inputs`.
    pub dict: Dict,
    /// Highlight theme settings (filled with defaults when absent).
    pub highlight: HighlightConfig,
}

impl AsterConfig {
    /// Read `aster.toml` and parse both `sys.inputs` dict and
    /// `[highlight]` section in a single pass.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let table: toml::Table = content
            .parse()
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let highlight = parse_highlight_inner(&table);
        let value: Value = toml::Value::Table(table)
            .try_into()
            .with_context(|| format!("failed to convert {} to Typst inputs", path.display()))?;

        let dict = match value {
            Value::Dict(d) => d,
            _ => bail!("unexpected value type from toml conversion"),
        };
        Ok(Self { dict, highlight })
    }
}

/// Extract `[highlight]` config from an already-parsed TOML table.
fn parse_highlight_inner(table: &toml::Table) -> HighlightConfig {
    let light = table
        .get("highlight")
        .and_then(|h| h.get("themes"))
        .and_then(|t| t.get("light"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| DEFAULT_LIGHT.to_string());

    let dark = table
        .get("highlight")
        .and_then(|h| h.get("themes"))
        .and_then(|t| t.get("dark"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| DEFAULT_DARK.to_string());

    HighlightConfig {
        themes: Themes { light, dark },
    }
}
