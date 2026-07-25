use std::fmt::Write;
use std::sync::LazyLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SynColor, FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use typst::ecow::EcoVec;
use typst::syntax::Span;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Theme used for internal tokenization (determines token boundaries and
/// font style bits only — actual colors are deferred to CSS variables).
const TOKEN_THEME: &str = "InspiredGitHub";

type Rgb = (u8, u8, u8);

/// Find every `<code data-lang="X">` in the document, tokenise it, replace its
/// children with `<span class="hl-N">` and return a `<style>` block that maps
/// each class to a CSS custom property.
///
/// The generated CSS uses `var(--hl-N)` instead of hardcoded hex values so
/// that users can supply their own theme via CSS variables:
///
/// ```css
/// :root {
///   --hl-0: #d73a49;
///   --hl-1: #6f42c1;
/// }
/// @media (prefers-color-scheme: dark) {
///   :root {
///     --hl-0: #f97583;
///     --hl-1: #b392f0;
///   }
/// }
/// ```
pub fn rehighlight(doc: &mut HtmlDocument) -> String {
    let theme = &THEMES.themes[TOKEN_THEME];
    let mut cls: Vec<(Rgb, u8)> = Vec::new();

    for child in doc.root_mut().children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk(e, theme, &mut cls);
        }
    }

    if cls.is_empty() {
        return String::new();
    }

    let mut style = String::from("<style>\n");
    for (i, &((r, g, b), bits)) in cls.iter().enumerate() {
        let _ = write!(style, ".hl-{i}{{color:var(--hl-{i},#{r:02x}{g:02x}{b:02x})");
        if bits & 1 != 0 {
            style.push_str(";font-weight:bold");
        }
        if bits & 2 != 0 {
            style.push_str(";font-style:italic");
        }
        style.push_str("}\n");
    }
    style.push_str("@media(prefers-color-scheme:dark){\n");
    for (i, &((r, g, b), bits)) in cls.iter().enumerate() {
        let _ = write!(style, ".hl-{i}{{color:var(--hl-{i}-dark,#{r:02x}{g:02x}{b:02x})");
        if bits & 1 != 0 {
            style.push_str(";font-weight:bold");
        }
        if bits & 2 != 0 {
            style.push_str(";font-style:italic");
        }
        style.push_str("}\n");
    }
    style.push_str("}\n</style>\n");
    style
}

fn walk(elem: &mut HtmlElement, theme: &Theme, cls: &mut Vec<(Rgb, u8)>) {
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
        for (st, txt) in &tokens {
            let fg = (st.foreground.r, st.foreground.g, st.foreground.b);
            let bits = st.font_style.bits();

            let idx = cls
                .iter()
                .position(|&(f, b)| f == fg && b == bits)
                .unwrap_or_else(|| {
                    cls.push((fg, bits));
                    cls.len() - 1
                });

            let mut span = HtmlElement::new(typst_html::tag::span);
            span.attrs
                .push(typst_html::attr::class, format!("hl-{idx}"));
            span.children
                .push(HtmlNode::Text(txt.clone().into(), Span::detached()));
            new_children.push(HtmlNode::Element(span));
        }
        elem.children = new_children;
        return;
    }

    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk(e, theme, cls);
        }
    }
}

fn do_highlight(
    code: &str,
    lang: &str,
    theme: &Theme,
) -> Vec<(syntect::highlighting::Style, String)> {
    let syntax = SS
        .find_syntax_by_token(lang)
        .or_else(|| SS.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SS.find_syntax_plain_text());

    let mut hl = HighlightLines::new(syntax, theme);
    let mut out = Vec::new();
    for line in code.lines() {
        if let Ok(tokens) = hl.highlight_line(line, &SS) {
            for (st, txt) in &tokens {
                out.push((*st, txt.to_string()));
            }
        }
        out.push((
            syntect::highlighting::Style {
                foreground: theme.settings.foreground.unwrap_or(SynColor::BLACK),
                background: SynColor::WHITE,
                font_style: FontStyle::empty(),
            },
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
