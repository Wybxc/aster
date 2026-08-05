mod asset;
mod dom;
mod highlight;
mod navigation;
mod url;

use anyhow::Result;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::output::PagePublication;

pub use asset::{AssetProcessor, ComponentResources};
pub use highlight::HighlightProcessor;
pub use navigation::NavigationProcessor;

/// One participant in the shared document traversal.
pub trait Processor {
    /// Process one element and optionally suppress traversal into its children.
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl>;

    /// Apply transformations that must run after element traversal.
    fn end_document(
        &mut self,
        _document: &mut HtmlDocument,
        _page: &mut PagePublication<'_>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Run prepared processors over one document in caller-defined order.
pub fn process_document(
    doc: &mut HtmlDocument,
    page: &mut PagePublication<'_>,
    processors: &mut [&mut dyn Processor],
) -> Result<()> {
    walk_document(doc.root_mut(), &mut |element| {
        let mut control = WalkControl::Continue;
        for processor in processors.iter_mut() {
            if matches!(
                processor.process_element(element, page)?,
                WalkControl::SkipChildren
            ) {
                control = WalkControl::SkipChildren;
            }
        }
        Ok(control)
    })?;

    for processor in processors {
        processor.end_document(doc, page)?;
    }
    Ok(())
}

pub enum WalkControl {
    Continue,
    SkipChildren,
}

fn walk_document(
    element: &mut HtmlElement,
    transform: &mut impl FnMut(&mut HtmlElement) -> Result<WalkControl>,
) -> Result<()> {
    if matches!(transform(element)?, WalkControl::SkipChildren) {
        return Ok(());
    }
    for child in element.children.make_mut().iter_mut() {
        if let HtmlNode::Element(element) = child {
            walk_document(element, transform)?;
        }
    }
    Ok(())
}
