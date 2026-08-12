//! Capture of the final HTML represented by `<aster-content>`.

use anyhow::{Context, Result, bail, ensure};
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode, HtmlSliceExt};

use crate::engine::content::SiteContent;

use super::dom::HtmlElementExt;

const MARKER: &str = "data-aster-content-root";

/// A content element marked before document mutation and extracted afterward.
pub struct ContentCapture {
    marked: bool,
}

impl ContentCapture {
    pub fn mark(document: &mut HtmlDocument) -> Result<Self> {
        let Some(position) = labelled_content_position(document)? else {
            return Ok(Self { marked: false });
        };
        ensure!(
            !contains_attr(document.root(), MARKER),
            "{MARKER} is reserved by Aster"
        );
        let element = element_at_position_mut(document.root_mut(), &position)
            .context("<aster-content> label must identify an HTML element")?;
        let marker = typst_html::HtmlAttr::intern(MARKER)
            .map_err(|error| anyhow::anyhow!("invalid internal content marker: {error}"))?;
        element.attrs.push(marker, "");
        Ok(Self { marked: true })
    }

    pub fn extract(self, document: &mut HtmlDocument, pretty: bool) -> Result<Option<SiteContent>> {
        if !self.marked {
            return Ok(None);
        }
        let element = take_marked_content(document.root_mut())
            .context("<aster-content> element was removed by document processing")?;
        let text = element.inner_text();
        let mut fragment = document.clone();
        *fragment.root_mut() = element;
        let html = typst_html::html(&fragment, &typst_html::HtmlOptions { pretty })
            .map_err(|error| anyhow::anyhow!("HTML content encoding failed: {error:?}"))?;
        let html = html
            .strip_prefix("<!DOCTYPE html>")
            .unwrap_or(&html)
            .trim_start_matches('\n')
            .into();
        Ok(Some(SiteContent { html, text }))
    }
}

fn labelled_content_position(document: &HtmlDocument) -> Result<Option<Vec<usize>>> {
    let selector = Selector::Label(
        Label::construct("aster-content".into()).expect("content label is non-empty"),
    );
    let mut matches = document.introspector().query(&selector).into_iter();
    let Some(content) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        bail!("page must contain at most one <aster-content> element");
    }
    let location = content
        .location()
        .context("<aster-content> element has no location")?;
    let position = document
        .introspector()
        .position(location)
        .context("<aster-content> element has no HTML position")?;
    Ok(Some(position.element().collect()))
}

fn contains_attr(root: &HtmlElement, name: &str) -> bool {
    root.get_attr(name).is_some()
        || root.children.iter().any(
            |child| matches!(child, HtmlNode::Element(element) if contains_attr(element, name)),
        )
}

fn element_at_position_mut<'a>(
    root: &'a mut HtmlElement,
    position: &[usize],
) -> Option<&'a mut HtmlElement> {
    let Some((&target, rest)) = position.split_first() else {
        return Some(root);
    };
    let child_index = root.children.iter_with_dom_indices().enumerate().find_map(
        |(child_index, (node, dom_index))| {
            matches!(node, HtmlNode::Element(_))
                .then_some((child_index, dom_index))
                .filter(|(_, dom_index)| *dom_index == target)
                .map(|(child_index, _)| child_index)
        },
    )?;
    let HtmlNode::Element(child) = &mut root.children.make_mut()[child_index] else {
        unreachable!("selected child was checked as an element")
    };
    element_at_position_mut(child, rest)
}

fn take_marked_content(root: &mut HtmlElement) -> Option<HtmlElement> {
    if let Some(index) = root
        .attrs
        .0
        .iter()
        .position(|(attr, _)| attr.resolve().as_str() == MARKER)
    {
        root.attrs.0.remove(index);
        return Some(root.clone());
    }
    root.children.make_mut().iter_mut().find_map(|child| {
        let HtmlNode::Element(element) = child else {
            return None;
        };
        take_marked_content(element)
    })
}
