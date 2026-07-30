use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use typst_html::HtmlDocument;

use super::{ElementProcessor, ProcessingContext, WalkControl};
use crate::utils::{AssetCollector, HtmlElementExt};

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

pub(super) struct ImageProcessor;

impl ElementProcessor for ImageProcessor {
    fn process(
        &self,
        doc: &mut HtmlDocument,
        assets: &mut AssetCollector,
        _ctx: &ProcessingContext<'_>,
    ) -> Result<()> {
        doc.root_mut().walk_mut(&mut |elem| {
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

            if let Some((content, ext)) = try_extract(&src)? {
                let path = assets.add(Path::new(""), "img", ext, content);
                elem.update_attr("src", |v| *v = path.to_string_lossy().into());
            }
            Ok(WalkControl::Continue)
        })
    }
}

/// Try to extract a data URI image.
/// Returns `(decoded_bytes, file_extension)` on success.
fn try_extract(src: &str) -> Result<Option<(Vec<u8>, &'static str)>> {
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

    let ext = media_type_to_ext(mediatype);
    Ok(Some((decoded, ext)))
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
