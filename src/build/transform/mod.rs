mod css;
pub(super) mod dom;
mod highlight;
mod image;

use anyhow::{Result, ensure};
use comemo::Tracked;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::output::{AssetPath, PagePublication};
use crate::foundation::Project;
use crate::foundation::config::HighlightConfig;
use crate::foundation::files::ProjectFiles;

pub(crate) fn compute_highlight_css(
    config: &HighlightConfig,
    project: &Project,
    project_files: Tracked<ProjectFiles>,
) -> Result<Option<String>> {
    highlight::compute_highlight_css(config, project, project_files)
}

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
    let highlight_url = highlight_css
        .map(|asset| page.reference(asset))
        .transpose()?;
    let mut stylesheet_injected = highlight_url.is_none();

    walk_document(doc.root_mut(), &mut |element| {
        css::process_element(element, page, project_files)?;
        image::process_element(element, page)?;
        let control = highlight::process_element(element);

        if element.tag == typst_html::tag::head
            && let Some(url) = &highlight_url
        {
            highlight::inject_stylesheet(element, url);
            stylesheet_injected = true;
        }
        Ok(control)
    })?;

    ensure!(
        stylesheet_injected,
        "highlight CSS configured but found no <head> element"
    );
    Ok(())
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
