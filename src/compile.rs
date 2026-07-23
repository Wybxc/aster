use std::path::Path;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::SourceDiagnostic;
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
use typst::ecow::EcoVec;
use typst_html::{HtmlDocument, HtmlNode, HtmlOptions, HtmlTag};

use crate::world::{build_library, build_world};

/// Compile a single entry point and return the rendered HTML string.
///
/// The output is the inner content of `<body>` — the document shell
/// (`<html>`, `<head>`, `<body>`) is stripped away by modifying the
/// document tree before serialization.
pub fn run(entry: &Path, project_root: &Path) -> Result<String> {
    let library = build_library();
    let world = build_world(entry, project_root, &library);

    let warned = typst::compile::<HtmlDocument>(&world);
    emit_diags(&world, &warned.warnings);

    match warned.output {
        Ok(mut doc) => {
            extract_body(&mut doc);
            let html = typst_html::html(&doc, &HtmlOptions::default())
                .map_err(|_| anyhow::anyhow!("failed to encode HTML"))?;
            Ok(strip_shell(&html))
        }
        Err(errors) => {
            emit_diags(&world, &errors);
            bail!("compilation failed")
        }
    }
}

/// Replaces the `<html>` element's children with the `<body>`'s children
/// and replaces its tag with `x`, so that the serialized output has a
/// trivial wrapper we can strip mechanically.
fn extract_body(doc: &mut HtmlDocument) {
    let root = doc.root_mut();
    let mut body_children: Option<EcoVec<HtmlNode>> = None;

    for child in &root.children {
        if let HtmlNode::Element(e) = child {
            if e.tag == typst_html::tag::body {
                body_children = Some(e.children.clone());
                break;
            }
        }
    }

    if let Some(children) = body_children {
        root.children = children;
        root.attrs.0.clear();
        root.tag = HtmlTag::intern("x").expect("'x' is a valid tag name");
    }
}

/// Strip the `<!DOCTYPE html><x>` prefix and `</x>` suffix left by
/// [`extract_body`].
fn strip_shell(html: &str) -> String {
    const PREFIX: &str = "<!DOCTYPE html><x>";
    const SUFFIX: &str = "</x>";

    let start = html.find(PREFIX).map(|i| i + PREFIX.len()).unwrap_or(0);
    let end = html.rfind(SUFFIX).unwrap_or(html.len());
    html[start..end].to_owned()
}

fn emit_diags(world: &impl DiagnosticWorld, diags: &[SourceDiagnostic]) {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if diagnostics::emit(&mut writer, world, diags.iter(), DiagnosticFormat::Human).is_err() {
        for diag in diags {
            eprintln!("error: {diag:?}");
        }
    }
}
