use std::path::Path;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::SourceDiagnostic;
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};

use crate::world::{build_library, build_world};

/// Compile a single entry point and return the rendered HTML string.
///
/// The output is the inner content of `<body>` — the document shell
/// (`<html>`, `<head>`, `<body>`) is stripped away. Only typst's
/// `html.html` / `html.body` / etc. produce those wrappers implicitly.
pub fn run(entry: &Path, project_root: &Path) -> Result<String> {
    let library = build_library();
    let world = build_world(entry, project_root, &library);

    let warned = typst::compile::<typst_html::HtmlDocument>(&world);
    emit_diags(&world, &warned.warnings);

    let result = warned
        .output
        .and_then(|doc| typst_html::html(&doc, &typst_html::HtmlOptions::default()));

    match result {
        Ok(html) => Ok(strip_html_shell(&html)),
        Err(errors) => {
            emit_diags(&world, &errors);
            bail!("compilation failed")
        }
    }
}

/// Strip the `<html>`, `<head>`, and `<body>` wrapper, returning only the
/// inner body content.
fn strip_html_shell(html: &str) -> String {
    let start = html.find("<body").map(|i| {
        // Advance past `<body` and any attributes until `>`.
        let rest = &html[i + 5..];
        let close = rest.find('>').unwrap_or(0);
        i + 5 + close + 1
    }).unwrap_or(0);

    let end = html.rfind("</body>").unwrap_or(html.len());

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
