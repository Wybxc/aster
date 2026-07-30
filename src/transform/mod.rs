pub mod css;
pub mod highlight;
pub mod image;

use std::path::PathBuf;

use anyhow::Result;
use typst_html::HtmlDocument;

use crate::utils::Asset;

pub use crate::utils::WalkControl;

/// A processor that transforms the document and returns generated assets.
///
/// The returned assets are collected by the caller (e.g. [`process_document`])
/// and batch-written via [`AssetCollector`] after all pages are rendered.
pub trait ElementProcessor {
    fn process(&self, doc: &mut HtmlDocument, ctx: &ProcessingContext) -> Result<Vec<Asset>>;
}

/// Per-page context for the document processing pipeline.
pub struct ProcessingContext {
    pub page_path: PathBuf,
    pub hl_css_path: Option<PathBuf>,
    src_dir: PathBuf,
    output_dir: PathBuf,
}

impl ProcessingContext {
    pub fn new(
        page_path: PathBuf,
        hl_css_path: Option<PathBuf>,
        src_dir: PathBuf,
        output_dir: PathBuf,
    ) -> Self {
        Self {
            page_path,
            hl_css_path,
            src_dir,
            output_dir,
        }
    }

    pub fn src_dir(&self) -> PathBuf {
        self.src_dir.clone()
    }

    pub fn output_dir(&self) -> PathBuf {
        self.output_dir.clone()
    }

    /// Subdirectory of `src_dir` where the current template lives, derived from
    /// the page's output path (e.g. `blog` for page `dist/blog/page.html`).
    pub fn template_subdir(&self) -> PathBuf {
        let output = &self.output_dir;
        self.page_path
            .parent()
            .and_then(|p| p.strip_prefix(output).ok())
            .map(|p| p.to_path_buf())
            .unwrap_or_default()
    }
}

/// Run every built-in processor in order, returning all generated assets.
pub fn process_document(doc: &mut HtmlDocument, ctx: &ProcessingContext) -> Result<Vec<Asset>> {
    let mut all_assets = Vec::new();
    for p in &[
        &css::CssProcessor as &dyn ElementProcessor,
        &image::ImageProcessor,
        &highlight::HighlightProcessor,
    ] {
        all_assets.extend(p.process(doc, ctx)?);
    }
    Ok(all_assets)
}
