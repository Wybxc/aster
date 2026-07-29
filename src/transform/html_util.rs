//! Convenience helpers for common [`HtmlElement`] operations.
//!
//! These DRY up the frequently repeated `elem.attrs.0.iter()` patterns
//! that appear in every processor.

use typst::ecow::EcoString;
use typst_html::{HtmlElement, HtmlNode};

/// Check if the element's tag matches `tag`.
#[inline]
pub fn is_tag(elem: &HtmlElement, tag: typst_html::HtmlTag) -> bool {
    elem.tag == tag
}

/// Check whether an attribute with the given `name` exists and satisfies
/// `predicate`.
pub fn has_attr(elem: &HtmlElement, name: &str, predicate: impl Fn(&EcoString) -> bool) -> bool {
    elem.attrs
        .0
        .iter()
        .any(|(a, v)| a.resolve().as_str() == name && predicate(v))
}

/// Return a clone of the value of the first attribute matching `name`.
pub fn get_attr(elem: &HtmlElement, name: &str) -> Option<EcoString> {
    elem.attrs
        .0
        .iter()
        .find(|(a, _)| a.resolve().as_str() == name)
        .map(|(_, v)| v.clone())
}

/// Mutate every attribute whose name is `name` with `f`.
///
/// Callers can also iterate `elem.attrs.0.make_mut()` directly when they
/// need finer control (e.g. matching several attribute names in one pass).
pub fn update_attr(elem: &mut HtmlElement, name: &str, f: impl Fn(&mut EcoString)) {
    for (a, v) in elem.attrs.0.make_mut().iter_mut() {
        if a.resolve().as_str() == name {
            f(v);
        }
    }
}

/// Iterate mutable children, returning the first [`HtmlElement`] child
/// whose tag matches `tag`.
///
/// ```ignore
/// if let Some(head) = find_child_mut(root, tag::head) {
///     head.children.push(...);
/// }
/// ```
pub fn find_child_mut<'a>(
    parent: &'a mut HtmlElement,
    tag: typst_html::HtmlTag,
) -> Option<&'a mut HtmlElement> {
    for child in parent.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child
            && e.tag == tag
        {
            return Some(e);
        }
    }
    None
}
