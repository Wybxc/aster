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

const DEFAULT_LIGHT: &str = "InspiredGitHub";
const DEFAULT_DARK: &str = "base16-ocean.dark";

type Rgb = (u8, u8, u8);

/// Find every `<code data-lang="X">` in the document, re‑highlight it with
/// syntect using both themes, replace its children with `<span class="hl-N">`,
/// and return a `<style>` block string (to be appended after serialization).
pub fn rehighlight(doc: &mut HtmlDocument) -> String {
    let light = &THEMES.themes[DEFAULT_LIGHT];
    let dark = &THEMES.themes[DEFAULT_DARK];
    let mut cls: Vec<(Rgb, Rgb, u8)> = Vec::new();

    for child in doc.root_mut().children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk(e, light, dark, &mut cls);
        }
    }

    if cls.is_empty() {
        return String::new();
    }

    let mut style = String::from("<style>\n");
    for (i, &((lr, lg, lb), _, bits)) in cls.iter().enumerate() {
        let mut s = format!("color:#{lr:02x}{lg:02x}{lb:02x}");
        if bits & 1 != 0 {
            s.push_str(";font-weight:bold");
        }
        if bits & 2 != 0 {
            s.push_str(";font-style:italic");
        }
        let _ = writeln!(style, ".hl-{i}{{{s}}}");
    }
    style.push_str("@media(prefers-color-scheme:dark){\n");
    for (i, &(_, (dr, dg, db), bits)) in cls.iter().enumerate() {
        let mut s = format!("color:#{dr:02x}{dg:02x}{db:02x}");
        if bits & 1 != 0 {
            s.push_str(";font-weight:bold");
        }
        if bits & 2 != 0 {
            s.push_str(";font-style:italic");
        }
        let _ = writeln!(style, ".hl-{i}{{{s}}}");
    }
    style.push_str("}\n</style>\n");
    style
}

fn walk(elem: &mut HtmlElement, light: &Theme, dark: &Theme, cls: &mut Vec<(Rgb, Rgb, u8)>) {
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

        let ltokens = do_highlight(&raw, &lang, light);
        let dtokens = do_highlight(&raw, &lang, dark);

        let mut new_children: EcoVec<HtmlNode> = EcoVec::new();
        for ((ls, ltxt), (ds, _)) in ltokens.iter().zip(dtokens.iter()) {
            let lfg = (ls.foreground.r, ls.foreground.g, ls.foreground.b);
            let dfg = (ds.foreground.r, ds.foreground.g, ds.foreground.b);
            let bits = ls.font_style.bits() | ds.font_style.bits();

            let idx = cls
                .iter()
                .position(|&(l, d, b)| l == lfg && d == dfg && b == bits)
                .unwrap_or_else(|| {
                    cls.push((lfg, dfg, bits));
                    cls.len() - 1
                });

            if lfg == (0, 0, 0) && dfg == (0, 0, 0) && bits == 0 {
                new_children.push(HtmlNode::Text(ltxt.clone().into(), Span::detached()));
            } else {
                let mut span = HtmlElement::new(typst_html::tag::span);
                span.attrs
                    .push(typst_html::attr::class, format!("hl-{idx}"));
                span.children
                    .push(HtmlNode::Text(ltxt.clone().into(), Span::detached()));
                new_children.push(HtmlNode::Element(span));
            }
        }
        elem.children = new_children;
        return;
    }

    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk(e, light, dark, cls);
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
