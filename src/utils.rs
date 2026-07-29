//! Utility helpers — DOM traversal, HTML element extension methods, path
//! utilities, colour formatting, etc.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use typst::ecow::EcoString;
use typst_html::{HtmlElement, HtmlNode};

/// Signal returned by the callback passed to [`walk_mut`] to control
/// whether children of the current element should be visited.
pub enum WalkControl {
    Continue,
    SkipChildren,
}

/// Resolve `..` components in a path without requiring the file to exist.
pub fn normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => result.push(comp),
        }
    }
    result
}

/// Compute a relative path from `from` (a directory) to `to` (a file).
pub fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut result = PathBuf::new();
    for _ in common..from.len() {
        result.push("..");
    }
    for comp in &to[common..] {
        result.push(comp);
    }
    result
}

/// Format a syntect [`Color`](syntect::highlighting::Color) as `#rrggbb`.
pub fn color_to_hex(c: syntect::highlighting::Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Compute a content-hash hex string (seahash, first 16 hex chars).
pub fn content_hash(data: &[u8]) -> String {
    format!("{:016x}", seahash::hash(data))
}

/// Write `data` to `path`, creating parent directories first.
pub fn write_file(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    std::fs::write(path, data).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Extension-trait helpers for common [`HtmlElement`] operations.

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

    /// Collect the text of all descendant `HtmlNode::Text` nodes,
    /// inserting `\n` for `<br>` elements so the result reflects
    /// multi-line content in source order.
    fn collect_text(&self) -> String;

    /// Recursively visit every descendant `HtmlElement` depth‑first.
    /// Return `WalkControl::SkipChildren` from the callback to skip
    /// an element's children.  Capture external state in the closure.
    fn walk_mut(
        &mut self,
        f: &mut impl FnMut(&mut HtmlElement) -> Result<WalkControl>,
    ) -> Result<()>;
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

    fn walk_mut(
        &mut self,
        f: &mut impl FnMut(&mut HtmlElement) -> Result<WalkControl>,
    ) -> Result<()> {
        if matches!(f(self)?, WalkControl::SkipChildren) {
            return Ok(());
        }
        for child in self.children.make_mut().iter_mut() {
            if let HtmlNode::Element(e) = child {
                e.walk_mut(f)?;
            }
        }
        Ok(())
    }

    fn collect_text(&self) -> String {
        let mut out = String::new();
        collect_impl(self, &mut out);
        out
    }
}

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
