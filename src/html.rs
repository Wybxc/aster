//! Custom HTML serializer with dual‑theme syntax highlighting.
//!
//! Two entry points:
//! - [`serialize_body`] — emits only the `<body>` children (no shell).
//! - [`serialize_full`] — emits the full document tree as-is.

use std::fmt::Write;
use std::sync::LazyLock;

use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Theme, FontStyle, Color as SynColor};
use syntect::parsing::SyntaxSet;
use typst::comemo::{Track, Tracked};
use typst::ecow::EcoVec;
use typst::model::LateLinkResolver;
use typst::syntax::Span;
use typst_html::{HtmlAttr, HtmlDocument, HtmlElement, HtmlFrame, HtmlNode};

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
    buf: String,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Serialize only the `<body>` children (auto shell is skipped).
pub fn serialize_body(doc: &HtmlDocument) -> String {
    let mut ctx = new_ctx(doc);
    let root = doc.root();
    for child in &root.children {
        if let HtmlNode::Element(e) = child
            && e.tag == typst_html::tag::body
        {
            for gc in &e.children { write_node(&mut ctx, gc); }
            break;
        }
    }
    append_style(&mut ctx);
    ctx.buf
}

/// Serialize the entire document tree preserving all structure.
pub fn serialize_full(doc: &HtmlDocument) -> String {
    let mut ctx = new_ctx(doc);
    write_node(&mut ctx, &HtmlNode::Element(doc.root().clone()));
    append_style(&mut ctx);
    ctx.buf
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

fn new_ctx(doc: &HtmlDocument) -> Ctx {
    let link = LateLinkResolver::new(None, doc.introspector().as_ref());
    let link: Tracked<'static, LateLinkResolver<'static>> =
        unsafe { std::mem::transmute(link.track()) };
    Ctx {
        light: &THEMES.themes[DEFAULT_LIGHT],
        dark: &THEMES.themes[DEFAULT_DARK],
        cls: Vec::new(),
        link,
        buf: String::with_capacity(doc.root().children.len() * 256),
    }
}

fn append_style(ctx: &mut Ctx) {
    if ctx.cls.is_empty() { return; }
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
    // Highlight <code data-lang="X"> inline.
    if elem.tag == typst_html::tag::code
        && let Some(lang) = elem.attrs.0.iter().find_map(|(a, v)| {
            (*a.resolve() == *"data-lang").then(|| v.clone())
        })
    {
        return write_highlighted(ctx, elem, &lang);
    }

    emit_open(ctx, &elem.tag.resolve(), &elem.attrs.0);

    let void = typst_html::tag::is_void(elem.tag);
    let foreign = typst_html::tag::is_foreign_self_closing(elem.tag);
    if void || foreign { return; }

    if matches!(elem.tag, typst_html::tag::pre | typst_html::tag::textarea)
        && elem.children.first().map_or(false, |c| matches!(c, HtmlNode::Text(t, _) if t.starts_with(['\n', '\r'])))
    { ctx.buf.push('\n'); }

    if typst_html::tag::is_raw(elem.tag) {
        for child in &elem.children {
            if let HtmlNode::Text(t, _) = child { ctx.buf.push_str(t); }
        }
    } else if typst_html::tag::is_escapable_raw(elem.tag) {
        for child in &elem.children {
            if let HtmlNode::Text(t, sp) = child { write_text(ctx, t, *sp); }
        }
    } else {
        for child in &elem.children { write_node(ctx, child); }
    }

    ctx.buf.push_str("</");
    ctx.buf.push_str(&elem.tag.resolve());
    ctx.buf.push('>');
}

fn emit_open(ctx: &mut Ctx, tag: &str, attrs: &EcoVec<(HtmlAttr, typst::ecow::EcoString)>) {
    ctx.buf.push('<');
    ctx.buf.push_str(tag);
    for (attr, value) in attrs {
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
    ctx.buf.push('>');
}

// ---------------------------------------------------------------------------
// Code highlighting
// ---------------------------------------------------------------------------

fn write_highlighted(ctx: &mut Ctx, code_elem: &HtmlElement, lang: &str) {
    let raw = collect_text(&code_elem.children).trim().to_owned();
    if raw.is_empty() {
        emit_open(ctx, &code_elem.tag.resolve(), &code_elem.attrs.0);
        ctx.buf.push_str("</");
        ctx.buf.push_str(&code_elem.tag.resolve());
        ctx.buf.push('>');
        return;
    }

    let ltokens = do_highlight(&raw, lang, ctx.light);
    let dtokens = do_highlight(&raw, lang, ctx.dark);

    emit_open(ctx, &code_elem.tag.resolve(), &code_elem.attrs.0);
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
    ctx.buf.push_str(&code_elem.tag.resolve());
    ctx.buf.push('>');
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

fn write_frame(ctx: &mut Ctx, frame: &HtmlFrame) {
    let css_inline = frame.css.to_inline().to_string();
    let svg = typst_svg::svg_in_html(
        &frame.inner, frame.text_size, false,
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
