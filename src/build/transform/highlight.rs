use std::fmt::Write;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use anyhow::Result;
use comemo::Tracked;
use syntect::easy::ScopeRegionIterator;
use syntect::highlighting::{Highlighter, Theme, ThemeSet};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};
use typst::ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst::foundations::Bytes;
use typst::syntax::{LinkedNode, Span, SyntaxNode, VirtualPath, parse_code, parse_math};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::BuildWarning;
use crate::build::output::{AssetPath, OutputPublication, PagePublication};
use crate::foundation::config::HighlightConfig;
use crate::foundation::files::{FileAccessError, ProjectFiles};

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

type HighlightToken = (Option<EcoString>, EcoString);

pub(crate) struct HighlightProcessor {
    enabled: bool,
    stylesheet: Option<AssetPath>,
}

impl HighlightProcessor {
    pub fn new(
        config: &HighlightConfig,
        project_files: Tracked<ProjectFiles>,
        publication: &mut OutputPublication,
    ) -> Result<(Self, Option<BuildWarning>)> {
        if !config.enabled {
            return Ok((
                Self {
                    enabled: false,
                    stylesheet: None,
                },
                None,
            ));
        }
        let (stylesheet, warning) = match compute_highlight_css(config, project_files) {
            Ok(Some(css)) => (Some(publication.add_highlight_stylesheet(css)?), None),
            Ok(None) => (None, None),
            Err(error) => {
                let warning =
                    BuildWarning::new(eco_format!("failed to resolve highlight CSS: {error:#}"));
                (None, Some(warning))
            }
        };
        Ok((
            Self {
                enabled: true,
                stylesheet,
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
        if !self.enabled {
            return Ok(WalkControl::Continue);
        }
        Ok(process_element(element))
    }

    fn end_document(
        &mut self,
        document: &mut HtmlDocument,
        page: &mut PagePublication<'_>,
    ) -> Result<()> {
        if let Some(stylesheet) = &self.stylesheet {
            let url = page.reference(stylesheet)?;
            attach_stylesheet(document, url);
        }
        Ok(())
    }
}

fn process_element(element: &mut HtmlElement) -> WalkControl {
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
    let mut children = EcoVec::new();
    for (class, text) in tokens {
        let mut span = HtmlElement::new(typst_html::tag::span)
            .with_children(eco_vec![HtmlNode::Text(text, Span::detached())]);
        if let Some(class) = class {
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

/// Derive a semantic CSS variable suffix from a slice of scopes.
fn scope_css_name(scopes: &[Scope]) -> Option<EcoString> {
    for scope in scopes.iter().rev() {
        let scope = eco_format!("{scope}");
        let mut parts = scope.split('.');
        let (Some(kind), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        if kind != "source" && kind != "meta" {
            return Some(eco_format!("{kind}-{name}"));
        }
    }
    None
}

fn scope_class(scopes: &[Scope]) -> Option<EcoString> {
    scope_css_name(scopes).map(|name| eco_format!("hl-{name}"))
}

#[comemo::memoize]
fn highlight_tokens(code: &str, lang: &str) -> EcoVec<HighlightToken> {
    if TYPST_LANGS.contains(&lang) {
        do_typst_highlight(code, lang)
    } else {
        do_syntect_highlight(code, lang)
    }
}

/// Highlight code using syntect (all non-Typst languages).
///
/// This is **theme-independent** — it only derives CSS class names from
/// the scope stack produced by the syntax parser.  All colour / font
/// decisions live in the generated CSS.
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

            out.push((scope_class(scope_stack.as_slice()), EcoString::from(text)));
        }

        out.push((None, "\n".into()));
    }

    out
}

/// Highlight code using Typst's native AST (languages: typ, typst, typc, typm).
///
/// Theme-independent — scope-based class names only, no colour resolution.
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
    for (class, text) in native_tokens {
        for (i, segment) in text.split('\n').enumerate() {
            if i > 0 {
                out.push((None, "\n".into()));
            }
            if !segment.is_empty() {
                out.push((class.clone(), EcoString::from(segment)));
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
            tokens.push((scope_class(scopes.as_slice()), EcoString::from(text)));
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

// ---------------------------------------------------------------------------
// Theme resolution for CSS variable generation
// ---------------------------------------------------------------------------

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

/// Resolve highlight theme colours and return the CSS content.
///
/// Returns `None` when no scopes need highlighting. Asset identity and naming
/// are owned by the output publication module.
fn compute_highlight_css(
    config: &HighlightConfig,
    project_files: Tracked<ProjectFiles>,
) -> Result<Option<Bytes>> {
    compute_highlight_css_impl(
        config.themes.light.as_str(),
        config.themes.dark.as_str(),
        project_files,
    )
    .map_err(anyhow::Error::msg)
}

#[comemo::memoize]
fn compute_highlight_css_impl(
    light_theme: &str,
    dark_theme: &str,
    project_files: Tracked<ProjectFiles>,
) -> std::result::Result<Option<Bytes>, ThemeError> {
    let light = load_theme(light_theme, project_files)?;
    let dark = load_theme(dark_theme, project_files)?;
    let light_h = Highlighter::new(&light);
    let dark_h = Highlighter::new(&dark);

    let default_dark = crate::build::transform::dom::color_to_hex(
        dark.settings
            .foreground
            .unwrap_or(syntect::highlighting::Color::BLACK),
    );

    // Collect unique scope names from theme selectors, resolve colours + font style.
    let mut vars: Vec<(EcoString, EcoString, u8, EcoString, u8)> = Vec::new();
    for theme in [&light, &dark] {
        for scope_entry in &theme.scopes {
            for single in &scope_entry.scope.selectors {
                let Some(scope) = single.extract_single_scope() else {
                    continue;
                };

                let Some(name) = scope_css_name(std::slice::from_ref(&scope)) else {
                    continue;
                };
                // Deduplicate by scope name.
                if vars.iter().any(|(n, _, _, _, _)| *n == name) {
                    continue;
                }
                let light_st = light_h.style_for_stack(std::slice::from_ref(&scope));
                let dark_st = dark_h.style_for_stack(std::slice::from_ref(&scope));
                vars.push((
                    name,
                    crate::build::transform::dom::color_to_hex(light_st.foreground),
                    light_st.font_style.bits(),
                    crate::build::transform::dom::color_to_hex(dark_st.foreground),
                    dark_st.font_style.bits(),
                ));
            }
        }
    }

    if vars.is_empty() {
        return Ok(None);
    }

    fn font_style(bits: u8) -> &'static str {
        match bits & 3 {
            1 => ";font-weight:bold",
            2 => ";font-style:italic",
            3 => ";font-weight:bold;font-style:italic",
            _ => "",
        }
    }

    // Generate CSS, one class per scope, with prefers-color-scheme + data-theme cascade:
    //   1. default = light
    //   2. @media(prefers-color-scheme:dark) — OS-level dark preference
    //   3. [data-theme="dark"] — explicit override
    //   4. [data-theme="light"] — explicit override (wins over OS dark)
    let mut css = String::new();
    for (name, lc, lb, _, _) in &vars {
        let _ = writeln!(css, ".hl-{name}{{color:{lc}{}}}", font_style(*lb));
    }
    for (name, _, _, dc, db) in &vars {
        if *dc != default_dark || *db != 0 {
            let _ = writeln!(
                css,
                "@media(prefers-color-scheme:dark){{.hl-{name}{{color:{dc}{}}}}}",
                font_style(*db),
            );
        }
    }
    for (name, lc, lb, _, _) in &vars {
        let _ = writeln!(
            css,
            "[data-theme=\"light\"] .hl-{name}{{color:{lc}{}}}",
            font_style(*lb)
        );
    }
    for (name, _, _, dc, db) in &vars {
        let _ = writeln!(
            css,
            "[data-theme=\"dark\"] .hl-{name}{{color:{dc}{}}}",
            font_style(*db),
        );
    }

    Ok(Some(Bytes::from_string(css)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted_text(tokens: &[HighlightToken]) -> String {
        tokens.iter().map(|(_, text)| text.as_str()).collect()
    }

    #[test]
    fn scopes_without_semantic_style_have_no_class() {
        assert_eq!(scope_css_name(&[]), None);

        let source = Scope::new("source.rust").unwrap();
        assert_eq!(scope_css_name(std::slice::from_ref(&source)), None);

        let keyword = Scope::new("keyword.control").unwrap();
        assert_eq!(
            scope_css_name(std::slice::from_ref(&keyword)).as_deref(),
            Some("keyword-control")
        );
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
