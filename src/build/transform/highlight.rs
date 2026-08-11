use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;

use ::typst::ecow::{EcoString, EcoVec, eco_format, eco_vec};
use ::typst::foundations::Bytes;
use ::typst::syntax::{Span, VirtualPath};
use anyhow::Result;
use comemo::Tracked;
use lumis_core::events::HighlightEvent;
use lumis_core::highlights::HIGHLIGHT_NAMES;
use lumis_core::themes::{self, Style, TextDecoration, Theme, UnderlineStyle};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::BuildWarning;
use crate::build::files::{FileAccessError, ProjectFiles};
use crate::build::output::PagePublication;
use crate::foundation::config::HighlightConfig;

use super::dom::{HtmlElementExt, append_to_head};
use super::{Processor, WalkControl};

mod language;
mod typst;

use self::language::LanguageRegistry;

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

type HighlightToken = (EcoString, EcoString, EcoString);

#[derive(Clone, Debug, Eq, PartialEq)]
struct HighlightStyle {
    light: Option<Style>,
    dark: Option<Style>,
}

impl HighlightStyle {
    fn resolve(light: &Theme, dark: &Theme, language: &str, scope: &str) -> Self {
        Self {
            light: resolve_theme_style(light, language, scope),
            dark: resolve_theme_style(dark, language, scope),
        }
    }

    fn is_plain(&self) -> bool {
        [self.light.as_ref(), self.dark.as_ref()]
            .into_iter()
            .flatten()
            .all(|style| style == &Style::default())
    }
}

fn resolve_theme_style(theme: &Theme, language: &str, scope: &str) -> Option<Style> {
    if scope.is_empty() {
        return None;
    }
    let specialized = eco_format!("{scope}.{language}");
    theme.get_style(&specialized).cloned()
}

pub struct HighlightProcessor {
    themes: Option<(Theme, Theme)>,
    styles: Vec<HighlightStyle>,
    languages: Option<LanguageRegistry>,
    dynamic_tokens: HashMap<(EcoString, EcoString), EcoVec<HighlightToken>>,
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
                    languages: None,
                    dynamic_tokens: HashMap::new(),
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
                languages: None,
                dynamic_tokens: HashMap::new(),
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
        if self.themes.is_none() {
            return Ok(WalkControl::Continue);
        }
        if !element.is_tag(typst_html::tag::code) {
            return Ok(WalkControl::Continue);
        }
        let Some(lang) = element.get_attr("data-lang") else {
            return Ok(WalkControl::Continue);
        };
        let raw = element.inner_text();
        if raw.is_empty() {
            return Ok(WalkControl::SkipChildren);
        }

        // External languages are deliberately kept outside comemo: loading a
        // parser mutates the local registry and may touch the network. The
        // build-scoped map still avoids repeating the same block in one build.
        let tokens = self.highlight_tokens(&raw, &lang)?;
        let (light, dark) = self.themes.as_ref().expect("checked above");
        Ok(process_tokens(
            element,
            light,
            dark,
            &mut self.styles,
            tokens,
        ))
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

impl HighlightProcessor {
    fn highlight_tokens(&mut self, code: &str, lang: &str) -> Result<EcoVec<HighlightToken>> {
        if let Some(tokens) = typst::highlight(code, lang) {
            return Ok(tokens);
        }

        let key = (lang.into(), code.into());
        if let Some(tokens) = self.dynamic_tokens.get(&key) {
            return Ok(tokens.clone());
        }

        let operation =
            tracing::debug_span!(target: "aster::build", "highlight", detail = %lang).entered();
        if self.languages.is_none() {
            self.languages = Some(LanguageRegistry::new()?);
        }
        let tokens = match self
            .languages
            .as_ref()
            .expect("language registry initialized")
            .highlight(code, lang)?
        {
            Some(events) => events_to_tokens(code, lang, events),
            None => eco_vec![(lang.into(), EcoString::new(), code.into())],
        };
        drop(operation);
        self.dynamic_tokens.insert(key, tokens.clone());
        Ok(tokens)
    }
}

fn process_tokens(
    element: &mut HtmlElement,
    light: &Theme,
    dark: &Theme,
    styles: &mut Vec<HighlightStyle>,
    tokens: EcoVec<HighlightToken>,
) -> WalkControl {
    let mut children = EcoVec::new();
    for (language, scope, text) in tokens {
        let mut span = HtmlElement::new(typst_html::tag::span)
            .with_children(eco_vec![HtmlNode::Text(text, Span::detached())]);
        let style = HighlightStyle::resolve(light, dark, &language, &scope);
        if !style.is_plain() {
            let index = styles
                .iter()
                .position(|candidate| *candidate == style)
                .unwrap_or_else(|| {
                    styles.push(style);
                    styles.len() - 1
                });
            let class = eco_format!("hl-s{index}");
            span = span.with_attr(typst_html::attr::class, class);
        }
        children.push(HtmlNode::Element(span));
    }
    element.children = children;
    WalkControl::SkipChildren
}

fn attach_stylesheet(document: &mut HtmlDocument, url: EcoString) {
    let link = HtmlElement::new(typst_html::tag::link)
        .with_attr(typst_html::attr::rel, "stylesheet")
        .with_attr(typst_html::attr::href, url);
    append_to_head(document, link);
}

/// Convert Lumis source ranges into tokens carrying their effective semantic scope.
fn events_to_tokens(
    code: &str,
    root_language: &str,
    events: Vec<HighlightEvent>,
) -> EcoVec<HighlightToken> {
    let mut scopes = vec![(EcoString::from(root_language), EcoString::new())];
    let mut out = EcoVec::new();

    for event in events {
        match event {
            HighlightEvent::Start {
                scope_index,
                language,
            } => {
                let scope = HIGHLIGHT_NAMES.get(scope_index).copied().unwrap_or("");
                scopes.push((language.into(), scope.into()));
            }
            HighlightEvent::End => {
                if scopes.len() > 1 {
                    scopes.pop();
                }
            }
            HighlightEvent::Source { start, end } => {
                if let Some(text) = code.get(start..end)
                    && !text.is_empty()
                {
                    let (language, scope) = scopes.last().expect("root scope is always present");
                    out.push((language.clone(), scope.clone(), text.into()));
                }
            }
        }
    }
    out
}

/// Load a Lumis theme by built-in name or project-root-relative JSON path.
fn load_theme(
    name_or_path: &str,
    project_files: Tracked<ProjectFiles>,
) -> std::result::Result<Theme, ThemeError> {
    if let Ok(theme) = themes::get(name_or_path) {
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
    themes::from_json(source).map_err(|error| ThemeError::Load {
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
        write_theme_rule(&mut css, "", index, style.light.as_ref(), style);
    }

    if styles.iter().any(|style| style.light != style.dark) {
        css.push_str("@media(prefers-color-scheme:dark){");
        for (index, style) in styles.iter().enumerate() {
            if style.light != style.dark {
                write_theme_rule(&mut css, "", index, style.dark.as_ref(), style);
            }
        }
        css.push_str("}\n");

        for (index, style) in styles.iter().enumerate() {
            if style.light != style.dark {
                write_theme_rule(
                    &mut css,
                    "[data-theme=\"light\"] ",
                    index,
                    style.light.as_ref(),
                    style,
                );
                write_theme_rule(
                    &mut css,
                    "[data-theme=\"dark\"] ",
                    index,
                    style.dark.as_ref(),
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
    current: Option<&Style>,
    pair: &HighlightStyle,
) {
    let _ = write!(css, "{selector_prefix}.hl-s{index}{{");
    let mut has_property = false;

    if pair.light.as_ref().is_some_and(|style| style.fg.is_some())
        || pair.dark.as_ref().is_some_and(|style| style.fg.is_some())
    {
        write_property(
            css,
            &mut has_property,
            "color",
            current
                .and_then(|style| style.fg.as_deref())
                .unwrap_or("inherit"),
        );
    }
    if pair.light.as_ref().is_some_and(|style| style.bg.is_some())
        || pair.dark.as_ref().is_some_and(|style| style.bg.is_some())
    {
        write_property(
            css,
            &mut has_property,
            "background-color",
            current
                .and_then(|style| style.bg.as_deref())
                .unwrap_or("transparent"),
        );
    }
    if pair.light.as_ref().is_some_and(|style| style.bold)
        || pair.dark.as_ref().is_some_and(|style| style.bold)
    {
        write_property(
            css,
            &mut has_property,
            "font-weight",
            if current.is_some_and(|style| style.bold) {
                "bold"
            } else {
                "normal"
            },
        );
    }
    if pair.light.as_ref().is_some_and(|style| style.italic)
        || pair.dark.as_ref().is_some_and(|style| style.italic)
    {
        write_property(
            css,
            &mut has_property,
            "font-style",
            if current.is_some_and(|style| style.italic) {
                "italic"
            } else {
                "normal"
            },
        );
    }

    let light_decoration = pair
        .light
        .as_ref()
        .map_or(TextDecoration::default(), |style| style.text_decoration);
    let dark_decoration = pair
        .dark
        .as_ref()
        .map_or(TextDecoration::default(), |style| style.text_decoration);
    let current_decoration =
        current.map_or(TextDecoration::default(), |style| style.text_decoration);
    let uses_decoration = light_decoration != TextDecoration::default()
        || dark_decoration != TextDecoration::default();
    if uses_decoration {
        write_property(
            css,
            &mut has_property,
            "text-decoration-line",
            decoration_line(current_decoration),
        );
    }
    if light_decoration.underline != UnderlineStyle::None
        || dark_decoration.underline != UnderlineStyle::None
    {
        write_property(
            css,
            &mut has_property,
            "text-decoration-style",
            underline_style(current_decoration.underline),
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
    fn theme_css_resets_styles_between_modes() {
        let style = HighlightStyle {
            light: Some(Style {
                fg: Some("#112233".into()),
                bold: true,
                italic: true,
                text_decoration: TextDecoration {
                    underline: UnderlineStyle::Wavy,
                    strikethrough: true,
                },
                ..Style::default()
            }),
            dark: Some(Style {
                bg: Some("#445566".into()),
                ..Style::default()
            }),
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
        let json = "{\n  \"a\": 1,\n  \"b\": 2\n}\n";

        let events = vec![HighlightEvent::Source {
            start: 0,
            end: json.len(),
        }];
        assert_eq!(
            highlighted_text(&events_to_tokens(json, "json", events)),
            json
        );
    }

    #[test]
    fn preserves_lumis_scope_and_injected_language() {
        assert!(themes::get("github_light").is_ok());

        let events = vec![
            HighlightEvent::Start {
                scope_index: HIGHLIGHT_NAMES
                    .iter()
                    .position(|name| *name == "keyword")
                    .unwrap(),
                language: "toml".into(),
            },
            HighlightEvent::Source { start: 0, end: 4 },
            HighlightEvent::End,
        ];
        let tokens = events_to_tokens("name", "markdown", events);
        assert_eq!(highlighted_text(&tokens), "name");
        assert_eq!(tokens[0].0, "toml");
        assert_eq!(tokens[0].1, "keyword");
    }

    #[test]
    fn lumis_themes_resolve_language_specific_then_generic_scopes() {
        let theme = themes::from_json(
            r##"{
                "name": "test",
                "appearance": "light",
                "revision": "test",
                "highlights": {
                    "keyword": { "fg": "#112233" },
                    "keyword.rust": { "fg": "#445566" }
                }
            }"##,
        )
        .unwrap();

        assert_eq!(
            resolve_theme_style(&theme, "rust", "keyword").unwrap().fg,
            Some("#445566".to_owned())
        );
        assert_eq!(
            resolve_theme_style(&theme, "toml", "keyword").unwrap().fg,
            Some("#112233".to_owned())
        );
    }
}
