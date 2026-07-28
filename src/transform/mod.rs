pub mod css;
pub mod highlight;
pub mod image;

use std::path::PathBuf;

use anyhow::Result;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

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
    f: &mut impl FnMut(&mut HtmlElement) -> Result<WalkControl, E>,
) -> Result<(), E> {
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

/// A processor that matches and transforms individual [`HtmlElement`] nodes.
pub trait ElementProcessor {
    fn matches(&self, elem: &HtmlElement) -> bool;
    fn process(&self, elem: &mut HtmlElement) -> Result<WalkControl>;
}

/// Run a set of [`ElementProcessor`]s on the document in a single traversal.
pub fn process_all(doc: &mut HtmlDocument, processors: &[&dyn ElementProcessor]) -> Result<()> {
    walk_mut(doc.root_mut(), &mut |elem| {
        for p in processors {
            if p.matches(elem) && matches!(p.process(elem)?, WalkControl::SkipChildren) {
                return Ok(WalkControl::SkipChildren);
            }
        }
        Ok(WalkControl::Continue)
    })
}

// ---------------------------------------------------------------------------
// Processing context
// ---------------------------------------------------------------------------

/// Per-page context for the document processing pipeline.
pub struct ProcessingContext {
    pub src_dir: PathBuf,
    pub dist_dir: PathBuf,
    /// Subdirectory of `src_dir` containing the current template
    /// (e.g. `"blog"` for `src/blog/[slug].typ`).
    pub template_subdir: PathBuf,
    /// Absolute output path of the current page
    /// (e.g. `dist/blog/hello-world.html`).
    pub page_path: PathBuf,
}

/// Run every built-in processor in a single DOM traversal.
pub fn process_document(doc: &mut HtmlDocument, ctx: &ProcessingContext) -> Result<()> {
    let css = css::CssProcessor {
        src_dir: ctx.src_dir.clone(),
        dist_dir: ctx.dist_dir.clone(),
        template_subdir: ctx.template_subdir.clone(),
        page_path: ctx.page_path.clone(),
    };
    let img = image::ImageProcessor {
        dist_dir: ctx.dist_dir.clone(),
    };
    let hl = highlight::HighlightProcessor;

    let processors: [&dyn ElementProcessor; 3] = [&css, &img, &hl];
    process_all(doc, &processors)
}
