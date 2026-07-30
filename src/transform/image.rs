use anyhow::{Context, Result};
use base64::Engine;
use typst_html::HtmlDocument;

use super::{ElementProcessor, WalkControl};
use crate::output::PagePublication;
use crate::utils::HtmlElementExt;

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

pub(super) struct ImageProcessor;

impl ElementProcessor for ImageProcessor {
    fn process(&self, doc: &mut HtmlDocument, page: &mut PagePublication<'_>) -> Result<()> {
        doc.root_mut().walk_mut(&mut |elem| {
            if !elem.is_tag(typst_html::tag::img) {
                return Ok(WalkControl::Continue);
            }
            if !elem.has_attr("src", |value| value.as_str().starts_with("data:")) {
                return Ok(WalkControl::Continue);
            }

            let Some(src) = elem.get_attr("src") else {
                return Ok(WalkControl::Continue);
            };
            if let Some((content, extension)) = try_extract(&src)? {
                let url = page.add_asset("img", extension, content)?;
                elem.update_attr("src", |value| *value = url.as_str().into());
            }
            Ok(WalkControl::Continue)
        })
    }
}

fn try_extract(src: &str) -> Result<Option<(Vec<u8>, &'static str)>> {
    let Some(data) = src.strip_prefix("data:") else {
        return Ok(None);
    };
    let Some((header, encoded)) = data.split_once(',') else {
        return Ok(None);
    };
    if !header.contains(";base64") {
        return Ok(None);
    }

    let mediatype = header.trim_end_matches(";base64");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim_end())
        .context("failed to decode base64 image data")?;
    if decoded.len() < IMAGE_EXTRACT_THRESHOLD {
        return Ok(None);
    }
    Ok(Some((decoded, media_type_to_ext(mediatype))))
}

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
