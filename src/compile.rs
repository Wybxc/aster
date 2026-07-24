use std::path::Path;

use anyhow::Result;
use termcolor::{ColorChoice, StandardStream};
use typst::diag::SourceDiagnostic;
use typst::ecow::EcoVec;
use typst::foundations::Dict;
use typst_html::{HtmlDocument, HtmlNode, HtmlOptions, HtmlTag};
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};

use crate::highlight;
use crate::world::{build_library, build_world};

// ---------------------------------------------------------------------------
// Low-level: compile a single file into an HtmlDocument (shared by pages and
// content entries).  Diagnostics are printed to stderr automatically.
// ---------------------------------------------------------------------------

pub fn compile_document(
    entry: &Path,
    project_root: &Path,
    inputs: Dict,
) -> Result<HtmlDocument, String> {
    let library = build_library(inputs);
    let world = build_world(entry, project_root, &library);

    let warned = typst::compile::<HtmlDocument>(&world);
    emit_diags(&world, &warned.warnings);

    match warned.output {
        Ok(doc) => Ok(doc),
        Err(errors) => {
            emit_diags(&world, &errors);
            Err("compilation failed".to_owned())
        }
    }
}

// ---------------------------------------------------------------------------
// High-level: compile a page → stripped HTML string
// ---------------------------------------------------------------------------

pub fn run(entry: &Path, project_root: &Path, inputs: Dict) -> Result<String> {
    let mut doc = compile_document(entry, project_root, inputs)
        .map_err(|_| anyhow::anyhow!("compilation failed"))?;

    extract_body(&mut doc);
    let style_css = highlight::apply_to_doc(&mut doc, None, None);
    let body = typst_html::html(&doc, &HtmlOptions::default())
        .map_err(|_| anyhow::anyhow!("failed to encode HTML"))?;
    Ok(strip_shell(&body) + &style_css)
}

// ---------------------------------------------------------------------------
// Body extraction helpers
// ---------------------------------------------------------------------------

/// Replaces the `<html>` element's children with the `<body>`'s children
/// and replaces its tag with `aster-body`, so that the serialized output has a
/// trivial wrapper we can strip mechanically.
fn extract_body(doc: &mut HtmlDocument) {
    let root = doc.root_mut();
    let mut body_children: Option<EcoVec<HtmlNode>> = None;

    for child in &root.children {
        if let HtmlNode::Element(e) = child
            && e.tag == typst_html::tag::body
        {
            body_children = Some(e.children.clone());
            break;
        }
    }

    if let Some(children) = body_children {
        root.children = children;
        root.attrs.0.clear();
        root.tag = HtmlTag::intern("aster-body").expect("'aster-body' is a valid tag name");
    }
}

/// Strip the `<!DOCTYPE html><aster-body>` prefix and `</aster-body>` suffix
/// left by [`extract_body`].
fn strip_shell(html: &str) -> String {
    const PREFIX: &str = "<!DOCTYPE html><aster-body>";
    const SUFFIX: &str = "</aster-body>";

    let start = html.find(PREFIX).map(|i| i + PREFIX.len()).unwrap_or(0);
    let end = html.rfind(SUFFIX).unwrap_or(html.len());
    html[start..end].to_owned()
}

// ---------------------------------------------------------------------------
// Diagnostic printing
// ---------------------------------------------------------------------------

fn emit_diags(world: &impl DiagnosticWorld, diags: &[SourceDiagnostic]) {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if diagnostics::emit(&mut writer, world, diags.iter(), DiagnosticFormat::Human).is_err() {
        for diag in diags {
            eprintln!("error: {diag:?}");
        }
    }
}
