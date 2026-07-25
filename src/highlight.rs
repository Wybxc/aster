use std::sync::LazyLock;

use syntect::easy::ScopeRegionIterator;
use syntect::highlighting::{Highlighter, Theme, ThemeSet};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};
use typst::ecow::EcoVec;
use typst::syntax::Span;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Theme used for internal tokenization (determines token boundaries and
/// font style bits only — actual colors are deferred to CSS variables).
const TOKEN_THEME: &str = "InspiredGitHub";

/// Find every `<code data-lang="X">` in the document and replace its children
/// with `<span style="color:var(--hl-{scope})">` tokens, where `{scope}` is a
/// semantic name derived from syntect's scope stack.
///
/// Users define the `--hl-{scope}` CSS variables in their own stylesheet to
/// control colors for both light and dark mode:
///
/// ```css
/// :root {
///   --hl-keyword-control: #d73a49;
///   --hl-string-quoted: #6f42c1;
/// }
/// @media (prefers-color-scheme: dark) {
///   :root {
///     --hl-keyword-control: #f97583;
///     --hl-string-quoted: #b392f0;
///   }
/// }
/// ```
pub fn rehighlight(doc: &mut HtmlDocument) {
    let theme = &THEMES.themes[TOKEN_THEME];

    for child in doc.root_mut().children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk(e, theme);
        }
    }
}

/// Derive a semantic CSS variable suffix from a scope stack.
///
/// Iterates the stack bottom-up (most specific first), returning the first
/// non-source, non-meta scope's first two dot-separated segments joined by a
/// hyphen.  Falls back to `"default"`.
fn scope_css_name(scopes: &[Scope]) -> String {
    for scope in scopes.iter().rev() {
        let s = scope.to_string();
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() >= 2 && parts[0] != "source" && parts[0] != "meta" {
            return format!("{}-{}", parts[0], parts[1]);
        }
    }
    "default".to_string()
}

fn walk(elem: &mut HtmlElement, theme: &Theme) {
    if elem.tag == typst_html::tag::code
        && let Some(lang) = elem
            .attrs
            .0
            .iter()
            .find_map(|(a, v)| (*a.resolve() == *"data-lang").then(|| v.clone()))
    {
        let raw = collect_text(&elem.children).trim().to_owned();
        if raw.is_empty() {
            return;
        }

        let tokens = do_highlight(&raw, &lang, theme);

        let mut new_children: EcoVec<HtmlNode> = EcoVec::new();
        for (scope_name, bits, txt) in &tokens {
            let mut style = format!("color:var(--hl-{scope_name})");
            if bits & 1 != 0 {
                style.push_str(";font-weight:bold");
            }
            if bits & 2 != 0 {
                style.push_str(";font-style:italic");
            }

            let mut span = HtmlElement::new(typst_html::tag::span);
            span.attrs.push(typst_html::attr::style, style);
            span.children
                .push(HtmlNode::Text(txt.clone().into(), Span::detached()));
            new_children.push(HtmlNode::Element(span));
        }
        elem.children = new_children;
        return;
    }

    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk(e, theme);
        }
    }
}

/// Tokenise `code` using syntect's lower-level API, returning
/// `(css_var_suffix, font_style_bits, token_text)` tuples.
///
/// Uses `Highlighter` + `ParseState` + `ScopeStack` directly instead of the
/// easy `HighlightLines` wrapper, so we retain scope information for semantic
/// CSS variable naming.
fn do_highlight(
    code: &str,
    lang: &str,
    theme: &Theme,
) -> Vec<(String, u8, String)> {
    let syntax = SS
        .find_syntax_by_token(lang)
        .or_else(|| SS.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SS.find_syntax_plain_text());

    let highlighter = Highlighter::new(theme);
    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();
    let default_fg = theme.settings.foreground.unwrap_or(syntect::highlighting::Color::BLACK);
    let mut out = Vec::new();

    for line in code.lines() {
        let Ok(ops) = parse_state.parse_line(line, &SS) else {
            continue;
        };

        for (text, op) in ScopeRegionIterator::new(&ops, line) {
            // Apply the operation to update the scope stack for this region.
            let _ = scope_stack.apply(op);

            if text.is_empty() {
                continue;
            }

            let style = highlighter.style_for_stack(scope_stack.as_slice());
            let bits = style.font_style.bits();
            let fg = style.foreground;

            // Tokens whose style matches the theme default get a plain-text
            // variable name so they can be themed independently.
            let name = if fg == default_fg && bits == 0 {
                "default".to_string()
            } else {
                scope_css_name(scope_stack.as_slice())
            };

            out.push((name, bits, text.to_string()));
        }

        // Append a newline separator with the default look.
        out.push((
            "default".to_string(),
            0,
            "\n".into(),
        ));
    }

    out
}

fn collect_text(nodes: &[HtmlNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            HtmlNode::Text(t, _) => out.push_str(t),
            HtmlNode::Element(e) => out.push_str(&collect_text(&e.children)),
            _ => {}
        }
    }
    out
}
