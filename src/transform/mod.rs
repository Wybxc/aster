pub mod css;
pub mod highlight;
pub mod image;

use std::path::PathBuf;

use anyhow::Result;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::project::ProjectRoot;

/// Signal returned by the callback passed to [`walk_mut`] to control
/// whether children of the current element should be visited.
pub enum WalkControl {
    Continue,
    SkipChildren,
}

/// A processor that transforms the document as a whole — CSS bundling,
/// image extraction, syntax highlighting, etc.
pub trait ElementProcessor {
    fn process(&self, doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()>;
}

/// Recursively visit every descendant `HtmlElement` depth-first (mutable).
///
/// Processors that need element-level access can use this instead of
/// writing their own traversal.  Capture external state (such as
/// [`ProcessingContext`]) in the closure.
pub fn walk_mut(
    elem: &mut HtmlElement,
    f: &mut impl FnMut(&mut HtmlElement) -> Result<WalkControl>,
) -> Result<()> {
    if matches!(f(elem)?, WalkControl::SkipChildren) {
        return Ok(());
    }
    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk_mut(e, f)?;
        }
    }
    Ok(())
}

/// Per-page context for the document processing pipeline.
pub struct ProcessingContext<'a> {
    pub project: &'a ProjectRoot,
    pub page_path: PathBuf,
    pub hl_css_path: Option<PathBuf>,
    /// Name of the syntect theme used for token-level highlighting
    /// (determines which scopes get non-default classification).  When
    /// `None`, a built-in default is used.
    pub highlight_theme: Option<String>,
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
