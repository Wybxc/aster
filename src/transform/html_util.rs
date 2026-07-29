//! Extension-trait helpers for common [`HtmlElement`] operations.
//!
//! Usage: `use transform::html_util::HtmlElementExt;` — then call methods
//! directly on `HtmlElement` values.

use typst::ecow::EcoString;
use typst_html::{HtmlElement, HtmlNode};

/// Extension methods on [`HtmlElement`] that DRY up the frequently
/// repeated `elem.attrs.0.iter()` patterns.
pub trait HtmlElementExt {
    /// Check if the element's tag matches `tag`.
    fn is_tag(&self, tag: typst_html::HtmlTag) -> bool;

    /// Check whether an attribute with the given `name` exists and
    /// satisfies `predicate`.
    fn has_attr(&self, name: &str, predicate: impl Fn(&EcoString) -> bool) -> bool;

    /// Return a clone of the value of the first attribute matching `name`.
    fn get_attr(&self, name: &str) -> Option<EcoString>;

    /// Mutate every attribute whose name is `name` with `f`.
    fn update_attr(&mut self, name: &str, f: impl Fn(&mut EcoString));

    /// Return the first mutable child element whose tag matches `tag`.
    fn find_child_mut(&mut self, tag: typst_html::HtmlTag) -> Option<&mut HtmlElement>;
}

impl HtmlElementExt for HtmlElement {
    #[inline]
    fn is_tag(&self, tag: typst_html::HtmlTag) -> bool {
        self.tag == tag
    }

    fn has_attr(&self, name: &str, predicate: impl Fn(&EcoString) -> bool) -> bool {
        self.attrs
            .0
            .iter()
            .any(|(a, v)| a.resolve().as_str() == name && predicate(v))
    }

    fn get_attr(&self, name: &str) -> Option<EcoString> {
        self.attrs
            .0
            .iter()
            .find(|(a, _)| a.resolve().as_str() == name)
            .map(|(_, v)| v.clone())
    }

    fn update_attr(&mut self, name: &str, f: impl Fn(&mut EcoString)) {
        for (a, v) in self.attrs.0.make_mut().iter_mut() {
            if a.resolve().as_str() == name {
                f(v);
            }
        }
    }

    fn find_child_mut(&mut self, tag: typst_html::HtmlTag) -> Option<&mut HtmlElement> {
        for child in self.children.make_mut().iter_mut() {
            if let HtmlNode::Element(e) = child
                && e.tag == tag
            {
                return Some(e);
            }
        }
        None
    }
}
