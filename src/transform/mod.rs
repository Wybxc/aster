pub mod css;
pub mod highlight;
pub mod image;

use anyhow::Result;
use comemo::Tracked;
use typst_html::HtmlDocument;

use crate::compile::ProjectFiles;
use crate::output::{AssetPath, PagePublication};

pub use crate::utils::WalkControl;

/// A document transform adapter. Output policy remains in [`PagePublication`].
pub trait ElementProcessor {
    fn process(&self, doc: &mut HtmlDocument, page: &mut PagePublication<'_>) -> Result<()>;
}

/// Run every built-in transform adapter in order.
pub fn process_document(
    doc: &mut HtmlDocument,
    page: &mut PagePublication<'_>,
    project_files: Tracked<ProjectFiles>,
    highlight_css: Option<&AssetPath>,
) -> Result<()> {
    css::CssProcessor::new(project_files).process(doc, page)?;
    image::ImageProcessor.process(doc, page)?;
    highlight::HighlightProcessor.process(doc, page)?;
    if let Some(asset) = highlight_css {
        highlight::inject_stylesheet(doc, page, asset)?;
    }
    Ok(())
}
