use anyhow::{Context, Result};
use data_url::{DataUrl, mime::Mime};

pub struct DataAsset {
    pub content: Vec<u8>,
    pub extension: Option<&'static str>,
}

pub fn decode_data_url(url: &str, inline_threshold: usize) -> Result<Option<DataAsset>> {
    let Ok(data_url) = DataUrl::process(url) else {
        return Ok(None);
    };
    let (content, _) = data_url
        .decode_to_vec()
        .context("failed to decode data URL")?;
    if content.len() < inline_threshold {
        return Ok(None);
    }
    let mime = data_url.mime_type();
    Ok(Some(DataAsset {
        content,
        extension: mime_extension(mime),
    }))
}

fn mime_extension(mime: &Mime) -> Option<&'static str> {
    let preferred = match (mime.type_.as_str(), mime.subtype.as_str()) {
        ("image", "jpeg" | "jpg") => Some("jpg"),
        ("audio", "mpeg") => Some("mp3"),
        ("video", "ogg") => Some("ogv"),
        ("application", "javascript") | ("text", "javascript") => Some("js"),
        ("application", "manifest+json") => Some("webmanifest"),
        _ => None,
    };
    preferred.or_else(|| {
        let mime = format!("{}/{}", mime.type_, mime.subtype);
        mime_guess::get_mime_extensions_str(&mime)
            .and_then(|extensions| extensions.first().copied())
    })
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

        let asset = decode_data_url(&source, DEFAULT_INLINE_THRESHOLD)
            .unwrap()
            .unwrap();

        assert_eq!(asset.content.len(), 1026);
        assert_eq!(asset.extension, Some("png"));
    }

    #[test]
    fn extracts_percent_encoded_data_urls() {
        let source = format!(
            "data:image/svg+xml,{}",
            "%78".repeat(DEFAULT_INLINE_THRESHOLD)
        );

        let asset = decode_data_url(&source, DEFAULT_INLINE_THRESHOLD)
            .unwrap()
            .unwrap();

        assert_eq!(asset.content, vec![b'x'; DEFAULT_INLINE_THRESHOLD]);
        assert_eq!(asset.extension, Some("svg"));
    }

    #[test]
    fn keeps_small_or_malformed_data_urls_inline() {
        assert!(
            decode_data_url("data:image/png;base64,AAAA", DEFAULT_INLINE_THRESHOLD)
                .unwrap()
                .is_none()
        );
        assert!(
            decode_data_url("data:image/png", DEFAULT_INLINE_THRESHOLD)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_invalid_base64_data() {
        assert!(decode_data_url("data:image/png;base64,%%%!", DEFAULT_INLINE_THRESHOLD).is_err());
    }

    #[test]
    fn configured_threshold_controls_extraction() {
        let source = "data:image/png;base64,AAAA";
        assert!(decode_data_url(source, 4).unwrap().is_none());
        assert!(decode_data_url(source, 3).unwrap().is_some());
    }

    #[test]
    fn derives_non_image_extensions_from_mime() {
        let asset = decode_data_url("data:audio/mpeg;base64,AAAA", 3)
            .unwrap()
            .unwrap();

        assert_eq!(asset.extension, Some("mp3"));
    }
}
