pub mod css;
pub mod highlight;
pub mod image;

use std::path::{Path, PathBuf};

use anyhow::Result;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::project::ProjectRoot;

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
    pub hl_css_path: Option<PathBuf>,
    /// Additional element processors to run after the built-in set.
    /// The default is empty — only built-in processors run.
    pub extra_processors: &'a [&'a dyn ElementProcessor],
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

/// Built-in processors run on every page, in this order.
const BUILTIN_PROCESSORS: &[&dyn ElementProcessor] = &[
    &css::CssProcessor,
    &image::ImageProcessor,
    &highlight::HighlightProcessor,
];

/// Run built-in processors followed by any extra processors from the
/// context, then inject the highlight CSS `<link>` if configured.
pub fn process_document(doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()> {
    let all: Vec<&dyn ElementProcessor> = BUILTIN_PROCESSORS
        .iter()
        .copied()
        .chain(ctx.extra_processors.iter().copied())
        .collect();
    process_all(doc, ctx, &all)?;

    if let Some(ref hl_css) = ctx.hl_css_path {
        inject_hl_link(doc, hl_css);
    }
    Ok(())
}

/// Inject `<link rel="stylesheet" href="...">` into `<head>`.
fn inject_hl_link(doc: &mut HtmlDocument, href: &Path) {
    use typst_html::{attr, tag};
    for child in doc.root_mut().children.make_mut().iter_mut() {
        if let HtmlNode::Element(head) = child {
            if head.tag == tag::head {
                let link = HtmlElement::new(tag::link)
                    .with_attr(attr::rel, "stylesheet")
                    .with_attr(attr::href, href.to_string_lossy().as_ref());
                head.children.push(HtmlNode::Element(link));
                return;
            }
        }
    }
}
