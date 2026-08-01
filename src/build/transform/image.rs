use anyhow::{Context, Result};
use data_url::{DataUrl, mime::Mime};
use typst_html::HtmlElement;

use crate::build::output::{ImageFormat, PagePublication};
use crate::build::transform::{Processor, WalkControl, dom::HtmlElementExt};

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

pub(crate) struct ImageProcessor;

impl ImageProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Processor for ImageProcessor {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        if !element.is_tag(typst_html::tag::img) {
            return Ok(WalkControl::Continue);
        }
        let Some(src) = element.get_attr("src") else {
            return Ok(WalkControl::Continue);
        };
        if let Some((content, format)) = try_extract(&src)? {
            let url = page.add_image(format, content)?;
            element.update_attr("src", |value| *value = url);
        }
        Ok(WalkControl::Continue)
    }
}

fn try_extract(src: &str) -> Result<Option<(Vec<u8>, ImageFormat)>> {
    let Ok(data_url) = DataUrl::process(src) else {
        return Ok(None);
    };
    let (decoded, _) = data_url
        .decode_to_vec()
        .context("failed to decode image data URL")?;
    if decoded.len() < IMAGE_EXTRACT_THRESHOLD {
        return Ok(None);
    }
    Ok(Some((decoded, media_type_to_format(data_url.mime_type()))))
}

fn media_type_to_format(mediatype: &Mime) -> ImageFormat {
    if mediatype.matches("image", "png") {
        ImageFormat::Png
    } else if mediatype.matches("image", "jpeg") || mediatype.matches("image", "jpg") {
        ImageFormat::Jpeg
    } else if mediatype.matches("image", "gif") {
        ImageFormat::Gif
    } else if mediatype.matches("image", "svg+xml") {
        ImageFormat::Svg
    } else if mediatype.matches("image", "webp") {
        ImageFormat::Webp
    } else if mediatype.matches("image", "avif") {
        ImageFormat::Avif
    } else {
        ImageFormat::Binary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_standard_base64_data_urls_with_mime_parameters() {
        let source = format!(
            "DATA:image/png;charset=utf-8;base64,{}#ignored",
            "AAAA".repeat(342)
        );

        let (content, format) = try_extract(&source).unwrap().unwrap();

        assert_eq!(content.len(), 1026);
        assert_eq!(format, ImageFormat::Png);
    }

    #[test]
    fn extracts_percent_encoded_data_urls() {
        let source = format!(
            "data:image/svg+xml,{}",
            "%78".repeat(IMAGE_EXTRACT_THRESHOLD)
        );

        let (content, format) = try_extract(&source).unwrap().unwrap();

        assert_eq!(content, vec![b'x'; IMAGE_EXTRACT_THRESHOLD]);
        assert_eq!(format, ImageFormat::Svg);
    }

    #[test]
    fn keeps_small_or_non_data_urls_inline() {
        assert!(try_extract("data:image/png;base64,AAAA").unwrap().is_none());
        assert!(try_extract("image.png").unwrap().is_none());
        assert!(try_extract("data:image/png").unwrap().is_none());
    }

    #[test]
    fn rejects_invalid_base64_data() {
        assert!(try_extract("data:image/png;base64,%%%!").is_err());
    }
}
