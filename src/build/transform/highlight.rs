use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use comemo::Tracked;
use lumis::highlight::{HighlightError, highlight_iter};
use lumis::languages::Language;
use lumis::themes::{Style, TextDecoration, Theme, UnderlineStyle};
use typst::ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst::foundations::Bytes;
use typst::syntax::{Span, VirtualPath};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::BuildWarning;
use crate::build::files::{FileAccessError, ProjectFiles};
use crate::build::output::PagePublication;
use crate::foundation::config::HighlightConfig;

use super::dom::{HtmlElementExt, append_to_head};
use super::{Processor, WalkControl};

/// A cheaply cloneable theme-loading error at the memoization seam.
#[derive(Debug, Clone, thiserror::Error)]
enum ThemeError {
    #[error("failed to load theme from {path}: {inner}")]
    Load {
        path: EcoString,
        #[source]
        inner: Arc<anyhow::Error>,
    },
    #[error("invalid theme path {path}: {message}")]
    InvalidPath { path: EcoString, message: EcoString },
    #[error(transparent)]
    File(#[from] FileAccessError),
}

type HighlightToken = (Language, &'static str, EcoString);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ThemeStyle {
    foreground: Option<EcoString>,
    background: Option<EcoString>,
    bold: bool,
    italic: bool,
    decoration: TextDecoration,
}

impl From<Option<&Style>> for ThemeStyle {
    fn from(style: Option<&Style>) -> Self {
        let Some(style) = style else {
            return Self::default();
        };
        Self {
            foreground: style.fg.as_deref().map(Into::into),
            background: style.bg.as_deref().map(Into::into),
            bold: style.bold,
            italic: style.italic,
            decoration: style.text_decoration,
        }
    }
}

impl ThemeStyle {
    fn resolve(theme: &Theme, language: Language, scope: &str) -> Self {
        if scope.is_empty() {
            return Self::default();
        }
        let specialized = eco_format!("{scope}.{}", language.id_name());
        theme.get_style(&specialized).into()
    }

    fn is_plain(&self) -> bool {
        self.foreground.is_none()
            && self.background.is_none()
            && !self.bold
            && !self.italic
            && self.decoration == TextDecoration::default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HighlightStyle {
    light: ThemeStyle,
    dark: ThemeStyle,
}

impl HighlightStyle {
    fn resolve(light: &Theme, dark: &Theme, language: Language, scope: &str) -> Self {
        Self {
            light: ThemeStyle::resolve(light, language, scope),
            dark: ThemeStyle::resolve(dark, language, scope),
        }
    }

    fn is_plain(&self) -> bool {
        self.light.is_plain() && self.dark.is_plain()
    }
}

pub struct HighlightProcessor {
    themes: Option<(Theme, Theme)>,
    styles: Vec<HighlightStyle>,
}

impl HighlightProcessor {
    pub fn new(
        config: &HighlightConfig,
        project_files: Tracked<ProjectFiles>,
    ) -> Result<(Self, Option<BuildWarning>)> {
        if !config.enabled {
            return Ok((
                Self {
                    themes: None,
                    styles: Vec::new(),
                },
                None,
            ));
        }

        let (themes, warning) = match load_themes(
            config.themes.light.as_str(),
            config.themes.dark.as_str(),
            project_files,
        ) {
            Ok(themes) => (Some(themes), None),
            Err(error) => {
                let warning =
                    BuildWarning::new(eco_format!("failed to resolve highlight CSS: {error:#}"));
                (None, Some(warning))
            }
        };
        Ok((
            Self {
                themes,
                styles: Vec::new(),
            },
            warning,
        ))
    }
}

impl Processor for HighlightProcessor {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        _page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        let Some((light, dark)) = &self.themes else {
            return Ok(WalkControl::Continue);
        };
        process_element(element, light, dark, &mut self.styles)
    }

    fn end_document(
        &mut self,
        document: &mut HtmlDocument,
        page: &mut PagePublication<'_>,
    ) -> Result<()> {
        if !self.styles.is_empty() {
            let styles = std::mem::take(&mut self.styles);
            let url = page.add_highlight_stylesheet(highlight_css(&styles))?;
            attach_stylesheet(document, url);
        }
        Ok(())
    }
}

fn process_element(
    element: &mut HtmlElement,
    light: &Theme,
    dark: &Theme,
    styles: &mut Vec<HighlightStyle>,
) -> Result<WalkControl> {
    if !element.is_tag(typst_html::tag::code) {
        return Ok(WalkControl::Continue);
    }

    let Some(language) = element.get_attr("data-lang") else {
        return Ok(WalkControl::Continue);
    };
    let source = element.inner_text();
    if source.is_empty() {
        return Ok(WalkControl::SkipChildren);
    }

    let mut children = EcoVec::new();
    for (language, scope, text) in highlight_tokens(&source, &language)? {
        let mut span = HtmlElement::new(typst_html::tag::span)
            .with_children(eco_vec![HtmlNode::Text(text, Span::detached())]);
        let style = HighlightStyle::resolve(light, dark, language, scope);
        if !style.is_plain() {
            let index = styles
                .iter()
                .position(|candidate| *candidate == style)
                .unwrap_or_else(|| {
                    styles.push(style);
                    styles.len() - 1
                });
            span = span.with_attr(typst_html::attr::class, eco_format!("hl-s{index}"));
        }
        children.push(HtmlNode::Element(span));
    }
    element.children = children;
    Ok(WalkControl::SkipChildren)
}

fn attach_stylesheet(document: &mut HtmlDocument, url: EcoString) {
    let link = HtmlElement::new(typst_html::tag::link)
        .with_attr(typst_html::attr::rel, "stylesheet")
        .with_attr(typst_html::attr::href, url);
    append_to_head(document, link);
}

#[comemo::memoize]
fn highlight_tokens(
    source: &str,
    language: &str,
) -> std::result::Result<EcoVec<HighlightToken>, HighlightError> {
    let (prefix, suffix, language) = match language {
        "typc" => ("#", "", Language::Typst),
        "typm" => ("$", "$", Language::Typst),
        _ => ("", "", Language::guess(Some(language), source)),
    };
    let wrapped =
        (!prefix.is_empty() || !suffix.is_empty()).then(|| eco_format!("{prefix}{source}{suffix}"));
    let highlighted = wrapped.as_deref().unwrap_or(source);
    let source_range = prefix.len()..highlighted.len() - suffix.len();
    let mut tokens = EcoVec::new();
    highlight_iter(
        highlighted,
        language,
        None,
        |_text, language, range, scope, _style| {
            let start = range.start.max(source_range.start);
            let end = range.end.min(source_range.end);
            if start < end {
                tokens.push((language, scope, highlighted[start..end].into()));
            }
            Ok::<(), std::io::Error>(())
        },
    )?;
    Ok(tokens)
}

/// Load a Lumis theme by built-in name or project-root-relative JSON path.
fn load_theme(
    name_or_path: &str,
    project_files: Tracked<ProjectFiles>,
) -> std::result::Result<Theme, ThemeError> {
    if let Ok(theme) = lumis::themes::get(name_or_path) {
        return Ok(theme);
    }

    if Path::new(name_or_path).is_absolute() {
        return Err(ThemeError::InvalidPath {
            path: name_or_path.into(),
            message: "theme path must be relative to the project root".into(),
        });
    }
    let path = VirtualPath::new(name_or_path).map_err(|error| ThemeError::InvalidPath {
        path: name_or_path.into(),
        message: eco_format!("{error}"),
    })?;
    let bytes = project_files.read(&path)?;
    let source = std::str::from_utf8(bytes.as_slice()).map_err(|error| ThemeError::Load {
        path: path.get_with_slash().into(),
        inner: Arc::new(anyhow::Error::new(error)),
    })?;
    lumis::themes::from_json(source).map_err(|error| ThemeError::Load {
        path: path.get_with_slash().into(),
        inner: Arc::new(anyhow::Error::new(error)),
    })
}

#[comemo::memoize]
fn load_themes(
    light_theme: &str,
    dark_theme: &str,
    project_files: Tracked<ProjectFiles>,
) -> std::result::Result<(Theme, Theme), ThemeError> {
    let light = load_theme(light_theme, project_files)?;
    let dark = load_theme(dark_theme, project_files)?;
    Ok((light, dark))
}

fn highlight_css(styles: &[HighlightStyle]) -> Bytes {
    let mut css = String::new();
    for (index, style) in styles.iter().enumerate() {
        write_theme_rule(&mut css, "", index, &style.light, style);
    }

    if styles.iter().any(|style| style.light != style.dark) {
        css.push_str("@media(prefers-color-scheme:dark){");
        for (index, style) in styles.iter().enumerate() {
            if style.light != style.dark {
                write_theme_rule(&mut css, "", index, &style.dark, style);
            }
        }
        css.push_str("}\n");

        for (index, style) in styles.iter().enumerate() {
            if style.light != style.dark {
                write_theme_rule(
                    &mut css,
                    "[data-theme=\"light\"] ",
                    index,
                    &style.light,
                    style,
                );
                write_theme_rule(
                    &mut css,
                    "[data-theme=\"dark\"] ",
                    index,
                    &style.dark,
                    style,
                );
            }
        }
    }

    Bytes::from_string(css)
}

fn write_theme_rule(
    css: &mut String,
    selector_prefix: &str,
    index: usize,
    current: &ThemeStyle,
    pair: &HighlightStyle,
) {
    let _ = write!(css, "{selector_prefix}.hl-s{index}{{");
    let mut has_property = false;

    if pair.light.foreground.is_some() || pair.dark.foreground.is_some() {
        write_property(
            css,
            &mut has_property,
            "color",
            current.foreground.as_deref().unwrap_or("inherit"),
        );
    }
    if pair.light.background.is_some() || pair.dark.background.is_some() {
        write_property(
            css,
            &mut has_property,
            "background-color",
            current.background.as_deref().unwrap_or("transparent"),
        );
    }
    if pair.light.bold || pair.dark.bold {
        write_property(
            css,
            &mut has_property,
            "font-weight",
            if current.bold { "bold" } else { "normal" },
        );
    }
    if pair.light.italic || pair.dark.italic {
        write_property(
            css,
            &mut has_property,
            "font-style",
            if current.italic { "italic" } else { "normal" },
        );
    }

    let uses_decoration = pair.light.decoration != TextDecoration::default()
        || pair.dark.decoration != TextDecoration::default();
    if uses_decoration {
        write_property(
            css,
            &mut has_property,
            "text-decoration-line",
            decoration_line(current.decoration),
        );
    }
    if pair.light.decoration.underline != UnderlineStyle::None
        || pair.dark.decoration.underline != UnderlineStyle::None
    {
        write_property(
            css,
            &mut has_property,
            "text-decoration-style",
            underline_style(current.decoration.underline),
        );
    }
    css.push_str("}\n");
}

fn decoration_line(decoration: TextDecoration) -> &'static str {
    match (
        decoration.underline != UnderlineStyle::None,
        decoration.strikethrough,
    ) {
        (true, true) => "underline line-through",
        (true, false) => "underline",
        (false, true) => "line-through",
        (false, false) => "none",
    }
}

fn underline_style(underline: UnderlineStyle) -> &'static str {
    match underline {
        UnderlineStyle::None | UnderlineStyle::Solid => "solid",
        UnderlineStyle::Wavy => "wavy",
        UnderlineStyle::Double => "double",
        UnderlineStyle::Dotted => "dotted",
        UnderlineStyle::Dashed => "dashed",
    }
}

fn write_property(css: &mut String, has_property: &mut bool, name: &str, value: &str) {
    if *has_property {
        css.push(';');
    } else {
        *has_property = true;
    }
    let _ = write!(css, "{name}:{value}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted_text(tokens: &[HighlightToken]) -> String {
        tokens.iter().map(|(_, _, text)| text.as_str()).collect()
    }

    #[test]
    fn recognizes_rust_modifiers() {
        let tokens = highlight_tokens("pub fn main() {}", "rust").unwrap();
        assert!(
            tokens
                .iter()
                .any(|(_, scope, text)| *scope == "keyword.modifier" && text == "pub")
        );
    }

    #[test]
    fn theme_css_resets_styles_between_modes() {
        let style = HighlightStyle {
            light: ThemeStyle {
                foreground: Some("#112233".into()),
                background: None,
                bold: true,
                italic: true,
                decoration: TextDecoration {
                    underline: UnderlineStyle::Wavy,
                    strikethrough: true,
                },
            },
            dark: ThemeStyle {
                foreground: None,
                background: Some("#445566".into()),
                ..ThemeStyle::default()
            },
        };

        let css = String::from_utf8(highlight_css(&[style]).to_vec()).unwrap();
        assert!(css.contains(
            ".hl-s0{color:#112233;background-color:transparent;font-weight:bold;font-style:italic;text-decoration-line:underline line-through;text-decoration-style:wavy}"
        ));
        assert!(css.contains(
            "[data-theme=\"dark\"] .hl-s0{color:inherit;background-color:#445566;font-weight:normal;font-style:normal;text-decoration-line:none;text-decoration-style:solid}"
        ));
    }

    #[test]
    fn highlighting_preserves_source_text() {
        let typst = concat!(
            "let protocol = 1\n",
            "let posts = (\n",
            "  blog: (hello-world: (id: \"hello-world\",)),\n",
            ")\n",
        );
        let invalid_typst = concat!(
            "state.protocol = 1\n",
            "collections.blog.\"hello-world\".rendered = (\n",
            "  (kind: \"element\", tag: \"h2\"),\n",
            ")\n",
        );
        let json = "{\n  \"a\": 1,\n  \"b\": 2\n}\n";

        for code in [typst, invalid_typst] {
            assert_eq!(
                highlighted_text(&highlight_tokens(code, "typc").unwrap()),
                code
            );
        }
        assert_eq!(
            highlighted_text(&highlight_tokens(json, "json").unwrap()),
            json
        );
    }

    #[test]
    fn highlights_typst_code_and_math_fragments() {
        for (language, source) in [("typc", "let answer = 42"), ("typm", "x^2 + y^2")] {
            let tokens = highlight_tokens(source, language).unwrap();
            assert_eq!(highlighted_text(&tokens), source);
            assert!(tokens.iter().any(|(_, scope, _)| !scope.is_empty()));
        }
    }
}
