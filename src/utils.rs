//! Utility helpers — HTML element attributes, text extraction, content hashes,
//! and colour formatting.

use typst::ecow::EcoString;
use typst_html::{HtmlElement, HtmlNode};

/// Format a syntect [`Color`](syntect::highlighting::Color) as `#rrggbb`.
pub fn color_to_hex(c: syntect::highlighting::Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Compute a compact 64-bit content fingerprint for generated asset URLs.
pub fn content_hash(data: &[u8]) -> String {
    format!("{:016x}", seahash::hash(data))
}

/// Extension methods on [`HtmlElement`] for common attribute and text access.
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

    /// Collect the text of all descendant `HtmlNode::Text` nodes,
    /// inserting `\n` for `<br>` elements so the result reflects
    /// multi-line content in source order.
    fn collect_text(&self) -> String;
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

    fn collect_text(&self) -> String {
        fn collect_impl(elem: &HtmlElement, out: &mut String) {
            for child in &elem.children {
                match child {
                    HtmlNode::Text(t, _) => out.push_str(t.as_str()),
                    HtmlNode::Element(e) if e.tag == typst_html::tag::br => out.push('\n'),
                    HtmlNode::Element(e) => collect_impl(e, out),
                    _ => {}
                }
            }
        }

        let mut out = String::new();
        collect_impl(self, &mut out);
        out
    }
}
