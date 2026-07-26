use anyhow::Result;
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

/// Recursively visit every descendant `HtmlElement` depth-first (immutable).
///
/// Unlike [`walk_mut`], this does not support `SkipChildren` or error
/// propagation — it always visits the full subtree.
pub fn walk(elem: &HtmlElement, f: &mut dyn FnMut(&HtmlElement)) {
    f(elem);
    for child in &elem.children {
        if let HtmlNode::Element(e) = child {
            walk(e, f);
        }
    }
}

/// Signal returned by the callback passed to [`walk_mut`] to control
/// whether children of the current element should be visited.
pub enum WalkControl {
    /// Continue recursion into children.
    Continue,
    /// Skip the current element's subtree.
    SkipChildren,
}

/// Recursively visit every descendant `HtmlElement` depth-first.
///
/// The callback `f` is called for each element.  Return
/// [`WalkControl::SkipChildren`] to skip that element's subtree.
///
/// Errors from `f` are propagated via the `E` type parameter — use
/// `walk_mut::<()>` for infallible walks.
pub fn walk_mut<E>(
    elem: &mut HtmlElement,
    f: &mut impl FnMut(&mut HtmlElement) -> Result<WalkControl, E>,
) -> Result<(), E> {
    if matches!(f(elem)?, WalkControl::SkipChildren) {
        return Ok(());
    }
    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk_mut(e, f)?;
        }
    }
    Ok(())
}

/// A processor that matches and transforms individual [`HtmlElement`] nodes.
///
/// Implementations are registered via [`process_all`], which runs them in a
/// single DOM traversal.  Each processor is called for every element where
/// [`matches`](Self::matches) returns `true`.
pub trait ElementProcessor {
    /// Whether this processor should handle `elem`.
    fn matches(&self, elem: &HtmlElement) -> bool;
    /// Transform `elem`.  Return [`WalkControl::SkipChildren`] to stop
    /// recursion into the element's subtree.
    fn process(&self, elem: &mut HtmlElement) -> Result<WalkControl>;
}

/// Run a set of [`ElementProcessor`]s on the document in a single traversal.
pub fn process_all(doc: &mut HtmlDocument, processors: &[&dyn ElementProcessor]) -> Result<()> {
    walk_mut(doc.root_mut(), &mut |elem| {
        for p in processors {
            if p.matches(elem) && matches!(p.process(elem)?, WalkControl::SkipChildren) {
                return Ok(WalkControl::SkipChildren);
            }
        }
        Ok(WalkControl::Continue)
    })
}
