mod asset;
mod content;
mod dom;
mod highlight;
mod navigation;
mod url;

use std::path::Path;

use anyhow::Result;
use comemo::Tracked;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

use crate::build::BuildWarning;
use crate::build::files::ProjectFiles;
use crate::build::output::PagePublication;
use crate::engine::content::SitePage;
use crate::foundation::config::{AssetsConfig, CssConfig, HighlightConfig};

use self::asset::{AssetProcessor, ComponentResources};
use self::content::ContentCapture;
use self::highlight::HighlightProcessor;
use self::navigation::NavigationProcessor;

/// The build-scoped transformation from compiled Typst HTML to a published page.
pub struct DocumentTransform<'a> {
    assets: AssetProcessor<'a>,
    highlight: HighlightProcessor,
}

impl<'a> DocumentTransform<'a> {
    pub fn new(
        project_files: Tracked<'a, ProjectFiles>,
        project_root: &Path,
        assets: &AssetsConfig,
        css: &CssConfig,
        highlight: &HighlightConfig,
        warnings: &mut Vec<BuildWarning>,
    ) -> Result<Self> {
        let assets = AssetProcessor::new(project_files, project_root, assets, css)?;
        let highlight = HighlightProcessor::new(highlight, project_files, warnings);
        Ok(Self { assets, highlight })
    }

    pub fn render(
        &mut self,
        mut document: HtmlDocument,
        mut page: PagePublication<'_>,
        pretty: bool,
    ) -> Result<SitePage> {
        let capture = ContentCapture::mark(&mut document)?;

        let stage = tracing::debug_span!("transform", message = "transformed document").entered();
        let resources = ComponentResources::collect(&document)?;
        {
            let mut navigation = NavigationProcessor;
            let mut processors: [&mut dyn Processor; 3] =
                [&mut self.assets, &mut navigation, &mut self.highlight];
            process_document(&mut document, &mut page, &mut processors)?;
        }
        resources.apply(&mut document, &mut page, &mut self.assets)?;
        drop(stage);

        let stage = tracing::debug_span!("encode", message = "encoded HTML").entered();
        let content = capture.extract(&mut document, pretty)?;
        let html = typst_html::html(&document, &typst_html::HtmlOptions { pretty })
            .map_err(|error| anyhow::anyhow!("HTML encoding failed: {error:?}"))?;
        let snapshot = SitePage {
            path: page.page_url_path(),
            html: html.as_str().into(),
            content,
        };
        page.add_html(html)?;
        drop(stage);
        Ok(snapshot)
    }
}

/// One participant in the shared document traversal.
trait Processor {
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
fn process_document(
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

enum WalkControl {
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
