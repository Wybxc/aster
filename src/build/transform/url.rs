/// The lexical interpretation of a URL reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UrlReference<'a> {
    /// A single leading slash identifies a local root reference.
    Rooted { path: &'a str, suffix: &'a str },
    /// A path is relative to the context that contains it.
    Relative { path: &'a str, suffix: &'a str },
    /// An inline data URL.
    Data { url: &'a str },
    /// A protocol URL, protocol-relative URL, fragment, or query stays in the browser.
    Browser,
}

pub fn classify_url(reference: &str) -> UrlReference<'_> {
    if reference.is_empty()
        || reference.starts_with("//")
        || matches!(reference.as_bytes().first(), Some(b'#' | b'?'))
    {
        return UrlReference::Browser;
    }
    if let Ok(url) = url::Url::parse(reference) {
        return if url.scheme() == "data" {
            UrlReference::Data { url: reference }
        } else {
            UrlReference::Browser
        };
    }

    let suffix_start = reference.find(['?', '#']).unwrap_or(reference.len());
    let (path, suffix) = reference.split_at(suffix_start);
    if path.starts_with('/') {
        UrlReference::Rooted { path, suffix }
    } else {
        UrlReference::Relative { path, suffix }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_local_references_and_browser_urls() {
        assert_eq!(
            classify_url("/assets/image.png?v=1#hero"),
            UrlReference::Rooted {
                path: "/assets/image.png",
                suffix: "?v=1#hero"
            }
        );
        assert_eq!(
            classify_url("../image.png"),
            UrlReference::Relative {
                path: "../image.png",
                suffix: ""
            }
        );
        assert_eq!(
            classify_url("DATA:image/png;base64,AA=="),
            UrlReference::Data {
                url: "DATA:image/png;base64,AA=="
            }
        );
        for reference in [
            "",
            "//cdn.example/image.png",
            "https://example.com/image.png",
            "#icon",
            "?raw",
        ] {
            assert_eq!(classify_url(reference), UrlReference::Browser);
        }
    }
}
