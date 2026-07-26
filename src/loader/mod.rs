use std::path::Path;

use anyhow::Result;

/// The result of bundling a `<link>` resource: the new attribute values
/// that should replace the original element's `href` and `rel`.
pub struct BundleResult {
    pub href: String,
    pub rel: String,
}

/// Try to bundle the resource referenced by `<link rel="..." href="...">`.
///
/// Returns `Ok(None)` when `rel` is not recognised — the caller should
/// leave the element untouched.  Recognised `rel` values and their effect:
///
/// | `rel`  | Outcome |
/// |--------|---------|
/// | `"css"` | CSS bundled through lightningcss, content-hashed filename, `rel` rewritten to `"stylesheet"` |
pub fn try_bundle(
    rel: &str,
    href: &str,
    src_dir: &Path,
    dist_dir: &Path,
) -> Result<Option<BundleResult>> {
    match rel {
        "css" => {
            let hashed = css::bundle_relative(href, src_dir, dist_dir)?;
            Ok(Some(BundleResult {
                href: hashed,
                rel: "stylesheet".to_owned(),
            }))
        }
        _ => Ok(None),
    }
}

pub mod css;
