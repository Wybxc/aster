use anyhow::{Context, Result};
use data_url::{DataUrl, mime::Mime};
use typst_html::HtmlElement;

use crate::output::PagePublication;
use crate::utils::HtmlElementExt;

/// Minimum decoded size (bytes) below which a data URI stays inline.
const IMAGE_EXTRACT_THRESHOLD: usize = 1024;

pub(super) fn process_element(
    element: &mut HtmlElement,
    page: &mut PagePublication<'_>,
) -> Result<()> {
    if !element.is_tag(typst_html::tag::img) {
        return Ok(());
    }
    let Some(src) = element.get_attr("src") else {
        return Ok(());
    };
    if let Some((content, extension)) = try_extract(&src)? {
        let url = page.add_asset("img", extension, content)?;
        element.update_attr("src", |value| *value = url.as_str().into());
    }
    Ok(())
}

fn try_extract(src: &str) -> Result<Option<(Vec<u8>, &'static str)>> {
    let Ok(data_url) = DataUrl::process(src) else {
        return Ok(None);
    };
    let (decoded, _) = data_url
        .decode_to_vec()
        .context("failed to decode image data URL")?;
    if decoded.len() < IMAGE_EXTRACT_THRESHOLD {
        return Ok(None);
    }
    Ok(Some((decoded, media_type_to_ext(data_url.mime_type()))))
}

fn media_type_to_ext(mediatype: &Mime) -> &'static str {
    if mediatype.matches("image", "png") {
        "png"
    } else if mediatype.matches("image", "jpeg") || mediatype.matches("image", "jpg") {
        "jpg"
    } else if mediatype.matches("image", "gif") {
        "gif"
    } else if mediatype.matches("image", "svg+xml") {
        "svg"
    } else if mediatype.matches("image", "webp") {
        "webp"
    } else if mediatype.matches("image", "avif") {
        "avif"
    } else {
        "bin"
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

        let (content, extension) = try_extract(&source).unwrap().unwrap();

        assert_eq!(content.len(), 1026);
        assert_eq!(extension, "png");
    }

    #[test]
    fn extracts_percent_encoded_data_urls() {
        let source = format!(
            "data:image/svg+xml,{}",
            "%78".repeat(IMAGE_EXTRACT_THRESHOLD)
        );

        let (content, extension) = try_extract(&source).unwrap().unwrap();

        assert_eq!(content, vec![b'x'; IMAGE_EXTRACT_THRESHOLD]);
        assert_eq!(extension, "svg");
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
