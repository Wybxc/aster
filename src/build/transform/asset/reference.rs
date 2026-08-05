use anyhow::{Result, ensure};
use typst::ecow::{EcoString, eco_format};
use typst::syntax::{Span, VirtualPath, VirtualRoot};

use crate::build::output::PagePublication;
use crate::build::transform::url::UrlReference;

/// A classified project reference resolved to the project virtual filesystem.
pub struct ProjectReference {
    pub source: VirtualPath,
    suffix: EcoString,
}

impl ProjectReference {
    pub fn with_url(self, url: EcoString) -> EcoString {
        eco_format!("{url}{}", self.suffix)
    }
}

pub fn resolve_project_reference(
    page: &PagePublication<'_>,
    span: Span,
    reference: UrlReference<'_>,
) -> Result<Option<ProjectReference>> {
    let (source, suffix) = match reference {
        UrlReference::Rooted { path, suffix } => (page.resolve_source(path)?, suffix),
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

pub fn source_origin(page: &PagePublication<'_>, span: Span) -> Result<VirtualPath> {
    let Some(id) = span.id() else {
        return Ok(page.template().clone());
    };
    ensure!(
        matches!(id.root(), VirtualRoot::Project),
        "relative HTML resource originates outside the project"
    );
    Ok(id.vpath().clone())
}
