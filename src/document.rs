use typst_html::{HtmlElement, HtmlNode};

/// Signal returned by the callback passed to [`walk_mut`] to control
/// whether children of the current element should be visited.
pub enum WalkControl {
    /// Continue recursion into children.
    Continue,
    /// Skip the current element's children.
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
