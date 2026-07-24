use std::fmt::Write;
use std::sync::LazyLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{self, Theme, ThemeSettings, Style, FontStyle, Color as SynColor};
use syntect::parsing::SyntaxSet;

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
        settings: ThemeSettings {
            foreground: Some(color(fg)),
            ..Default::default()
        },
        scopes: rules
            .iter()
            .map(|r| {
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
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Post-process an HTML string: find every `<code data-lang="X">` block,
/// re-highlight it with both themes, replace its contents with dual‑theme
/// `<span>` tags, and inject a `<style>` block into the output.
pub fn apply(html: &str, light: Option<&Theme>, dark: Option<&Theme>) -> String {
    let light = light.unwrap_or(&LIGHT);
    let dark = dark.unwrap_or(&DARK);

    let mut result = String::with_capacity(html.len() + 4096);
    let mut style = String::new();
    // Maps (light_rgb, dark_rgb, fontstyle_bits) → class index.
    let mut cls: Vec<(Rgb, Rgb, u8)> = Vec::new();

    let mut pos = 0;
    while let Some(start) = html[pos..].find("<code") {
        let abs = pos + start;
        let rest = &html[abs..];

        // Must have data-lang=; otherwise skip.
        let Some(lang_start) = rest.find("data-lang=") else {
            pos = abs + 5; continue
        };
        let after_eq  = &rest[lang_start + 10..];
        let q = char::from(after_eq.as_bytes().first().copied().unwrap_or(b'"'));
        let end_q = after_eq[1..].find(q).map(|i| i + 1).unwrap_or(0);
        let lang = &after_eq[1..end_q];

        let after_open = rest.find('>').map(|i| i + 1).unwrap_or(0);
        let inner = &rest[after_open..];
        let Some(end_tag) = inner.find("</code>") else { pos = abs + 5; continue };

        let inner_html = &inner[..end_tag];
        let raw = strip_html(inner_html).trim().to_owned();

        // Re-highlight
        let ltokens = highlight(&raw, lang, light);
        let dtokens = highlight(&raw, lang, dark);

        // Write everything before this tag
        result.push_str(&html[pos..abs]);

        // Open tag
        let tag_end = rest.find('>').map(|i| i + 1).unwrap_or(0);
        result.push_str(&rest[..tag_end]);

        // Emit dual‑theme spans
        for ((ls, ltxt), (ds, _)) in ltokens.iter().zip(dtokens.iter()) {
            let lfg = (ls.foreground.r, ls.foreground.g, ls.foreground.b);
            let dfg = (ds.foreground.r, ds.foreground.g, ds.foreground.b);
            let bits = ls.font_style.bits() | ds.font_style.bits();

            let idx = cls.iter().position(|&(l, d, b)| l == lfg && d == dfg && b == bits)
                .unwrap_or_else(|| { cls.push((lfg, dfg, bits)); cls.len() - 1 });

            let txt = ltxt
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            if lfg == (0,0,0) && dfg == (0,0,0) && bits == 0 {
                result.push_str(&txt);
            } else {
                let _ = write!(result, r#"<span class="hl-{idx}">{txt}</span>"#);
            }
        }

        result.push_str("</code>");
        pos = abs + after_open + end_tag + "</code>".len();
    }
    result.push_str(&html[pos..]);

    // Build style block
    if !cls.is_empty() {
        style.push_str("<style>\n");
        for (i, &((lr,lg,lb), (_dr,_dg,_db), bits)) in cls.iter().enumerate() {
            let mut s = format!("color:#{lr:02x}{lg:02x}{lb:02x}");
            if bits & 1 != 0 { s.push_str(";font-weight:bold"); }
            if bits & 2 != 0 { s.push_str(";font-style:italic"); }
            let _ = writeln!(style, ".hl-{i}{{{s}}}");
        }
        style.push_str("@media(prefers-color-scheme:dark){\n");
        for (i, &((_,_,_), (dr,dg,db), bits)) in cls.iter().enumerate() {
            let mut s = format!("color:#{dr:02x}{dg:02x}{db:02x}");
            if bits & 1 != 0 { s.push_str(";font-weight:bold"); }
            if bits & 2 != 0 { s.push_str(";font-style:italic"); }
            let _ = writeln!(style, ".hl-{i}{{{s}}}");
        }
        style.push_str("}\n</style>\n");
    }

    if !style.is_empty() {
        if let Some(body) = result.rfind("</body>") {
            result.insert_str(body, &style);
        } else {
            result.push_str(&style);
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Highlight a single block of code, return (Style, text) pairs.
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
        // Syntect 5.x omits the newline; add one explicitly.
        out.push((Style {
            foreground: theme.settings.foreground.unwrap_or(SynColor::BLACK),
            background: SynColor::WHITE,
            font_style: FontStyle::empty(),
        }, "\n".into()));
    }
    out
}

/// Strip HTML tags & entities to recover raw text.
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let (mut tag, mut ent) = (false, false);
    let mut ent_buf = String::new();

    for c in s.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if tag => {}
            '&' => { ent = true; ent_buf.clear(); }
            ';' if ent => {
                let e = ent_buf.as_str();
                match e {
                    "amp"  => out.push('&'),
                    "lt"   => out.push('<'),
                    "gt"   => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    _ if e.starts_with('#') => {
                        let n: u32 = e[1..].parse().unwrap_or(0);
                        if let Some(ch) = char::from_u32(n) { out.push(ch); }
                    }
                    _ => {}
                }
                ent = false;
            }
            _ if ent => ent_buf.push(c),
            _ => out.push(c),
        }
    }
    out
}
