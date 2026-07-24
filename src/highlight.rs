use std::fmt::Write;
use std::sync::LazyLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{self, Theme, ThemeSettings, Style, FontStyle, Color as SynColor};
use syntect::parsing::SyntaxSet;
use typst::ecow::EcoVec;
use typst::syntax::Span;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

static LIGHT: LazyLock<Theme> = LazyLock::new(||
    build_theme("Aster Light", (0x3b, 0x3b, 0x3f), &LIGHT_ITEMS));
static DARK: LazyLock<Theme> = LazyLock::new(||
    build_theme("Aster Dark",  (0xdc, 0xdc, 0xdc), &DARK_ITEMS));

type Rgb = (u8, u8, u8);

struct Rule { scope: &'static str, fg: Rgb, bold: bool, italic: bool }

const LIGHT_ITEMS: &[Rule] = &[
    Rule { scope: "comment",                         fg: (0x74, 0x74, 0x7c), bold: false, italic: false },
    Rule { scope: "constant",                        fg: (0x1d, 0x6c, 0x76), bold: false, italic: false },
    Rule { scope: "constant.character.escape",       fg: (0x1d, 0x6c, 0x76), bold: false, italic: false },
    Rule { scope: "entity.name",                     fg: (0x00, 0x00, 0x00), bold: true,  italic: false },
    Rule { scope: "entity.name.label",               fg: (0x1d, 0x6c, 0x76), bold: false, italic: false },
    Rule { scope: "entity.name.section",             fg: (0x00, 0x00, 0x00), bold: true,  italic: false },
    Rule { scope: "keyword",                         fg: (0xd7, 0x39, 0x48), bold: false, italic: false },
    Rule { scope: "keyword.control",                 fg: (0xd7, 0x39, 0x48), bold: false, italic: false },
    Rule { scope: "keyword.operator",                fg: (0x1d, 0x6c, 0x76), bold: false, italic: false },
    Rule { scope: "markup.bold",                     fg: (0x00, 0x00, 0x00), bold: true,  italic: false },
    Rule { scope: "markup.italic",                   fg: (0x00, 0x00, 0x00), bold: false, italic: true  },
    Rule { scope: "markup.list",                     fg: (0x8b, 0x41, 0xb1), bold: false, italic: false },
    Rule { scope: "markup.raw",                      fg: (0x6b, 0x6b, 0x6f), bold: false, italic: false },
    Rule { scope: "punctuation.definition.list",     fg: (0x8b, 0x41, 0xb1), bold: false, italic: false },
    Rule { scope: "punctuation.definition.math",     fg: (0x19, 0x88, 0x10), bold: false, italic: false },
    Rule { scope: "string",                          fg: (0x1d, 0x6c, 0x76), bold: false, italic: false },
    Rule { scope: "string.other.math.typst",         fg: (0x00, 0x00, 0x00), bold: false, italic: false },
    Rule { scope: "variable.language",               fg: (0xd7, 0x39, 0x48), bold: false, italic: false },
    Rule { scope: "variable.other",                  fg: (0x00, 0x00, 0x00), bold: false, italic: false },
];

const DARK_ITEMS: &[Rule] = &[
    Rule { scope: "comment",                         fg: (0x6a, 0x6a, 0x72), bold: false, italic: false },
    Rule { scope: "constant",                        fg: (0x66, 0xc2, 0xcd), bold: false, italic: false },
    Rule { scope: "constant.character.escape",       fg: (0x66, 0xc2, 0xcd), bold: false, italic: false },
    Rule { scope: "entity.name",                     fg: (0xe6, 0xe6, 0xe6), bold: true,  italic: false },
    Rule { scope: "entity.name.label",               fg: (0x66, 0xc2, 0xcd), bold: false, italic: false },
    Rule { scope: "entity.name.section",             fg: (0xe6, 0xe6, 0xe6), bold: true,  italic: false },
    Rule { scope: "keyword",                         fg: (0xff, 0x6b, 0x6b), bold: false, italic: false },
    Rule { scope: "keyword.control",                 fg: (0xff, 0x6b, 0x6b), bold: false, italic: false },
    Rule { scope: "keyword.operator",                fg: (0x66, 0xc2, 0xcd), bold: false, italic: false },
    Rule { scope: "markup.bold",                     fg: (0xe6, 0xe6, 0xe6), bold: true,  italic: false },
    Rule { scope: "markup.italic",                   fg: (0xe6, 0xe6, 0xe6), bold: false, italic: true  },
    Rule { scope: "markup.list",                     fg: (0xb7, 0x7a, 0xd5), bold: false, italic: false },
    Rule { scope: "markup.raw",                      fg: (0x9b, 0x9b, 0xa0), bold: false, italic: false },
    Rule { scope: "punctuation.definition.list",     fg: (0xb7, 0x7a, 0xd5), bold: false, italic: false },
    Rule { scope: "punctuation.definition.math",     fg: (0x44, 0xbb, 0x66), bold: false, italic: false },
    Rule { scope: "string",                          fg: (0x66, 0xc2, 0xcd), bold: false, italic: false },
    Rule { scope: "string.other.math.typst",         fg: (0xe6, 0xe6, 0xe6), bold: false, italic: false },
    Rule { scope: "variable.language",               fg: (0xff, 0x6b, 0x6b), bold: false, italic: false },
    Rule { scope: "variable.other",                  fg: (0xe6, 0xe6, 0xe6), bold: false, italic: false },
];

fn color(rgb: Rgb) -> SynColor { SynColor { r: rgb.0, g: rgb.1, b: rgb.2, a: 255 } }

fn build_theme(name: &str, fg: Rgb, rules: &[Rule]) -> Theme {
    Theme {
        name: Some(name.into()),
        author: None,
        settings: ThemeSettings { foreground: Some(color(fg)), ..Default::default() },
        scopes: rules.iter().map(|r| {
            let mut fs = FontStyle::empty();
            if r.bold   { fs |= FontStyle::BOLD; }
            if r.italic { fs |= FontStyle::ITALIC; }
            highlighting::ThemeItem {
                scope: r.scope.parse().unwrap(),
                style: highlighting::StyleModifier {
                    foreground: Some(color(r.fg)),
                    font_style: Some(fs),
                    ..Default::default()
                },
            }
        }).collect(),
    }
}

// ---------------------------------------------------------------------------
// DOM‑level API — apply highlighting in-place on an HtmlDocument
// ---------------------------------------------------------------------------

/// Re‑highlight every `<code>` element that carries a `data-lang` attribute,
/// replacing its children with `<span class="hl-N">` elements, and return a
/// CSS style block string (to be injected by the caller after serialization).
pub fn apply_to_doc(
    doc: &mut HtmlDocument,
    light: Option<&Theme>,
    dark: Option<&Theme>,
) -> String {
    let light = light.unwrap_or(&LIGHT);
    let dark = dark.unwrap_or(&DARK);

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
// DOM walk helpers
// ---------------------------------------------------------------------------

fn walk_code_blocks(
    elem: &mut HtmlElement,
    light: &Theme,
    dark: &Theme,
    cls: &mut Vec<(Rgb, Rgb, u8)>,
) {
    // Find <code> elements with data-lang
    let is_code = elem.tag == typst_html::tag::code;
    let lang = if is_code {
        elem.attrs.0.iter().find_map(|(attr, val)| {
            if *attr.resolve() == *"data-lang" { Some(val.clone()) } else { None }
        })
    } else {
        None
    };

    if let Some(lang_str) = lang {
        // Extract raw text from children
        let raw = collect_text(&elem.children);
        if raw.trim().is_empty() {
            // Still recurse into children for nested <code> (shouldn't happen)
            for child in elem.children.make_mut().iter_mut() {
                if let HtmlNode::Element(e) = child {
                    walk_code_blocks(e, light, dark, cls);
                }
            }
            return;
        }

        // Re‑highlight
        let ltokens = highlight(&raw, &lang_str, light);
        let dtokens = highlight(&raw, &lang_str, dark);

        // Build new children: <span class="hl-N">token</span>
        let mut new_children = EcoVec::new();
        // █ a span that holds consecutive identical-style tokens
        for ((ls, ltxt), (ds, _)) in ltokens.iter().zip(dtokens.iter()) {
            let lfg = (ls.foreground.r, ls.foreground.g, ls.foreground.b);
            let dfg = (ds.foreground.r, ds.foreground.g, ds.foreground.b);
            let bits = ls.font_style.bits() | ds.font_style.bits();

            let idx = cls.iter().position(|&(l, d, b)| l == lfg && d == dfg && b == bits)
                .unwrap_or_else(|| { cls.push((lfg, dfg, bits)); cls.len() - 1 });

            if lfg == (0,0,0) && dfg == (0,0,0) && bits == 0 {
                // Plain text — avoid wasteful spans
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
        // Recurse
        for child in elem.children.make_mut().iter_mut() {
            if let HtmlNode::Element(e) = child {
                walk_code_blocks(e, light, dark, cls);
            }
        }
    }
}

/// Collect text content from a flat list of child nodes (recursing into
/// elements but ignoring Tag / Frame markers).
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

fn highlight(code: &str, lang: &str, theme: &Theme) -> Vec<(Style, String)> {
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
        out.push((Style {
            foreground: theme.settings.foreground.unwrap_or(SynColor::BLACK),
            background: SynColor::WHITE,
            font_style: FontStyle::empty(),
        }, "\n".into()));
    }
    out
}
