use std::path::Path;

use anyhow::{Context, Result, bail};
use typst::ecow::EcoString;
use typst::foundations::{Dict, Value};

const DEFAULT_LIGHT: &str = "InspiredGitHub";
const DEFAULT_DARK: &str = "base16-eighties.dark";

/// Highlight theme configuration from `aster.toml`.
pub(crate) struct HighlightConfig {
    pub themes: Themes,
}

pub(crate) struct Themes {
    pub light: EcoString,
    pub dark: EcoString,
}

/// Complete parsed configuration from `aster.toml`.
///
/// Build once, share everywhere — no repeat file I/O.
pub(crate) struct AsterConfig {
    /// Typst-friendly config dict for `sys.inputs`.
    pub dict: Dict,
    /// Highlight theme settings (filled with defaults when absent).
    pub highlight: HighlightConfig,
}

impl AsterConfig {
    /// Read `aster.toml` and parse both `sys.inputs` dict and
    /// `[highlight]` section in a single pass.
    pub(crate) fn load(path: &Path) -> Result<Self> {
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
        .map(EcoString::from)
        .unwrap_or_else(|| DEFAULT_LIGHT.into());

    let dark = table
        .get("highlight")
        .and_then(|h| h.get("themes"))
        .and_then(|t| t.get("dark"))
        .and_then(|v| v.as_str())
        .map(EcoString::from)
        .unwrap_or_else(|| DEFAULT_DARK.into());

    HighlightConfig {
        themes: Themes { light, dark },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use typst::foundations::{Str, Value};

    #[test]
    fn loads_typst_inputs_and_highlight_config() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(
            &config_file,
            concat!(
                "title = \"Aster\"\n",
                "published = 1979-05-27T07:32:00Z\n",
                "[site]\n",
                "enabled = true\n",
                "[highlight.themes]\n",
                "light = \"Solarized (light)\"\n",
                "dark = \"Solarized (dark)\"\n",
            ),
        )
        .unwrap();

        let config = AsterConfig::load(&config_file).unwrap();

        assert_eq!(
            config.dict.get("title").unwrap(),
            &Value::Str(Str::from("Aster"))
        );
        assert_eq!(
            config.dict.get("published").unwrap(),
            &Value::Str(Str::from("1979-05-27T07:32:00Z"))
        );
        let Value::Dict(site) = config.dict.get("site").unwrap() else {
            panic!("site must be a dictionary");
        };
        assert_eq!(site.get("enabled").unwrap(), &Value::Bool(true));
        assert_eq!(config.highlight.themes.light, "Solarized (light)");
        assert_eq!(config.highlight.themes.dark, "Solarized (dark)");
    }
}
