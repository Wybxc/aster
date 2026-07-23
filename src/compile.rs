use std::path::Path;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::SourceDiagnostic;
use typst_kit::diagnostics::DiagnosticFormat;

use crate::world::{build_library, build_world};

/// Compile a single entry point and return the rendered HTML string.
///
/// Diagnostics (errors + warnings) are printed to stderr automatically.
/// The caller only needs to distinguish success from failure.
pub fn run(entry: &Path, project_root: &Path) -> Result<String> {
    let library = build_library();
    let world = build_world(entry, project_root, &library);

    let warned = typst::compile::<typst_html::HtmlDocument>(&world);

    let mut all_diags: Vec<&SourceDiagnostic> = warned.warnings.iter().collect();

    let result = warned
        .output
        .and_then(|doc| typst_html::html(&doc, &typst_html::HtmlOptions::default()));

    match result {
        Ok(html) => {
            if !all_diags.is_empty() {
                emit_diags(&world, &all_diags);
            }
            Ok(html)
        }
        Err(errors) => {
            all_diags.extend(errors.iter());
            emit_diags(&world, &all_diags);
            bail!("compilation failed")
        }
    }
}

fn emit_diags(world: &impl typst_kit::diagnostics::DiagnosticWorld, diags: &[&SourceDiagnostic]) {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if typst_kit::diagnostics::emit(&mut writer, world, diags.iter().copied(), DiagnosticFormat::Human).is_err() {
        for diag in diags {
            eprintln!("error: {diag:?}");
        }
    }
}
