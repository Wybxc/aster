use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine;
use typst_html::HtmlElement;

use crate::document::{ElementProcessor, WalkControl};

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

/// Try to extract a data URI image to a separate file.
///
/// Returns `Some(filename)` when the `src` is a base64 data URI larger than
/// [`IMAGE_EXTRACT_THRESHOLD`], the image is written to `dist_dir` with a
/// content-hashed filename, and the new filename should replace the `src`.
/// Returns `None` when the src is not a data URI, not base64, or too small
/// to warrant extraction.
pub fn try_extract(src: &str, dist_dir: &Path) -> Result<Option<String>> {
    // Only process data URIs.
    let Some(data) = src.strip_prefix("data:") else {
        return Ok(None);
    };

    // Parse: data:[<mediatype>][;base64],<data>
    let Some((header, encoded)) = data.split_once(',') else {
        return Ok(None);
    };

    let is_base64 = header.contains(";base64");
    if !is_base64 {
        return Ok(None);
    }

    let mediatype = header.trim_end_matches(";base64");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim_end())
        .context("failed to decode base64 image data")?;

    if decoded.len() < IMAGE_EXTRACT_THRESHOLD {
        return Ok(None);
    }

    let hash = format!("{:016x}", seahash::hash(&decoded));
    let ext = media_type_to_ext(mediatype);
    let filename = format!("{hash}.{ext}");
    let output = dist_dir.join(&filename);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).context("failed to create dist directory")?;
    }
    std::fs::write(&output, &decoded)
        .with_context(|| format!("failed to write {}", output.display()))?;

    Ok(Some(filename))
}

/// Processor that extracts data URI images from `<img>` elements.
pub struct ImageProcessor {
    pub dist_dir: PathBuf,
}

impl ElementProcessor for ImageProcessor {
    fn matches(&self, elem: &HtmlElement) -> bool {
        if elem.tag != typst_html::tag::img {
            return false;
        }
        elem.attrs
            .0
            .iter()
            .any(|(a, v)| *a.resolve() == *"src" && v.as_str().starts_with("data:"))
    }

    fn process(&self, elem: &mut HtmlElement) -> Result<WalkControl> {
        let src = elem
            .attrs
            .0
            .iter()
            .find_map(|(a, v)| (*a.resolve() == *"src").then(|| v.clone()));
        let Some(src) = src else {
            return Ok(WalkControl::Continue);
        };

        if let Some(new_src) = try_extract(&src, &self.dist_dir)? {
            for (a, v) in elem.attrs.0.make_mut().iter_mut() {
                if *a.resolve() == *"src" {
                    *v = new_src.clone().into();
                }
            }
        }
        Ok(WalkControl::Continue)
    }
}

/// Map a MIME type string (e.g. `"image/png"`) to a file extension.
fn media_type_to_ext(mediatype: &str) -> &'static str {
    match mediatype {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        "image/avif" => "avif",
        _ => "bin",
    }
}
