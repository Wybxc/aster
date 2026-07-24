use std::path::Path;

use anyhow::Result;
use termcolor::{ColorChoice, StandardStream};
use typst::diag::SourceDiagnostic;
use typst::foundations::Dict;
use typst_html::{HtmlDocument, HtmlOptions};
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
// High-level: compile a page → serialized HTML string.
// ---------------------------------------------------------------------------

pub fn run(entry: &Path, project_root: &Path, inputs: Dict) -> Result<String> {
    let mut doc = compile_document(entry, project_root, inputs)
        .map_err(|_| anyhow::anyhow!("compilation failed"))?;

    let style_css = highlight::rehighlight(&mut doc);

    let raw = typst_html::html(&doc, &HtmlOptions::default())
        .map_err(|_| anyhow::anyhow!("failed to encode HTML"))?;

    let html = raw
        .strip_prefix("<!DOCTYPE html>")
        .unwrap_or(&raw)
        .to_owned();

    Ok(html + &style_css)
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
