use std::sync::LazyLock;

use syntect::easy::ScopeRegionIterator;
use syntect::highlighting::{Highlighter, Theme, ThemeSet};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};
use typst::ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst::syntax::{LinkedNode, Span, SyntaxNode, parse_code, parse_math};
use typst_html::{HtmlElement, HtmlNode};

use super::{ElementProcessor, ProcessingContext, WalkControl};

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Theme used for internal tokenization.
const TOKEN_THEME: &str = "InspiredGitHub";

/// Languages that Typst can parse with its own AST.
const TYPST_LANGS: &[&str] = &["typ", "typst", "typc", "typm"];

/// [`ElementProcessor`] that syntax-highlights `<code data-lang="...">` blocks.
pub(super) struct HighlightProcessor;

impl ElementProcessor for HighlightProcessor {
    fn matches(&self, elem: &HtmlElement) -> bool {
        elem.tag == typst_html::tag::code
            && elem
                .attrs
                .0
                .iter()
                .any(|(a, _v)| *a.resolve() == *"data-lang")
    }

    fn process(
        &self,
        elem: &mut HtmlElement,
        _ctx: &ProcessingContext<'_>,
    ) -> anyhow::Result<WalkControl> {
        let theme = &THEMES.themes[TOKEN_THEME];
        let lang = match elem
            .attrs
            .0
            .iter()
            .find(|(a, _)| *a.resolve() == *"data-lang")
            .map(|(_, v)| v.clone())
        {
            Some(l) => l,
            None => return Ok(WalkControl::Continue),
        };

        let raw = collect_text(elem);
        if raw.is_empty() {
            return Ok(WalkControl::SkipChildren);
        }

        let tokens = if TYPST_LANGS.contains(&lang.as_str()) {
            do_typst_highlight(&raw, &lang, theme)
        } else {
            do_syntect_highlight(&raw, &lang, theme)
        };

        let mut new_children: EcoVec<HtmlNode> = EcoVec::new();
        for (scope_name, bits, txt) in &tokens {
            let mut style = eco_format!("color:var(--hl-{scope_name})");
            if bits & 1 != 0 {
                style.push_str(";font-weight:bold");
            }
            if bits & 2 != 0 {
                style.push_str(";font-style:italic");
            }

            let span = HtmlElement::new(typst_html::tag::span)
                .with_attr(typst_html::attr::style, style)
                .with_children(eco_vec![HtmlNode::Text(txt.clone(), Span::detached())]);
            new_children.push(HtmlNode::Element(span));
        }
        elem.children = new_children;
        Ok(WalkControl::SkipChildren)
    }
}

/// Derive a semantic CSS variable suffix from a slice of scopes.
fn scope_css_name(scopes: &[Scope]) -> EcoString {
    for scope in scopes.iter().rev() {
        let s = eco_format!("{scope}");
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() >= 2 && parts[0] != "source" && parts[0] != "meta" {
            return eco_format!("{}-{}", parts[0], parts[1]);
        }
    }
    "default".into()
}

/// Collect the text content of all descendant `HtmlNode::Text` nodes
/// under the given element.
fn collect_text(elem: &HtmlElement) -> String {
    let mut out = String::new();
    super::walk(elem, &mut |el| {
        for child in &el.children {
            if let HtmlNode::Text(t, _) = child {
                out.push_str(t.as_str());
            }
        }
    });
    out
}

/// Highlight code using syntect (all non-Typst languages).
fn do_syntect_highlight(code: &str, lang: &str, theme: &Theme) -> Vec<(EcoString, u8, EcoString)> {
    let syntax = SS
        .find_syntax_by_token(lang)
        .or_else(|| SS.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SS.find_syntax_plain_text());

    let highlighter = Highlighter::new(theme);
    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let default_fg = theme
        .settings
        .foreground
        .unwrap_or(syntect::highlighting::Color::BLACK);
    let mut out = Vec::new();

    for line in code.lines() {
        let Ok(ops) = parse_state.parse_line(line, &SS) else {
            continue;
        };

        for (text, op) in ScopeRegionIterator::new(&ops, line) {
            let _ = scope_stack.apply(op);
            if text.is_empty() {
                continue;
            }

            let style = highlighter.style_for_stack(scope_stack.as_slice());
            let bits = style.font_style.bits();
            let fg = style.foreground;

            let name: EcoString = if fg == default_fg && bits == 0 {
                "default".into()
            } else {
                scope_css_name(scope_stack.as_slice())
            };

            out.push((name, bits, EcoString::from(text)));
        }

        out.push(("default".into(), 0, "\n".into()));
    }

    out
}

/// Highlight code using Typst's native AST (languages: typ, typst, typc, typm).
fn do_typst_highlight(code: &str, lang: &str, theme: &Theme) -> Vec<(EcoString, u8, EcoString)> {
    let root: SyntaxNode = match lang {
        "typc" => parse_code(code),
        "typm" => parse_math(code),
        _ => typst::syntax::parse(code),
    };

    let highlighter = Highlighter::new(theme);
    let mut tokens: Vec<(EcoString, u8, EcoString)> = Vec::new();
    let mut scopes: Vec<Scope> = Vec::new();

    walk_typst_node(
        code,
        &LinkedNode::new(&root),
        &highlighter,
        &mut scopes,
        &mut tokens,
    );

    let mut out: Vec<(EcoString, u8, EcoString)> = Vec::new();
    for (name, bits, text) in &tokens {
        for (i, segment) in text.split('\n').enumerate() {
            if i > 0 {
                out.push((name.clone(), *bits, "\n".into()));
            }
            if !segment.is_empty() {
                out.push((name.clone(), *bits, EcoString::from(segment)));
            }
        }
    }
    out
}

fn walk_typst_node(
    code: &str,
    node: &LinkedNode,
    highlighter: &Highlighter,
    scopes: &mut Vec<Scope>,
    tokens: &mut Vec<(EcoString, u8, EcoString)>,
) {
    if node.children().len() == 0 {
        let text = &code[node.range()];
        if !text.is_empty() {
            let style = highlighter.style_for_stack(scopes.as_slice());
            let bits = style.font_style.bits();
            let name = scope_css_name(scopes.as_slice());
            tokens.push((name, bits, EcoString::from(text)));
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
        walk_typst_node(code, &child, highlighter, scopes, tokens);
        std::mem::swap(&mut child_scopes, scopes);
    }
}
