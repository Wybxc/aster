use std::path::Path;

use anyhow::Result;

/// A strategy for processing a resource referenced by `<link href="...">`.
///
/// Each implementation checks `can_handle` to decide whether it should
/// process a given `href`.  When multiple loaders are registered, the first
/// match wins.  A catch-all [`URLLoader`] at the end ensures every href is
/// handled.
pub trait BundleLoader {
    /// Whether this loader can handle `href`.
    fn can_handle(&self, href: &str) -> bool;
    /// Bundle the resource at `href` (relative to `src_dir`), write the result
    /// to `dist_dir`, and return the new href value for the DOM.
    fn bundle(&self, href: &str, src_dir: &Path, dist_dir: &Path) -> Result<String>;
}

// ---------------------------------------------------------------------------
// URL loader — passes through external URLs unchanged
// ---------------------------------------------------------------------------

/// Passes through the `href` unchanged (catch-all for external URLs and
/// resource types that don't need transformation).
pub struct URLLoader;

impl BundleLoader for URLLoader {
    fn can_handle(&self, _href: &str) -> bool {
        true
    }
    fn bundle(&self, href: &str, _src_dir: &Path, _dist_dir: &Path) -> Result<String> {
        Ok(href.to_owned())
    }
}

// ---------------------------------------------------------------------------
// Sub-modules
// ---------------------------------------------------------------------------

pub mod css;
pub use css::CssLoader;
