pub mod css;
pub mod highlight;
pub mod image;

use std::path::{Path, PathBuf};

use anyhow::Result;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::project::ProjectRoot;

/// Recursively visit every descendant `HtmlElement` depth-first (immutable).
pub fn walk(elem: &HtmlElement, f: &mut dyn FnMut(&HtmlElement)) {
    f(elem);
    for child in &elem.children {
        if let HtmlNode::Element(e) = child {
            walk(e, f);
        }
    }
}

/// Signal returned by the callback passed to [`walk_mut`] to control
/// whether children of the current element should be visited.
pub enum WalkControl {
    Continue,
    SkipChildren,
}

/// Recursively visit every descendant `HtmlElement` depth-first (mutable).
pub fn walk_mut<E>(
    elem: &mut HtmlElement,
    ctx: &ProcessingContext,
    f: &mut impl FnMut(&mut HtmlElement, &ProcessingContext) -> Result<WalkControl, E>,
) -> Result<(), E> {
    if matches!(f(elem, ctx)?, WalkControl::SkipChildren) {
        return Ok(());
    }
    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk_mut(e, ctx, f)?;
        }
    }
    Ok(())
}

/// A processor that matches and transforms individual [`HtmlElement`] nodes.
pub trait ElementProcessor {
    fn matches(&self, elem: &HtmlElement) -> bool;
    fn process(&self, elem: &mut HtmlElement, ctx: &ProcessingContext) -> Result<WalkControl>;
}

/// Run a set of [`ElementProcessor`]s on the document in a single traversal.
pub fn process_all(
    doc: &mut HtmlDocument,
    ctx: &ProcessingContext,
    processors: &[&dyn ElementProcessor],
) -> Result<()> {
    walk_mut(doc.root_mut(), ctx, &mut |elem, ctx| {
        for p in processors {
            if p.matches(elem) && matches!(p.process(elem, ctx)?, WalkControl::SkipChildren) {
                return Ok(WalkControl::SkipChildren);
            }
        }
        Ok(WalkControl::Continue)
    })
}

/// Per-page context for the document processing pipeline.
pub struct ProcessingContext {
    pub src_dir: PathBuf,
    pub dist_dir: PathBuf,
    /// Absolute output path of the current page (e.g. `dist/blog/hello-world.html`).
    pub page_path: PathBuf,
}

impl ProcessingContext {
    /// Build context for a specific page from the project root.
    pub fn new(project: &ProjectRoot, page_path: PathBuf) -> Self {
        Self {
            src_dir: project.src_dir(),
            dist_dir: project.output_dir(),
            page_path,
        }
    }

    /// Subdirectory of `src_dir` where the current template lives, derived from
    /// the page's output path (e.g. `blog` for page `dist/blog/page.html`).
    pub fn template_subdir(&self) -> &Path {
        self.page_path
            .parent()
            .and_then(|p| p.strip_prefix(&self.dist_dir).ok())
            .unwrap_or(Path::new(""))
    }
}

/// Run every built-in processor in a single DOM traversal.
pub fn process_document(doc: &mut HtmlDocument, ctx: &ProcessingContext) -> Result<()> {
    let css = css::CssProcessor;
    let img = image::ImageProcessor;
    let hl = highlight::HighlightProcessor;

    let processors: [&dyn ElementProcessor; 3] = [&css, &img, &hl];
    process_all(doc, ctx, &processors)
}
