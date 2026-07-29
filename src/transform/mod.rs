pub mod css;
pub mod highlight;
pub mod image;

use std::path::PathBuf;

use anyhow::Result;
use typst_html::HtmlDocument;

use crate::project::ProjectRoot;

pub use crate::utils::{WalkControl, walk_mut};

/// A processor that transforms the document as a whole — CSS bundling,
/// image extraction, syntax highlighting, etc.
pub trait ElementProcessor {
    fn process(&self, doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()>;
}

/// Per-page context for the document processing pipeline.
pub struct ProcessingContext<'a> {
    pub project: &'a ProjectRoot,
    pub page_path: PathBuf,
    pub hl_css_path: Option<PathBuf>,
}

impl ProcessingContext<'_> {
    pub fn src_dir(&self) -> PathBuf {
        self.project.src_dir()
    }

    pub fn output_dir(&self) -> PathBuf {
        self.project.output_dir()
    }

    /// Subdirectory of `src_dir` where the current template lives, derived from
    /// the page's output path (e.g. `blog` for page `dist/blog/page.html`).
    pub fn template_subdir(&self) -> PathBuf {
        let output = self.output_dir();
        self.page_path
            .parent()
            .and_then(|p| p.strip_prefix(&output).ok())
            .map(|p| p.to_path_buf())
            .unwrap_or_default()
    }
}

/// Run every built-in processor in order.
pub fn process_document(doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()> {
    for p in &[
        &css::CssProcessor as &dyn ElementProcessor,
        &image::ImageProcessor,
        &highlight::HighlightProcessor,
    ] {
        p.process(doc, ctx)?;
    }
    Ok(())
}
