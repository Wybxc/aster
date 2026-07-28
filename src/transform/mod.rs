pub mod css;
pub mod highlight;
pub mod image;

use std::path::PathBuf;

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
pub fn walk_mut<'a, E>(
    elem: &mut HtmlElement,
    ctx: &ProcessingContext<'a>,
    f: &mut impl FnMut(&mut HtmlElement, &ProcessingContext<'a>) -> Result<WalkControl, E>,
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
    fn process(&self, elem: &mut HtmlElement, ctx: &ProcessingContext<'_>) -> Result<WalkControl>;
}

/// Run a set of [`ElementProcessor`]s on the document in a single traversal.
pub fn process_all(
    doc: &mut HtmlDocument,
    ctx: &ProcessingContext<'_>,
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
pub struct ProcessingContext<'a> {
    pub project: &'a ProjectRoot,
    pub page_path: PathBuf,
}

impl ProcessingContext<'_> {
    pub fn new(project: &ProjectRoot, page_path: PathBuf) -> ProcessingContext<'_> {
        ProcessingContext { project, page_path }
    }

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

/// Run every built-in processor in a single DOM traversal.
pub fn process_document(doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()> {
    let css = css::CssProcessor;
    let img = image::ImageProcessor;
    let hl = highlight::HighlightProcessor;

    let processors: [&dyn ElementProcessor; 3] = [&css, &img, &hl];
    process_all(doc, ctx, &processors)
}
