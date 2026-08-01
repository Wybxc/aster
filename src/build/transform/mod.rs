mod css;
pub(super) mod dom;
mod highlight;
mod image;

use anyhow::Result;
use comemo::Tracked;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::output::{AssetPath, PagePublication};
use crate::foundation::files::ProjectFiles;
pub use highlight::compute_highlight_css;

/// Transform one compiled document and register every generated page asset.
///
/// Traversal order, element policy, and highlight stylesheet injection remain
/// internal so callers provide only the document and publication context.
pub(crate) fn process_document(
    doc: &mut HtmlDocument,
    page: &mut PagePublication<'_>,
    project_files: Tracked<ProjectFiles>,
    highlight_css: Option<&AssetPath>,
) -> Result<()> {
    if let Some(asset) = highlight_css {
        let url = page.reference(asset)?;
        highlight::attach_stylesheet(doc, url);
    }

    walk_document(doc.root_mut(), &mut |element| {
        css::process_element(element, page, project_files)?;
        image::process_element(element, page)?;
        Ok(highlight::process_element(element))
    })
}

pub(super) enum WalkControl {
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
