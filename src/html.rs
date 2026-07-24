//! Custom HTML serializer that strips the document shell and applies
//! dual‑theme syntax highlighting in a single pass over the DOM tree.

use std::fmt::Write;
use std::sync::LazyLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Theme, FontStyle, Color as SynColor};
use syntect::parsing::SyntaxSet;
use typst::comemo::{Track, Tracked};
use typst::model::LateLinkResolver;
use typst::syntax::Span;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode, HtmlFrame};

static SS: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

const DEFAULT_LIGHT: &str = "InspiredGitHub";
const DEFAULT_DARK: &str = "base16-ocean.dark";

type Rgb = (u8, u8, u8);

struct Ctx {
    light: &'static Theme,
    dark: &'static Theme,
    cls: Vec<(Rgb, Rgb, u8)>,
    link: Tracked<'static, LateLinkResolver<'static>>,
    pretty: bool,
    level: usize,
    buf: String,
}

/// Serialize an `HtmlDocument` into a content HTML string (no auto-generated
/// shell).  Syntax highlighting and dual‑theme CSS are applied in one pass.
///
/// - If the document has an auto‑generated shell (detected by `lang="en"` on
///   the root), only `<body>` children are emitted.
/// - If the user wrote `#html.html(...)` (no `lang` on the root), the full
///   document structure is serialized as-is.
pub fn serialize_body(doc: &HtmlDocument) -> String {
    let link = LateLinkResolver::new(None, doc.introspector().as_ref());
    // SAFETY: the resolver lives for the duration of this call.
    let link: Tracked<'static, LateLinkResolver<'static>> =
        unsafe { std::mem::transmute(link.track()) };

    let mut ctx = Ctx {
        light: &THEMES.themes[DEFAULT_LIGHT],
        dark: &THEMES.themes[DEFAULT_DARK],
        cls: Vec::new(),
        link,
        pretty: false,
        level: 0,
        buf: String::with_capacity(doc.root().children.len() * 256),
    };

    let root = doc.root();
    let is_auto = root.attrs.0.iter().any(|(a, v)| *a.resolve() == *"lang" && v == "en");

    if is_auto {
        // Auto shell — find <body> and serialize its children.
        for child in &root.children {
            if let HtmlNode::Element(e) = child
                && e.tag == typst_html::tag::body
            {
                for gc in &e.children {
                    write_node(&mut ctx, gc);
                }
                break;
            }
        }
    } else {
        // User's #html.html() — serialize the entire root.
        write_node(&mut ctx, &HtmlNode::Element(root.clone()));
    }

    // Append style block for dual‑theme highlighting.
    if !ctx.cls.is_empty() {
        ctx.buf.push_str("<style>\n");
        for (i, &((lr, lg, lb), _, bits)) in ctx.cls.iter().enumerate() {
            let mut s = format!("color:#{lr:02x}{lg:02x}{lb:02x}");
            if bits & 1 != 0 { s.push_str(";font-weight:bold"); }
            if bits & 2 != 0 { s.push_str(";font-style:italic"); }
            let _ = writeln!(ctx.buf, ".hl-{i}{{{s}}}");
        }
        ctx.buf.push_str("@media(prefers-color-scheme:dark){\n");
        for (i, &(_, (dr, dg, db), bits)) in ctx.cls.iter().enumerate() {
            let mut s = format!("color:#{dr:02x}{dg:02x}{db:02x}");
            if bits & 1 != 0 { s.push_str(";font-weight:bold"); }
            if bits & 2 != 0 { s.push_str(";font-style:italic"); }
            let _ = writeln!(ctx.buf, ".hl-{i}{{{s}}}");
        }
        ctx.buf.push_str("}\n</style>\n");
    }

    ctx.buf
}

// ---------------------------------------------------------------------------
// Node serialization
// ---------------------------------------------------------------------------

fn write_node(ctx: &mut Ctx, node: &HtmlNode) {
    match node {
        HtmlNode::Tag(_) => {}
        HtmlNode::Text(t, span) => write_text(ctx, t, *span),
        HtmlNode::Element(e) => write_element(ctx, e),
        HtmlNode::Frame(f) => write_frame(ctx, f),
    }
}

fn write_text(ctx: &mut Ctx, text: &str, _span: Span) {
    for c in text.chars() {
        match c {
            '&' => ctx.buf.push_str("&amp;"),
            '<' => ctx.buf.push_str("&lt;"),
            '>' => ctx.buf.push_str("&gt;"),
            '"' => ctx.buf.push_str("&quot;"),
            '\'' => ctx.buf.push_str("&apos;"),
            c if c.is_ascii_graphic() || c.is_ascii_whitespace() => ctx.buf.push(c),
            c => { let _ = write!(ctx.buf, "&#x{:x};", c as u32); }
        }
    }
}

fn write_element(ctx: &mut Ctx, elem: &HtmlElement) {
    // Highlight <code data-lang="X">
    if elem.tag == typst_html::tag::code
        && let Some(lang) = elem.attrs.0.iter().find_map(|(a, v)| {
            (*a.resolve() == *"data-lang").then(|| v.clone())
        })
    {
        return write_highlighted(ctx, elem, &lang);
    }

    let tag: &str = &elem.tag.resolve();
    ctx.buf.push('<');
    ctx.buf.push_str(tag);
    for (attr, value) in &elem.attrs.0 {
        ctx.buf.push(' ');
        ctx.buf.push_str(&attr.resolve());
        if !value.is_empty() {
            ctx.buf.push_str("=\"");
            for c in value.chars() {
                match c {
                    '"' => ctx.buf.push_str("&quot;"),
                    '&' => ctx.buf.push_str("&amp;"),
                    '<' => ctx.buf.push_str("&lt;"),
                    '>' => ctx.buf.push_str("&gt;"),
                    c => ctx.buf.push(c),
                }
            }
            ctx.buf.push('"');
        }
    }

    let void = typst_html::tag::is_void(elem.tag);
    let foreign = typst_html::tag::is_foreign_self_closing(elem.tag);
    if foreign { ctx.buf.push('/'); }
    ctx.buf.push('>');
    if void || foreign { return; }

    // Leading newline for <pre>/<textarea>
    let leads_nl = matches!(elem.tag, typst_html::tag::pre | typst_html::tag::textarea)
        && elem.children.first().map_or(false, |c| matches!(c, HtmlNode::Text(t, _) if t.starts_with(['\n', '\r'])));
    if leads_nl { ctx.buf.push('\n'); }

    if typst_html::tag::is_raw(elem.tag) {
        for child in &elem.children {
            if let HtmlNode::Text(t, _) = child { ctx.buf.push_str(t); }
        }
    } else if typst_html::tag::is_escapable_raw(elem.tag) {
        for child in &elem.children {
            if let HtmlNode::Text(t, sp) = child { write_text(ctx, t, *sp); }
        }
    } else {
        for child in &elem.children {
            write_node(ctx, child);
        }
    }

    ctx.buf.push_str("</");
    ctx.buf.push_str(tag);
    ctx.buf.push('>');
}

// ---------------------------------------------------------------------------
// Highlighted code blocks
// ---------------------------------------------------------------------------

fn write_highlighted(ctx: &mut Ctx, code_elem: &HtmlElement, lang: &str) {
    let raw = collect_text(&code_elem.children).trim().to_owned();
    if raw.is_empty() {
        return fallback_element(ctx, code_elem);
    }

    let ltokens = do_highlight(&raw, lang, ctx.light);
    let dtokens = do_highlight(&raw, lang, ctx.dark);

    // Opening <code> tag
    let tag: &str = &code_elem.tag.resolve();
    ctx.buf.push('<');
    ctx.buf.push_str(tag);
    for (attr, value) in &code_elem.attrs.0 {
        ctx.buf.push(' ');
        ctx.buf.push_str(&attr.resolve());
        if !value.is_empty() {
            ctx.buf.push_str("=\"");
            for c in value.chars() {
                match c { '"' => ctx.buf.push_str("&quot;"), '&' => ctx.buf.push_str("&amp;"), _ => ctx.buf.push(c) }
            }
            ctx.buf.push('"');
        }
    }
    ctx.buf.push('>');

    // Highlighted spans
    for ((ls, ltxt), (ds, _)) in ltokens.iter().zip(dtokens.iter()) {
        let lfg = (ls.foreground.r, ls.foreground.g, ls.foreground.b);
        let dfg = (ds.foreground.r, ds.foreground.g, ds.foreground.b);
        let bits = ls.font_style.bits() | ds.font_style.bits();

        let idx = ctx.cls.iter().position(|&(l, d, b)| l == lfg && d == dfg && b == bits)
            .unwrap_or_else(|| { ctx.cls.push((lfg, dfg, bits)); ctx.cls.len() - 1 });

        if lfg == (0,0,0) && dfg == (0,0,0) && bits == 0 {
            write_text(ctx, ltxt, Span::detached());
        } else {
            let _ = write!(ctx.buf, r#"<span class="hl-{idx}">"#);
            write_text(ctx, ltxt, Span::detached());
            ctx.buf.push_str("</span>");
        }
    }

    ctx.buf.push_str("</");
    ctx.buf.push_str(tag);
    ctx.buf.push('>');
}

fn fallback_element(ctx: &mut Ctx, elem: &HtmlElement) {
    let tag: &str = &elem.tag.resolve();
    ctx.buf.push('<'); ctx.buf.push_str(tag);
    for (a, v) in &elem.attrs.0 {
        ctx.buf.push(' '); ctx.buf.push_str(&a.resolve());
        if !v.is_empty() {
            ctx.buf.push_str("=\"");
            for c in v.chars() {
                match c { '"' => ctx.buf.push_str("&quot;"), '&' => ctx.buf.push_str("&amp;"), _ => ctx.buf.push(c) }
            }
            ctx.buf.push('"');
        }
    }
    ctx.buf.push('>');
    for child in &elem.children { write_node(ctx, child); }
    ctx.buf.push_str("</"); ctx.buf.push_str(tag); ctx.buf.push('>');
}

// ---------------------------------------------------------------------------
// Frames (SVG)
// ---------------------------------------------------------------------------

fn write_frame(ctx: &mut Ctx, frame: &HtmlFrame) {
    let css_inline = frame.css.to_inline().to_string();
    let svg = typst_svg::svg_in_html(
        &frame.inner, frame.text_size, ctx.pretty,
        frame.id.as_deref(),
        &css_inline,
        &frame.anchors,
        ctx.link,
    );
    ctx.buf.push_str(&svg);
}

// ---------------------------------------------------------------------------
// Highlight helper
// ---------------------------------------------------------------------------

fn do_highlight(code: &str, lang: &str, theme: &Theme) -> Vec<(syntect::highlighting::Style, String)> {
    let syntax = SS
        .find_syntax_by_token(lang)
        .or_else(|| SS.find_syntax_by_extension(lang))
        .unwrap_or_else(|| SS.find_syntax_plain_text());

    let mut hl = HighlightLines::new(syntax, theme);
    let mut out = Vec::new();
    for line in code.lines() {
        if let Ok(tokens) = hl.highlight_line(line, &SS) {
            for (st, txt) in &tokens { out.push((*st, txt.to_string())); }
        }
        out.push((syntect::highlighting::Style {
            foreground: theme.settings.foreground.unwrap_or(SynColor::BLACK),
            background: SynColor::WHITE,
            font_style: FontStyle::empty(),
        }, "\n".into()));
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
