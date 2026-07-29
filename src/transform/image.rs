use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use typst::ecow::{EcoString, eco_format};
use typst_html::HtmlElement;

use super::{ElementProcessor, ProcessingContext, WalkControl};

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

/// Processor that extracts data URI images from `<img>` elements.
pub(super) struct ImageProcessor;

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

    fn process(&self, elem: &mut HtmlElement, ctx: &ProcessingContext<'_>) -> Result<WalkControl> {
        let src = elem
            .attrs
            .0
            .iter()
            .find_map(|(a, v)| (*a.resolve() == *"src").then(|| v.clone()));
        let Some(src) = src else {
            return Ok(WalkControl::Continue);
        };

        if let Some(new_src) = try_extract(&src, &ctx.output_dir())? {
            for (a, v) in elem.attrs.0.make_mut().iter_mut() {
                if *a.resolve() == *"src" {
                    *v = new_src.clone();
                }
            }
        }
        Ok(WalkControl::Continue)
    }
}

/// Try to extract a data URI image to a separate file.
fn try_extract(src: &str, dist_dir: &Path) -> Result<Option<EcoString>> {
    let Some(data) = src.strip_prefix("data:") else {
        return Ok(None);
    };

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
    let filename = eco_format!("{hash}.{ext}");
    let output = dist_dir.join(filename.as_str());

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).context("failed to create dist directory")?;
    }
    std::fs::write(&output, &decoded)
        .with_context(|| format!("failed to write {}", output.display()))?;

    Ok(Some(filename))
}

/// Map a MIME type string to a file extension.
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
