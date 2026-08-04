use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use typst::ecow::{EcoString, EcoVec};
use typst::foundations::{Dict, Value};

/// Complete `aster.toml` manifest, represented for both Typst and Aster.
pub(crate) struct ProjectManifest {
    /// Complete Typst-friendly manifest dictionary for `sys.inputs`.
    pub inputs: Dict,
    /// Strongly typed fields interpreted by Aster itself.
    pub config: AsterConfig,
}

/// Highlight theme configuration from `aster.toml`.
#[derive(Clone, Deserialize)]
#[serde(default)]
pub(crate) struct HighlightConfig {
    pub enabled: bool,
    pub themes: Themes,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            themes: Themes::default(),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub(crate) struct Themes {
    pub light: EcoString,
    pub dark: EcoString,
}

impl Default for Themes {
    fn default() -> Self {
        Self {
            light: "InspiredGitHub".into(),
            dark: "base16-eighties.dark".into(),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PathsConfig {
    pub pages: EcoString,
    pub content: EcoString,
    pub public: EcoString,
    pub output: EcoString,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            pages: "pages".into(),
            content: "content".into(),
            public: "public".into(),
            output: "dist".into(),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct OutputConfig {
    pub assets: EcoString,
    pub pretty: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            assets: "_assets".into(),
            pretty: false,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub(crate) struct AssetsConfig {
    pub image_inline_threshold: usize,
}

impl Default for AssetsConfig {
    fn default() -> Self {
        Self {
            image_inline_threshold: 1024,
        }
    }
}

/// CSS bundling and transformation settings.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq)]
#[serde(default, rename_all = "kebab-case")]
pub(crate) struct CssConfig {
    /// Remove whitespace and apply size optimizations to generated CSS.
    pub minify: bool,
    /// Browserslist queries used for syntax lowering and vendor prefixes.
    pub targets: EcoVec<EcoString>,
    /// Enable parsing and transforming `@custom-media` rules.
    pub custom_media: bool,
}

impl Default for CssConfig {
    fn default() -> Self {
        Self {
            minify: true,
            targets: EcoVec::new(),
            custom_media: false,
        }
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub(crate) struct TypstConfig {
    pub fonts: FontConfig,
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub(crate) struct FontConfig {
    pub paths: Vec<EcoString>,
    pub system: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            system: true,
        }
    }
}

/// Aster-owned configuration extracted from the project manifest.
#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct AsterConfig {
    pub paths: PathsConfig,
    pub output: OutputConfig,
    pub assets: AssetsConfig,
    pub css: CssConfig,
    pub typst: TypstConfig,
    pub highlight: HighlightConfig,
}

impl ProjectManifest {
    /// Parse `aster.toml` into its Typst and typed Aster views.
    pub(crate) fn parse(content: &[u8], path: &Path) -> Result<Self> {
        let content = std::str::from_utf8(content)
            .with_context(|| format!("{} is not valid UTF-8", path.display()))?;
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

    fn load(path: &Path) -> Result<ProjectManifest> {
        let content = std::fs::read(path)?;
        ProjectManifest::parse(&content, path)
    }

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

        let manifest = load(&config_file).unwrap();

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

        let manifest = load(&config_file).unwrap();

        assert_eq!(manifest.config.highlight.themes.light, "Solarized (light)");
        assert_eq!(
            manifest.config.highlight.themes.dark,
            "base16-eighties.dark"
        );
    }

    #[test]
    fn loads_build_options_and_fills_their_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(
            &config_file,
            concat!(
                "[paths]\n",
                "pages = \"routes\"\n",
                "public = \"files\"\n",
                "output = \"public\"\n",
                "[output]\n",
                "assets = \"static/generated\"\n",
                "pretty = true\n",
                "[assets]\n",
                "image-inline-threshold = 2048\n",
                "[css]\n",
                "minify = false\n",
                "targets = [\"last 2 Chrome versions\", \"Firefox ESR\"]\n",
                "custom-media = true\n",
                "[typst.fonts]\n",
                "paths = [\"fonts\"]\n",
                "system = false\n",
                "[highlight]\n",
                "enabled = false\n",
            ),
        )
        .unwrap();

        let config = load(&config_file).unwrap().config;

        assert_eq!(config.paths.pages, "routes");
        assert_eq!(config.paths.content, "content");
        assert_eq!(config.paths.public, "files");
        assert_eq!(config.paths.output, "public");
        assert_eq!(config.output.assets, "static/generated");
        assert!(config.output.pretty);
        assert_eq!(config.assets.image_inline_threshold, 2048);
        assert!(!config.css.minify);
        assert_eq!(
            config
                .css
                .targets
                .iter()
                .map(EcoString::as_str)
                .collect::<Vec<_>>(),
            ["last 2 Chrome versions", "Firefox ESR"]
        );
        assert!(config.css.custom_media);
        assert_eq!(config.typst.fonts.paths, ["fonts"]);
        assert!(!config.typst.fonts.system);
        assert!(!config.highlight.enabled);
        assert_eq!(config.highlight.themes.light, "InspiredGitHub");
    }

    #[test]
    fn rejects_invalid_highlight_settings() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(&config_file, "[highlight]\nthemes = \"InspiredGitHub\"\n").unwrap();

        let error = match load(&config_file) {
            Ok(_) => panic!("invalid highlight settings must be rejected"),
            Err(error) => error,
        };

        assert!(
            format!("{error:#}").contains("invalid type"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn rejects_removed_clean_urls_setting() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(&config_file, "[output]\nclean-urls = true\n").unwrap();

        let error = match load(&config_file) {
            Ok(_) => panic!("removed output setting must be rejected"),
            Err(error) => error,
        };

        assert!(
            format!("{error:#}").contains("unknown field `clean-urls`"),
            "unexpected error: {error:#}"
        );
    }
}
