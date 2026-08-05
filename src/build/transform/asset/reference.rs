use anyhow::{Result, ensure};
use typst::ecow::{EcoString, eco_format};
use typst::syntax::{Span, VirtualPath, VirtualRoot};
use url::Url;

use crate::build::output::PagePublication;

/// The lexical interpretation of a URL-bearing resource attribute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UrlReference<'a> {
    /// A single leading slash resolves from the project virtual root.
    ProjectRoot { path: &'a str, suffix: &'a str },
    /// A path resolves relative to the source that produced the HTML or CSS.
    Relative { path: &'a str, suffix: &'a str },
    /// An inline data URL.
    Data { url: &'a str },
    /// A protocol URL, protocol-relative URL, fragment, or query stays in the browser.
    Browser,
}

pub(crate) fn classify_url(reference: &str) -> UrlReference<'_> {
    if reference.is_empty()
        || reference.starts_with("//")
        || matches!(reference.as_bytes().first(), Some(b'#' | b'?'))
    {
        return UrlReference::Browser;
    }
    if let Ok(url) = Url::parse(reference) {
        return if url.scheme() == "data" {
            UrlReference::Data { url: reference }
        } else {
            UrlReference::Browser
        };
    }

    let suffix_start = reference.find(['?', '#']).unwrap_or(reference.len());
    let (path, suffix) = reference.split_at(suffix_start);
    if path.starts_with('/') {
        UrlReference::ProjectRoot { path, suffix }
    } else {
        UrlReference::Relative { path, suffix }
    }
}

/// A classified project reference resolved to the project virtual filesystem.
pub(crate) struct ProjectReference {
    pub source: VirtualPath,
    suffix: EcoString,
}

impl ProjectReference {
    pub fn with_url(self, url: EcoString) -> EcoString {
        eco_format!("{url}{}", self.suffix)
    }
}

pub(crate) fn resolve_project_reference(
    page: &PagePublication<'_>,
    span: Span,
    reference: UrlReference<'_>,
) -> Result<Option<ProjectReference>> {
    let (source, suffix) = match reference {
        UrlReference::ProjectRoot { path, suffix } => (page.resolve_source(path)?, suffix),
        UrlReference::Relative { path, suffix } => {
            let origin = source_origin(page, span)?;
            (page.resolve_source_from(&origin, path)?, suffix)
        }
        UrlReference::Data { .. } | UrlReference::Browser => return Ok(None),
    };
    Ok(Some(ProjectReference {
        source,
        suffix: suffix.into(),
    }))
}

pub(crate) fn source_origin(page: &PagePublication<'_>, span: Span) -> Result<VirtualPath> {
    let Some(id) = span.id() else {
        return Ok(page.template().clone());
    };
    ensure!(
        matches!(id.root(), VirtualRoot::Project),
        "relative HTML resource originates outside the project"
    );
    Ok(id.vpath().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_project_references_and_browser_urls() {
        assert_eq!(
            classify_url("/assets/image.png?v=1#hero"),
            UrlReference::ProjectRoot {
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
