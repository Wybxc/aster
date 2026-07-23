use std::path::Path;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst_kit::diagnostics::DiagnosticFormat;

use crate::world::{build_library, build_world};

/// Compile a single entry point and return the rendered HTML string.
///
/// Diagnostics for compilation errors are printed to stderr automatically.
/// The caller only needs to distinguish success from failure.
pub fn run(entry: &Path, project_root: &Path) -> Result<String> {
    let library = build_library();
    let world = build_world(entry, project_root, &library);

    let warned = typst::compile::<typst_html::HtmlDocument>(&world);
    match warned.output {
        Ok(doc) => typst_html::html(&doc, &typst_html::HtmlOptions::default())
            .map_err(|_| anyhow::anyhow!("failed to encode HTML")),
        Err(diagnostics) => {
            // Session-level diagnostics were already emitted by the compiler.
            // Print source-level diagnostics via codespan-reporting.
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            if typst_kit::diagnostics::emit(
                &mut writer,
                &world,
                &diagnostics,
                DiagnosticFormat::Human,
            )
            .is_err()
            {
                for diag in &diagnostics {
                    eprintln!("error: {diag:?}");
                }
            }
            bail!("compilation failed")
        }
    }
}
