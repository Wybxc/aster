use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use comemo::Tracked;
use lightningcss::bundler::{Bundler, FileProvider, ResolveResult, SourceProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst::ecow::EcoString;
use typst_html::HtmlElement;

use crate::build::output::PagePublication;
use crate::build::transform::dom::HtmlElementExt;
use crate::foundation::files::{FileAccessError, ProjectFiles};

/// A cheaply cloneable CSS transformation error at the memoization seam.
///
/// Upstream lightningcss errors carry non-static lifetimes or are not Clone,
/// so their stable classifications and locations are decomposed into fields;
/// the display strings are derived from those fields. All fields are
/// reference-counted or cheap to clone.
#[derive(Debug, Clone, thiserror::Error)]
enum BundleError {
    #[error("failed to bundle {path}: {kind}{location}")]
    Bundle {
        path: Arc<Path>,
        kind: EcoString,
        location: EcoString,
    },
    #[error("failed to minify CSS: {kind}{location}")]
    Minify {
        kind: EcoString,
        location: EcoString,
    },
    #[error("failed to serialize CSS: {kind}{location}")]
    Serialize {
        kind: EcoString,
        location: EcoString,
    },
    #[error("CSS import {path} escapes {source_root}")]
    Escapes {
        path: Arc<Path>,
        source_root: Arc<Path>,
    },
    #[error(transparent)]
    File(#[from] FileAccessError),
}

/// Decompose a lightningcss error into its stable classification and a
/// formatted source location, both of which are cheaply cloneable.
fn decompose(error: &lightningcss::error::Error<impl std::fmt::Display>) -> (EcoString, EcoString) {
    let location = error
        .loc
        .as_ref()
        .map(|loc| format!(" at {}:{}:{}", loc.filename, loc.line, loc.column))
        .unwrap_or_default();
    (error.kind.to_string().into(), location.into())
}

pub(super) fn process_element(
    element: &mut HtmlElement,
    page: &mut PagePublication<'_>,
    project_files: Tracked<ProjectFiles>,
) -> Result<()> {
    if !element.is_tag(typst_html::tag::link) || !element.has_attr("rel", |value| value == "css") {
        return Ok(());
    }

    let href = element
        .get_attr("href")
        .ok_or_else(|| anyhow::anyhow!("link element of type \"css\" is missing href attribute"))?;
    let source = page.resolve_source(Path::new(href.as_str()))?;
    let css = bundle_file(project_files, &source, &page.source_root()?)
        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    let url = page.add_asset("css", "css", css.into_bytes())?;

    element.update_attr("href", |value| *value = url.as_str().into());
    element.update_attr("rel", |value| *value = "stylesheet".into());
    Ok(())
}

/// Bundle a CSS entry point while confining and tracking every transitive import.
#[comemo::memoize]
fn bundle_file(
    project_files: Tracked<ProjectFiles>,
    entry: &Path,
    source_root: &Path,
) -> std::result::Result<String, BundleError> {
    let provider = ConfinedFileProvider::new(source_root.to_owned(), project_files);
    let mut bundler = Bundler::new(&provider, None, ParserOptions::default());
    let mut stylesheet = bundler.bundle(entry).map_err(|error| {
        let (kind, location) = decompose(&error);
        BundleError::Bundle {
            path: entry.into(),
            kind,
            location,
        }
    })?;
    stylesheet
        .minify(MinifyOptions {
            targets: Browsers::default().into(),
            ..MinifyOptions::default()
        })
        .map_err(|error| {
            let (kind, location) = decompose(&error);
            BundleError::Minify { kind, location }
        })?;
    let result = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        })
        .map_err(|error| {
            let (kind, location) = decompose(&error);
            BundleError::Serialize { kind, location }
        })?;
    Ok(result.code)
}

struct ConfinedFileProvider<'a> {
    source_root: PathBuf,
    project_files: Tracked<'a, ProjectFiles>,
    files: FileProvider,
}

impl<'a> ConfinedFileProvider<'a> {
    fn new(source_root: PathBuf, project_files: Tracked<'a, ProjectFiles>) -> Self {
        Self {
            source_root,
            project_files,
            files: FileProvider::new(),
        }
    }

    fn confined(&self, path: &Path) -> std::result::Result<PathBuf, BundleError> {
        let path = self.project_files.canonicalize(path)?;
        if !path.starts_with(&self.source_root) {
            return Err(BundleError::Escapes {
                path: path.into(),
                source_root: self.source_root.clone().into(),
            });
        }
        Ok(path)
    }
}

impl SourceProvider for ConfinedFileProvider<'_> {
    type Error = BundleError;

    fn read<'a>(&'a self, file: &Path) -> std::result::Result<&'a str, Self::Error> {
        let file = self.confined(file)?;
        self.project_files.read(&file)?;
        self.files
            .read(&file)
            .map_err(|error| FileAccessError::io(file.into(), error).into())
    }

    fn resolve(
        &self,
        specifier: &str,
        originating_file: &Path,
    ) -> std::result::Result<ResolveResult, Self::Error> {
        let candidate = originating_file.with_file_name(specifier);
        Ok(ResolveResult::File(self.confined(&candidate)?))
    }
}
