use std::fmt::Write;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use anyhow::Result;
use comemo::Tracked;
use syntect::easy::ScopeRegionIterator;
use syntect::highlighting::{Color, FontStyle, Highlighter, StyleModifier, Theme, ThemeSet};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};
use typst::ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst::foundations::Bytes;
use typst::syntax::{LinkedNode, Span, SyntaxNode, VirtualPath, parse_code, parse_math};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::BuildWarning;
use crate::build::files::{FileAccessError, ProjectFiles};
use crate::build::output::PagePublication;
use crate::foundation::config::HighlightConfig;

use super::{Processor, WalkControl};
use crate::build::transform::dom::{HtmlElementExt, append_to_head};

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

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Languages that Typst can parse with its own AST.
const TYPST_LANGS: &[&str] = &["typ", "typst", "typc", "typm"];

type HighlightToken = (EcoVec<Scope>, EcoString);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ThemeStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    font_style: FontStyle,
}

impl From<StyleModifier> for ThemeStyle {
    fn from(style: StyleModifier) -> Self {
        Self {
            foreground: style.foreground,
            background: style.background,
            font_style: style.font_style.unwrap_or_default(),
        }
    }
}

impl ThemeStyle {
    fn is_plain(self) -> bool {
        self.foreground.is_none() && self.background.is_none() && self.font_style.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HighlightStyle {
    light: ThemeStyle,
    dark: ThemeStyle,
}

impl HighlightStyle {
    fn resolve(light: &Highlighter<'_>, dark: &Highlighter<'_>, scopes: &[Scope]) -> Self {
        Self {
            light: light.style_mod_for_stack(scopes).into(),
            dark: dark.style_mod_for_stack(scopes).into(),
        }
    }

    fn is_plain(self) -> bool {
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
        Ok(process_element(element, light, dark, &mut self.styles))
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
) -> WalkControl {
    if !element.is_tag(typst_html::tag::code) {
        return WalkControl::Continue;
    }

    let Some(lang) = element.get_attr("data-lang") else {
        return WalkControl::Continue;
    };
    let raw = element.inner_text();
    if raw.is_empty() {
        return WalkControl::SkipChildren;
    }

    let tokens = highlight_tokens(&raw, &lang);
    let light = Highlighter::new(light);
    let dark = Highlighter::new(dark);
    let mut children = EcoVec::new();
    for (scopes, text) in tokens {
        let mut span = HtmlElement::new(typst_html::tag::span)
            .with_children(eco_vec![HtmlNode::Text(text, Span::detached())]);
        let style = HighlightStyle::resolve(&light, &dark, &scopes);
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

#[comemo::memoize]
fn highlight_tokens(code: &str, lang: &str) -> EcoVec<HighlightToken> {
    if TYPST_LANGS.contains(&lang) {
        do_typst_highlight(code, lang)
    } else {
        do_syntect_highlight(code, lang)
    }
}

/// Parse non-Typst code into its complete TextMate scope stacks.
fn do_syntect_highlight(code: &str, lang: &str) -> EcoVec<HighlightToken> {
    let syntax = SS
        .find_syntax_by_token(lang)
        .or_else(|| SS.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SS.find_syntax_plain_text());

    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let mut out = EcoVec::new();

    for line in code.lines() {
        let Ok(ops) = parse_state.parse_line(line, &SS) else {
            continue;
        };

        for (text, op) in ScopeRegionIterator::new(&ops, line) {
            let _ = scope_stack.apply(op);
            if text.is_empty() {
                continue;
            }

            out.push((
                scope_stack.as_slice().iter().copied().collect(),
                text.into(),
            ));
        }

        out.push((EcoVec::new(), "\n".into()));
    }

    out
}

/// Parse Typst code into TextMate-compatible scope stacks from its native AST.
fn do_typst_highlight(code: &str, lang: &str) -> EcoVec<HighlightToken> {
    let root: SyntaxNode = match lang {
        "typc" => parse_code(code),
        "typm" => parse_math(code),
        _ => typst::syntax::parse(code),
    };

    let mut native_tokens = EcoVec::new();
    let mut scopes = EcoVec::new();

    walk_typst_node(
        code,
        &LinkedNode::new(&root),
        &mut scopes,
        &mut native_tokens,
    );

    let mut out = EcoVec::new();
    for (scopes, text) in native_tokens {
        for (i, segment) in text.split('\n').enumerate() {
            if i > 0 {
                out.push((EcoVec::new(), "\n".into()));
            }
            if !segment.is_empty() {
                out.push((scopes.clone(), segment.into()));
            }
        }
    }
    out
}

/// Walk Typst's AST collecting scope-and-text pairs (theme‑independent).
fn walk_typst_node<'a>(
    code: &str,
    node: &LinkedNode<'a>,
    scopes: &mut EcoVec<Scope>,
    tokens: &mut EcoVec<HighlightToken>,
) {
    if node.children().len() == 0 {
        let text = &code[node.range()];
        if !text.is_empty() {
            tokens.push((scopes.clone(), text.into()));
        }
        return;
    }

    for child in node.children() {
        let mut child_scopes = scopes.clone();
        if let Some(tag) = typst::syntax::highlight(&child)
            && let Ok(s) = Scope::new(tag.tm_scope())
        {
            child_scopes.push(s);
        }
        std::mem::swap(&mut child_scopes, scopes);
        walk_typst_node(code, &child, scopes, tokens);
        std::mem::swap(&mut child_scopes, scopes);
    }
}

/// Load a syntect theme by built-in name or project-root-relative virtual path.
fn load_theme(
    name_or_path: &str,
    project_files: Tracked<ProjectFiles>,
) -> std::result::Result<Theme, ThemeError> {
    if let Some(theme) = THEMES.themes.get(name_or_path) {
        return Ok(theme.clone());
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
    let mut reader = std::io::Cursor::new(bytes);
    let theme = ThemeSet::load_from_reader(&mut reader).map_err(|error| ThemeError::Load {
        path: path.get_with_slash().into(),
        inner: Arc::new(anyhow::Error::new(error)),
    })?;
    Ok(theme)
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
    for (index, style) in styles.iter().copied().enumerate() {
        write_theme_rule(&mut css, "", index, style.light, style);
    }

    css.push_str("@media(prefers-color-scheme:dark){");
    for (index, style) in styles.iter().copied().enumerate() {
        if style.light != style.dark {
            write_theme_rule(&mut css, "", index, style.dark, style);
        }
    }
    css.push_str("}\n");

    for (index, style) in styles.iter().copied().enumerate() {
        if style.light != style.dark {
            write_theme_rule(
                &mut css,
                "[data-theme=\"light\"] ",
                index,
                style.light,
                style,
            );
            write_theme_rule(&mut css, "[data-theme=\"dark\"] ", index, style.dark, style);
        }
    }

    Bytes::from_string(css)
}

fn write_theme_rule(
    css: &mut String,
    selector_prefix: &str,
    index: usize,
    current: ThemeStyle,
    pair: HighlightStyle,
) {
    let _ = write!(css, "{selector_prefix}.hl-s{index}{{");
    let mut has_property = false;

    if pair.light.foreground.is_some() || pair.dark.foreground.is_some() {
        write_property_separator(css, &mut has_property);
        if let Some(color) = current.foreground {
            let _ = write!(
                css,
                "color:{}",
                crate::build::transform::dom::color_to_hex(color)
            );
        } else {
            css.push_str("color:inherit");
        }
    }
    if pair.light.background.is_some() || pair.dark.background.is_some() {
        write_property_separator(css, &mut has_property);
        if let Some(color) = current.background {
            let _ = write!(
                css,
                "background-color:{}",
                crate::build::transform::dom::color_to_hex(color)
            );
        } else {
            css.push_str("background-color:transparent");
        }
    }
    write_font_style(
        css,
        &mut has_property,
        "font-weight",
        current.font_style.contains(FontStyle::BOLD),
        pair.light.font_style.contains(FontStyle::BOLD)
            || pair.dark.font_style.contains(FontStyle::BOLD),
        "bold",
        "normal",
    );
    write_font_style(
        css,
        &mut has_property,
        "font-style",
        current.font_style.contains(FontStyle::ITALIC),
        pair.light.font_style.contains(FontStyle::ITALIC)
            || pair.dark.font_style.contains(FontStyle::ITALIC),
        "italic",
        "normal",
    );
    write_font_style(
        css,
        &mut has_property,
        "text-decoration",
        current.font_style.contains(FontStyle::UNDERLINE),
        pair.light.font_style.contains(FontStyle::UNDERLINE)
            || pair.dark.font_style.contains(FontStyle::UNDERLINE),
        "underline",
        "none",
    );
    css.push_str("}\n");
}

fn write_font_style(
    css: &mut String,
    has_property: &mut bool,
    property: &str,
    enabled: bool,
    used: bool,
    value: &str,
    reset: &str,
) {
    if used {
        write_property_separator(css, has_property);
        let _ = write!(css, "{property}:{}", if enabled { value } else { reset });
    }
}

fn write_property_separator(css: &mut String, has_property: &mut bool) {
    if *has_property {
        css.push(';');
    } else {
        *has_property = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted_text(tokens: &[HighlightToken]) -> String {
        tokens.iter().map(|(_, text)| text.as_str()).collect()
    }

    #[test]
    fn theme_css_resets_styles_between_modes() {
        let style = HighlightStyle {
            light: ThemeStyle {
                foreground: Some(Color {
                    r: 0x11,
                    g: 0x22,
                    b: 0x33,
                    a: 0xff,
                }),
                background: None,
                font_style: FontStyle::BOLD | FontStyle::ITALIC | FontStyle::UNDERLINE,
            },
            dark: ThemeStyle {
                foreground: None,
                background: Some(Color {
                    r: 0x44,
                    g: 0x55,
                    b: 0x66,
                    a: 0xff,
                }),
                font_style: FontStyle::empty(),
            },
        };

        let css = String::from_utf8(highlight_css(&[style]).to_vec()).unwrap();
        assert!(css.contains(
            ".hl-s0{color:#112233;background-color:transparent;font-weight:bold;font-style:italic;text-decoration:underline}"
        ));
        assert!(css.contains(
            "[data-theme=\"dark\"] .hl-s0{color:inherit;background-color:#445566;font-weight:normal;font-style:normal;text-decoration:none}"
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
            assert_eq!(highlighted_text(&do_typst_highlight(code, "typc")), code);
        }
        assert_eq!(highlighted_text(&do_syntect_highlight(json, "json")), json);
    }
}
