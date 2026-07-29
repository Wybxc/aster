use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use typst::ecow::{EcoString, eco_format};
use typst_html::HtmlDocument;

use super::{ElementProcessor, ProcessingContext, WalkControl};
use crate::utils::HtmlElementExt;

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

pub(super) struct ImageProcessor;

impl ElementProcessor for ImageProcessor {
    fn process(&self, doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()> {
        super::walk_mut(doc.root_mut(), &mut |elem| {
            if !elem.is_tag(typst_html::tag::img) {
                return Ok(WalkControl::Continue);
            }
            if !elem.has_attr("src", |v| v.as_str().starts_with("data:")) {
                return Ok(WalkControl::Continue);
            }

            let src = match elem.get_attr("src") {
                Some(s) => s,
                None => return Ok(WalkControl::Continue),
            };

            if let Some(new_src) = try_extract(&src, &ctx.output_dir())? {
                elem.update_attr("src", |v| *v = new_src.clone());
            }
            Ok(WalkControl::Continue)
        })
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

    let hash = crate::utils::content_hash(&decoded);
    let ext = media_type_to_ext(mediatype);
    let filename = eco_format!("{hash}.{ext}");
    let output = dist_dir.join(filename.as_str());

    crate::utils::write_file(&output, &decoded)?;

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
