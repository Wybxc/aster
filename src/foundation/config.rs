//! Typed views of the project manifest consumed by Aster.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use typst::ecow::{EcoString, EcoVec, eco_vec};

/// Highlight theme configuration from `aster.toml`.
#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct HighlightConfig {
    /// Whether syntax highlighting is enabled.
    pub enabled: bool,
    /// Light and dark syntax-highlighting themes.
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

/// Syntax-highlighting themes selected for light and dark color schemes.
#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct Themes {
    /// Syntect theme used for light color schemes.
    pub light: EcoString,
    /// Syntect theme used for dark color schemes.
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

/// Configured project directories.
#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PathsConfig {
    /// Directory containing page and endpoint templates.
    pub pages: EcoString,
    /// Directory containing content collections.
    pub content: EcoString,
    /// Directory copied verbatim into the output.
    pub public: EcoString,
    /// Directory receiving the generated site.
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

/// Generated-output settings.
#[derive(Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct OutputConfig {
    /// Directory for generated resources, relative to the output directory.
    pub assets: EcoString,
    /// Whether generated HTML is formatted with indentation.
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

/// Resource processing settings.
#[derive(Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct AssetsConfig {
    /// Maximum decoded image size retained as a data URL.
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
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct CssConfig {
    /// Serialize generated CSS without unnecessary whitespace.
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
            targets: eco_vec!["defaults".into()],
            custom_media: false,
        }
    }
}

/// Additional project paths observed by development commands.
#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WatchConfig {
    /// Additional project-relative files or trees observed by watch commands.
    pub paths: EcoVec<EcoString>,
}

/// Typst compiler settings.
#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct TypstConfig {
    /// Font discovery settings.
    pub fonts: FontConfig,
}

/// Font discovery settings for Typst compilation.
#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct FontConfig {
    /// Additional project-relative font directories.
    pub paths: EcoVec<EcoString>,
    /// Whether fonts installed on the system are discovered.
    pub system: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            paths: EcoVec::new(),
            system: true,
        }
    }
}

/// Aster-owned configuration extracted from the project manifest.
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct AsterConfig {
    /// Project directory layout.
    pub paths: PathsConfig,
    /// Generated-output settings.
    pub output: OutputConfig,
    /// Resource processing settings.
    pub assets: AssetsConfig,
    /// CSS transformation settings.
    pub css: CssConfig,
    /// Additional development watcher inputs.
    pub watch: WatchConfig,
    /// Typst compiler settings.
    pub typst: TypstConfig,
    /// Syntax-highlighting settings.
    pub highlight: HighlightConfig,
}

impl AsterConfig {
    /// Parse Aster-owned settings from `aster.toml`.
    pub fn parse(content: &[u8], path: &Path) -> Result<Self> {
        let content = std::str::from_utf8(content)
            .with_context(|| format!("{} is not valid UTF-8", path.display()))?;
        let table: toml::Table = content
            .parse()
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Self::deserialize(toml::Value::Table(table))
            .with_context(|| format!("failed to extract Aster config from {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(path: &Path) -> Result<AsterConfig> {
        let content = std::fs::read(path)?;
        AsterConfig::parse(&content, path)
    }

    #[test]
    fn loads_aster_config_and_ignores_project_fields() {
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

        let config = load(&config_file).unwrap();

        assert_eq!(config.highlight.themes.light, "Solarized (light)");
        assert_eq!(config.highlight.themes.dark, "Solarized (dark)");
        assert_eq!(
            config
                .css
                .targets
                .iter()
                .map(EcoString::as_str)
                .collect::<Vec<_>>(),
            ["defaults"]
        );
        assert!(config.watch.paths.is_empty());
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

        let config = load(&config_file).unwrap();

        assert_eq!(config.highlight.themes.light, "Solarized (light)");
        assert_eq!(config.highlight.themes.dark, "base16-eighties.dark");
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
                "[watch]\n",
                "paths = [\"components\", \"plugins/theme.ts\"]\n",
                "[typst.fonts]\n",
                "paths = [\"fonts\"]\n",
                "system = false\n",
                "[highlight]\n",
                "enabled = false\n",
            ),
        )
        .unwrap();

        let config = load(&config_file).unwrap();

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
        assert_eq!(
            config
                .watch
                .paths
                .iter()
                .map(EcoString::as_str)
                .collect::<Vec<_>>(),
            ["components", "plugins/theme.ts"]
        );
        assert_eq!(config.typst.fonts.paths, eco_vec!["fonts".into()]);
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

    #[test]
    fn rejects_removed_tailwind_setting() {
        let temp = tempfile::tempdir().unwrap();
        let config_file = temp.path().join("aster.toml");
        std::fs::write(&config_file, "[css]\ntailwind = true\n").unwrap();

        let error = load(&config_file)
            .err()
            .expect("removed Tailwind setting must be rejected");

        assert!(
            format!("{error:#}").contains("unknown field `tailwind`"),
            "unexpected error: {error:#}"
        );
    }
}
