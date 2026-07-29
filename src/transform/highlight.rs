use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use syntect::easy::ScopeRegionIterator;
use syntect::highlighting::{Highlighter, Theme, ThemeSet};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};
use typst::ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst::syntax::{LinkedNode, Span, SyntaxNode, parse_code, parse_math};
use typst_html::{HtmlElement, HtmlNode};

use crate::config::HighlightConfig;
use crate::project::ProjectRoot;

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
            let span = if scope_name == "default" && *bits == 0 {
                // Plain tokens: no inline style, inherit parent styling.
                HtmlElement::new(typst_html::tag::span)
                    .with_children(eco_vec![HtmlNode::Text(txt.clone(), Span::detached())])
            } else {
                let mut style = eco_format!("color:var(--hl-{scope_name})");
                if bits & 1 != 0 {
                    style.push_str(";font-weight:bold");
                }
                if bits & 2 != 0 {
                    style.push_str(";font-style:italic");
                }
                HtmlElement::new(typst_html::tag::span)
                    .with_attr(typst_html::attr::style, style)
                    .with_children(eco_vec![HtmlNode::Text(txt.clone(), Span::detached())])
            };
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

// ---------------------------------------------------------------------------
// Theme resolution for CSS variable generation
// ---------------------------------------------------------------------------

/// Load a syntect theme by built-in name or file path (relative to `project_root`).
fn load_theme(name_or_path: &str, project_root: &Path) -> Result<Theme> {
    if let Some(theme) = THEMES.themes.get(name_or_path) {
        return Ok(theme.clone());
    }
    // Treat as file path — parse a single .tmTheme file.
    let path = project_root.join(name_or_path);
    let file = std::fs::File::open(&path)
        .with_context(|| format!("failed to open theme file {}", path.display()))?;
    let mut reader = std::io::BufReader::new(file);
    let theme = ThemeSet::load_from_reader(&mut reader)
        .with_context(|| format!("failed to load theme from {}", path.display()))?;
    Ok(theme)
}

/// Resolve highlight theme colours and write the CSS file.
///
/// Returns the relative (from output_dir) path for `<link>` injection,
/// or `None` if no [highlight] config is provided.
pub fn resolve_highlight_css(
    config: &HighlightConfig,
    project: &ProjectRoot,
) -> Result<Option<PathBuf>> {
    let light = load_theme(&config.themes.light, project.root())?;
    let dark = load_theme(&config.themes.dark, project.root())?;
    let light_h = Highlighter::new(&light);
    let dark_h = Highlighter::new(&dark);

    let _default_light = color_to_hex(
        light
            .settings
            .foreground
            .unwrap_or(syntect::highlighting::Color::BLACK),
    );
    let default_dark = color_to_hex(
        dark.settings
            .foreground
            .unwrap_or(syntect::highlighting::Color::BLACK),
    );

    // Collect unique scope names from theme selectors, resolve colours.
    let mut vars: Vec<(EcoString, String, String)> = Vec::new();
    for theme in [&light, &dark] {
        for scope_entry in &theme.scopes {
            for single in &scope_entry.scope.selectors {
                let Some(scope) = single.extract_single_scope() else {
                    continue;
                };

                let name = scope_css_name(std::slice::from_ref(&scope));
                if name == "default" {
                    continue;
                }
                // Deduplicate by scope name.
                if vars.iter().any(|(n, _, _)| *n == name) {
                    continue;
                }
                let light_fg = light_h
                    .style_for_stack(std::slice::from_ref(&scope))
                    .foreground;
                let dark_fg = dark_h
                    .style_for_stack(std::slice::from_ref(&scope))
                    .foreground;
                vars.push((name, color_to_hex(light_fg), color_to_hex(dark_fg)));
            }
        }
    }

    if vars.is_empty() {
        return Ok(None);
    }

    // Generate CSS.
    let mut css = String::from(":root,[data-theme=\"light\"]{\n");
    for (name, lc, _) in &vars {
        let _ = std::fmt::Write::write_fmt(&mut css, format_args!("  --hl-{name}:{lc};\n"));
    }
    css.push_str("}\n[data-theme=\"dark\"]{\n");
    for (name, _, dc) in &vars {
        if *dc != default_dark {
            let _ = std::fmt::Write::write_fmt(&mut css, format_args!("  --hl-{name}:{dc};\n"));
        }
    }
    css.push_str("}\n");

    // Write to output directory with content hash.
    let hash = format!("{:016x}", seahash::hash(css.as_bytes()));
    let filename = format!("hl.{hash}.css");
    let output = project.output_dir().join(&filename);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    std::fs::write(&output, &css)
        .with_context(|| format!("failed to write {}", output.display()))?;

    Ok(Some(PathBuf::from(filename)))
}

fn color_to_hex(c: syntect::highlighting::Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}
