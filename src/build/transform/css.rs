use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use comemo::Tracked;
use lightningcss::bundler::{Bundler, FileProvider, ResolveResult, SourceProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst::ecow::{EcoString, eco_format};
use typst::syntax::VirtualPath;
use typst_html::HtmlElement;

use crate::build::output::PagePublication;
use crate::build::transform::{Processor, WalkControl, dom::HtmlElementExt};
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
    #[error("CSS import {path} escapes project root {project_root}")]
    Escapes {
        path: Arc<Path>,
        project_root: Arc<Path>,
    },
    #[error("invalid CSS path {path}: {message}")]
    InvalidPath { path: Arc<Path>, message: EcoString },
    #[error(transparent)]
    File(#[from] FileAccessError),
}

/// Decompose a lightningcss error into its stable classification and a
/// formatted source location, both of which are cheaply cloneable.
fn decompose(error: &lightningcss::error::Error<impl std::fmt::Display>) -> (EcoString, EcoString) {
    let location = error
        .loc
        .as_ref()
        .map(|loc| eco_format!(" at {}:{}:{}", loc.filename, loc.line, loc.column))
        .unwrap_or_default();
    (eco_format!("{}", error.kind), location)
}

pub(crate) struct CssProcessor<'a> {
    project_files: Tracked<'a, ProjectFiles>,
    minify: bool,
}

impl<'a> CssProcessor<'a> {
    pub fn new(project_files: Tracked<'a, ProjectFiles>, minify: bool) -> Self {
        Self {
            project_files,
            minify,
        }
    }
}

impl Processor for CssProcessor<'_> {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        if !element.is_tag(typst_html::tag::link)
            || !element.has_attr("rel", |value| value == "css")
        {
            return Ok(WalkControl::Continue);
        }

        let href = element.get_attr("href").ok_or_else(|| {
            anyhow::anyhow!("link element of type \"css\" is missing href attribute")
        })?;
        let source = page.resolve_source(Path::new(href.as_str()))?;
        let css = bundle_file(
            self.project_files,
            &source,
            page.project_root(),
            self.minify,
        )
        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
        let url = page.add_bundled_stylesheet(&source, css.into_bytes())?;

        element.update_attr("href", move |value| *value = url);
        element.update_attr("rel", |value| *value = "stylesheet".into());
        Ok(WalkControl::Continue)
    }
}

/// Bundle a CSS entry point while confining and tracking every transitive import.
#[comemo::memoize]
fn bundle_file(
    project_files: Tracked<ProjectFiles>,
    entry: &VirtualPath,
    project_root: &Path,
    minify: bool,
) -> std::result::Result<String, BundleError> {
    let entry = entry
        .realize(project_root)
        .map_err(|error| BundleError::InvalidPath {
            path: PathBuf::from(entry.get_with_slash()).into(),
            message: eco_format!("{error}"),
        })?;
    let provider = ConfinedFileProvider::new(project_root.to_owned(), project_files);
    let mut bundler = Bundler::new(&provider, None, ParserOptions::default());
    let mut stylesheet = bundler.bundle(&entry).map_err(|error| {
        let (kind, location) = decompose(&error);
        BundleError::Bundle {
            path: entry.into(),
            kind,
            location,
        }
    })?;
    if minify {
        stylesheet
            .minify(MinifyOptions {
                targets: Browsers::default().into(),
                ..MinifyOptions::default()
            })
            .map_err(|error| {
                let (kind, location) = decompose(&error);
                BundleError::Minify { kind, location }
            })?;
    }
    let result = stylesheet
        .to_css(PrinterOptions {
            minify,
            ..PrinterOptions::default()
        })
        .map_err(|error| {
            let (kind, location) = decompose(&error);
            BundleError::Serialize { kind, location }
        })?;
    Ok(result.code)
}

struct ConfinedFileProvider<'a> {
    project_root: PathBuf,
    project_files: Tracked<'a, ProjectFiles>,
    files: FileProvider,
}

impl<'a> ConfinedFileProvider<'a> {
    fn new(project_root: PathBuf, project_files: Tracked<'a, ProjectFiles>) -> Self {
        Self {
            project_root,
            project_files,
            files: FileProvider::new(),
        }
    }

    fn confined(&self, path: &Path) -> std::result::Result<(VirtualPath, PathBuf), BundleError> {
        let virtual_path = VirtualPath::virtualize(&self.project_root, path).map_err(|_| {
            BundleError::Escapes {
                path: path.into(),
                project_root: self.project_root.clone().into(),
            }
        })?;
        let path =
            virtual_path
                .realize(&self.project_root)
                .map_err(|error| BundleError::InvalidPath {
                    path: path.into(),
                    message: eco_format!("{error}"),
                })?;
        Ok((virtual_path, path))
    }
}

impl SourceProvider for ConfinedFileProvider<'_> {
    type Error = BundleError;

    fn read<'a>(&'a self, file: &Path) -> std::result::Result<&'a str, Self::Error> {
        let (virtual_file, file) = self.confined(file)?;
        self.project_files.read(&virtual_file)?;
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
        let (_, path) = self.confined(&candidate)?;
        Ok(ResolveResult::File(path))
    }
}
