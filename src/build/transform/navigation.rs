use anyhow::Result;
use typst_html::HtmlElement;

use crate::build::output::PagePublication;

use super::dom::HtmlElementExt;
use super::url::{UrlReference, classify_url};
use super::{Processor, WalkControl};

/// Rewrites site-root navigation into URLs relative to each output page.
pub struct NavigationProcessor;

impl Processor for NavigationProcessor {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        let attribute = if (element.is_tag(typst_html::tag::a)
            || element.is_tag(typst_html::tag::area))
            && element.get_attr("download").is_none()
        {
            "href"
        } else if element.is_tag(typst_html::tag::form) {
            "action"
        } else {
            return Ok(WalkControl::Continue);
        };

        let Some(reference) = element.get_attr(attribute) else {
            return Ok(WalkControl::Continue);
        };
        let UrlReference::Rooted { path, suffix } = classify_url(&reference) else {
            return Ok(WalkControl::Continue);
        };
        let mut url = page.site_reference(path);
        url.push_str(suffix);
        element.update_attr(attribute, move |value| *value = url);
        Ok(WalkControl::Continue)
    }
}
