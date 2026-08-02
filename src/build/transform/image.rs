use anyhow::{Context, Result};
use data_url::{DataUrl, mime::Mime};
use typst_html::HtmlElement;

use crate::build::output::{ImageFormat, PagePublication};
use crate::build::transform::{Processor, WalkControl, dom::HtmlElementExt};

pub(crate) struct ImageProcessor {
    inline_threshold: usize,
}

impl ImageProcessor {
    pub fn new(inline_threshold: usize) -> Self {
        Self { inline_threshold }
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
        if let Some((content, format)) = try_extract(&src, self.inline_threshold)? {
            let url = page.add_image(format, content)?;
            element.update_attr("src", |value| *value = url);
        }
        Ok(WalkControl::Continue)
    }
}

fn try_extract(src: &str, inline_threshold: usize) -> Result<Option<(Vec<u8>, ImageFormat)>> {
    let Ok(data_url) = DataUrl::process(src) else {
        return Ok(None);
    };
    let (decoded, _) = data_url
        .decode_to_vec()
        .context("failed to decode image data URL")?;
    if decoded.len() < inline_threshold {
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

    const DEFAULT_INLINE_THRESHOLD: usize = 1024;

    #[test]
    fn extracts_standard_base64_data_urls_with_mime_parameters() {
        let source = format!(
            "DATA:image/png;charset=utf-8;base64,{}#ignored",
            "AAAA".repeat(342)
        );

        let (content, format) = try_extract(&source, DEFAULT_INLINE_THRESHOLD)
            .unwrap()
            .unwrap();

        assert_eq!(content.len(), 1026);
        assert_eq!(format, ImageFormat::Png);
    }

    #[test]
    fn extracts_percent_encoded_data_urls() {
        let source = format!(
            "data:image/svg+xml,{}",
            "%78".repeat(DEFAULT_INLINE_THRESHOLD)
        );

        let (content, format) = try_extract(&source, DEFAULT_INLINE_THRESHOLD)
            .unwrap()
            .unwrap();

        assert_eq!(content, vec![b'x'; DEFAULT_INLINE_THRESHOLD]);
        assert_eq!(format, ImageFormat::Svg);
    }

    #[test]
    fn keeps_small_or_non_data_urls_inline() {
        assert!(
            try_extract("data:image/png;base64,AAAA", DEFAULT_INLINE_THRESHOLD)
                .unwrap()
                .is_none()
        );
        assert!(
            try_extract("image.png", DEFAULT_INLINE_THRESHOLD)
                .unwrap()
                .is_none()
        );
        assert!(
            try_extract("data:image/png", DEFAULT_INLINE_THRESHOLD)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_invalid_base64_data() {
        assert!(try_extract("data:image/png;base64,%%%!", DEFAULT_INLINE_THRESHOLD).is_err());
    }

    #[test]
    fn configured_threshold_controls_extraction() {
        let source = "data:image/png;base64,AAAA";
        assert!(try_extract(source, 4).unwrap().is_none());
        assert!(try_extract(source, 3).unwrap().is_some());
    }
}
