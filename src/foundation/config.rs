use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use typst::ecow::EcoString;
use typst::foundations::{Dict, Value};

const DEFAULT_LIGHT: &str = "InspiredGitHub";
const DEFAULT_DARK: &str = "base16-eighties.dark";

/// Complete `aster.toml` manifest, represented for both Typst and Aster.
pub(crate) struct ProjectManifest {
    /// Complete Typst-friendly manifest dictionary for `sys.inputs`.
    pub inputs: Dict,
    /// Strongly typed fields interpreted by Aster itself.
    pub config: AsterConfig,
}

/// Highlight theme configuration from `aster.toml`.
#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct HighlightConfig {
    pub themes: Themes,
}

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct Themes {
    pub light: EcoString,
    pub dark: EcoString,
}

impl Default for Themes {
    fn default() -> Self {
        Self {
            light: DEFAULT_LIGHT.into(),
            dark: DEFAULT_DARK.into(),
        }
    }
}

/// Aster-owned configuration extracted from the project manifest.
#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct AsterConfig {
    pub highlight: HighlightConfig,
}

impl ProjectManifest {
    /// Read `aster.toml` and create its Typst and typed Aster views.
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let table: toml::Table = content
            .parse()
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let value = toml::Value::Table(table);
        let config = AsterConfig::deserialize(value.clone())
            .with_context(|| format!("failed to extract Aster config from {}", path.display()))?;
        let value = Value::deserialize(value)
            .with_context(|| format!("failed to convert {} to Typst inputs", path.display()))?;

        let inputs = match value {
            Value::Dict(d) => d,
            _ => bail!("unexpected value type from toml conversion"),
        };
        Ok(Self { inputs, config })
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

        let manifest = ProjectManifest::load(&config_file).unwrap();

        assert_eq!(
            manifest.inputs.get("title").unwrap(),
            &Value::Str(Str::from("Aster"))
        );
        assert_eq!(
            manifest.inputs.get("published").unwrap(),
            &Value::Str(Str::from("1979-05-27T07:32:00Z"))
        );
        let Value::Dict(site) = manifest.inputs.get("site").unwrap() else {
            panic!("site must be a dictionary");
        };
        assert_eq!(site.get("enabled").unwrap(), &Value::Bool(true));
        assert!(matches!(
            manifest.inputs.get("highlight"),
            Ok(Value::Dict(_))
        ));
        assert_eq!(manifest.config.highlight.themes.light, "Solarized (light)");
        assert_eq!(manifest.config.highlight.themes.dark, "Solarized (dark)");
    }

    #[test]
    fn fills_missing_highlight_settings_with_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(
            &config_file,
            "[highlight.themes]\nlight = \"Solarized (light)\"\n",
        )
        .unwrap();

        let manifest = ProjectManifest::load(&config_file).unwrap();

        assert_eq!(manifest.config.highlight.themes.light, "Solarized (light)");
        assert_eq!(manifest.config.highlight.themes.dark, DEFAULT_DARK);
    }

    #[test]
    fn rejects_invalid_highlight_settings() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(&config_file, "[highlight]\nthemes = \"InspiredGitHub\"\n").unwrap();

        let error = match ProjectManifest::load(&config_file) {
            Ok(_) => panic!("invalid highlight settings must be rejected"),
            Err(error) => error,
        };

        assert!(
            format!("{error:#}").contains("invalid type"),
            "unexpected error: {error:#}"
        );
    }
}
