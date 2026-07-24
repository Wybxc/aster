use std::fmt::Write;
use std::sync::LazyLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Theme, FontStyle, Color as SynColor};
use syntect::parsing::SyntaxSet;
use typst::ecow::EcoVec;
use typst::syntax::Span;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Default themes shipped with `syntect`.
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Keys used as defaults in `aster.toml [raw]`.
const DEFAULT_LIGHT: &str = "InspiredGitHub";
const DEFAULT_DARK: &str = "base16-ocean.dark";

// ---------------------------------------------------------------------------
// DOM‑level API
// ---------------------------------------------------------------------------

/// Re‑highlight every `<code>` element with `data-lang`, replacing its children
/// with `<span class="hl-N">`, and return a `<style>` block string.
///
/// Pass `None` to use the Aster defaults (`InspiredGitHub` light,
/// `base16-ocean.dark` dark).
pub fn apply_to_doc(
    doc: &mut HtmlDocument,
    light: Option<&str>,
    dark: Option<&str>,
) -> String {
    let light = light
        .and_then(|k| THEMES.themes.get(k))
        .unwrap_or_else(|| &THEMES.themes[DEFAULT_LIGHT]);
    let dark = dark
        .and_then(|k| THEMES.themes.get(k))
        .unwrap_or_else(|| &THEMES.themes[DEFAULT_DARK]);

    let mut cls: Vec<(Rgb, Rgb, u8)> = Vec::new();
    let root = doc.root_mut();

    walk_code_blocks(root, light, dark, &mut cls);

    // Build CSS
    if cls.is_empty() {
        return String::new();
    }
    let mut style = String::from("<style>\n");
    for (i, &((lr,lg,lb), _, bits)) in cls.iter().enumerate() {
        let mut s = format!("color:#{lr:02x}{lg:02x}{lb:02x}");
        if bits & 1 != 0 { s.push_str(";font-weight:bold"); }
        if bits & 2 != 0 { s.push_str(";font-style:italic"); }
        let _ = writeln!(style, ".hl-{i}{{{s}}}");
    }
    style.push_str("@media(prefers-color-scheme:dark){\n");
    for (i, &(_, (dr,dg,db), bits)) in cls.iter().enumerate() {
        let mut s = format!("color:#{dr:02x}{dg:02x}{db:02x}");
        if bits & 1 != 0 { s.push_str(";font-weight:bold"); }
        if bits & 2 != 0 { s.push_str(";font-style:italic"); }
        let _ = writeln!(style, ".hl-{i}{{{s}}}");
    }
    style.push_str("}\n</style>\n");
    style
}

// ---------------------------------------------------------------------------
// DOM traversal
// ---------------------------------------------------------------------------

type Rgb = (u8, u8, u8);

fn walk_code_blocks(
    elem: &mut HtmlElement,
    light: &Theme,
    dark: &Theme,
    cls: &mut Vec<(Rgb, Rgb, u8)>,
) {
    let is_code = elem.tag == typst_html::tag::code;
    let lang = is_code.then(|| {
        elem.attrs.0.iter().find_map(|(attr, val)| {
            (*attr.resolve() == *"data-lang").then(|| val.clone())
        })
    }).flatten();

    if let Some(lang_str) = lang {
        let raw = collect_text(&elem.children);
        if raw.trim().is_empty() {
            recurse(elem, light, dark, cls);
            return;
        }

        let ltokens = highlight(&raw, &lang_str, light);
        let dtokens = highlight(&raw, &lang_str, dark);

        let mut new_children = EcoVec::new();
        for ((ls, ltxt), (ds, _)) in ltokens.iter().zip(dtokens.iter()) {
            let lfg = (ls.foreground.r, ls.foreground.g, ls.foreground.b);
            let dfg = (ds.foreground.r, ds.foreground.g, ds.foreground.b);
            let bits = ls.font_style.bits() | ds.font_style.bits();

            let idx = cls.iter().position(|&(l, d, b)| l == lfg && d == dfg && b == bits)
                .unwrap_or_else(|| { cls.push((lfg, dfg, bits)); cls.len() - 1 });

            if lfg == (0,0,0) && dfg == (0,0,0) && bits == 0 {
                new_children.push(HtmlNode::Text(ltxt.clone().into(), Span::detached()));
            } else {
                let mut span = HtmlElement::new(typst_html::tag::span);
                span.attrs.push(typst_html::attr::class, format!("hl-{idx}"));
                span.children.push(HtmlNode::Text(ltxt.clone().into(), Span::detached()));
                new_children.push(HtmlNode::Element(span));
            }
        }
        elem.children = new_children;
    } else {
        recurse(elem, light, dark, cls);
    }
}

fn recurse(elem: &mut HtmlElement, light: &Theme, dark: &Theme, cls: &mut Vec<(Rgb, Rgb, u8)>) {
    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk_code_blocks(e, light, dark, cls);
        }
    }
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

// ---------------------------------------------------------------------------
// Syntect highlighting
// ---------------------------------------------------------------------------

fn highlight(code: &str, lang: &str, theme: &Theme) -> Vec<(syntect::highlighting::Style, String)> {
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
        out.push((syntect::highlighting::Style {
            foreground: theme.settings.foreground.unwrap_or(SynColor::BLACK),
            background: SynColor::WHITE,
            font_style: FontStyle::empty(),
        }, "\n".into()));
    }
    out
}
