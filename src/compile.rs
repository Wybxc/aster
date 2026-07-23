use std::path::Path;

use typst::diag::SourceDiagnostic;
use typst::ecow::EcoVec;

use crate::world::{build_library, build_world, CompileWorld};

/// Carries a failed compilation together with its world so the caller
/// can print rich diagnostics through [`DiagnosticWorld`].
pub struct CompileError {
    pub world: CompileWorld,
    pub diagnostics: EcoVec<SourceDiagnostic>,
}

/// Compile a single entry point and return the rendered HTML string.
///
/// On failure the world is returned alongside the diagnostics so the
/// caller can format them with `codespan-reporting`.
pub fn run(entry: &Path, project_root: &Path) -> Result<String, CompileError> {
    let library = build_library();
    let world = build_world(entry, project_root, &library);

    let warned = typst::compile::<typst_html::HtmlDocument>(&world);
    match warned.output {
        Ok(doc) => match typst_html::html(&doc, &typst_html::HtmlOptions::default()) {
            Ok(html) => Ok(html),
            Err(e) => panic!("failed to encode HTML: {e:?}"),
        },
        Err(diagnostics) => Err(CompileError { world, diagnostics }),
    }
}
