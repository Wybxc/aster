use std::path::{Path, PathBuf};

use anyhow::Result;
use comemo::Tracked;
use lightningcss::bundler::{Bundler, FileProvider, ResolveResult, SourceProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst_html::HtmlElement;

use crate::compile::{FileAccessError, ProjectFiles};
use crate::output::PagePublication;
use crate::utils::HtmlElementExt;

/// A cloneable CSS transformation error at the memoization seam.
///
/// Upstream lightningcss errors carry non-static lifetimes or are not Clone,
/// so their stable classifications and locations are decomposed into fields;
/// the display strings are derived from those fields.
#[derive(Debug, Clone, thiserror::Error)]
enum BundleError {
    #[error("failed to bundle {path}: {kind}{location}")]
    Bundle {
        path: PathBuf,
        kind: String,
        location: String,
    },
    #[error("failed to minify CSS: {kind}{location}")]
    Minify { kind: String, location: String },
    #[error("failed to serialize CSS: {kind}{location}")]
    Serialize { kind: String, location: String },
    #[error("CSS import {path} escapes {source_root}")]
    Escapes { path: PathBuf, source_root: PathBuf },
    #[error(transparent)]
    File(#[from] FileAccessError),
}

/// Decompose a lightningcss error into its stable classification and a
/// formatted source location, both of which are cloneable.
fn decompose(error: &lightningcss::error::Error<impl std::fmt::Display>) -> (String, String) {
    let location = error
        .loc
        .as_ref()
        .map(|loc| format!(" at {}:{}:{}", loc.filename, loc.line, loc.column))
        .unwrap_or_default();
    (error.kind.to_string(), location)
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
            path: entry.to_owned(),
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
                path,
                source_root: self.source_root.clone(),
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
            .map_err(|error| FileAccessError::io(file.clone(), error).into())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::TypstSession;
    use crate::project::ProjectRoot;

    fn session(root: &Path) -> TypstSession {
        std::fs::write(root.join("aster.toml"), "").unwrap();
        TypstSession::new(ProjectRoot::new(root.to_owned()).unwrap())
    }

    #[test]
    fn rejects_transitive_import_outside_source_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("style.css"), "@import \"../secret.css\";").unwrap();
        std::fs::write(root.join("secret.css"), ".secret { color: red; }").unwrap();

        let session = session(root);
        let source_root = std::fs::canonicalize(&src).unwrap();
        assert!(
            bundle_file(
                session.project_files(),
                &src.join("style.css"),
                &source_root
            )
            .is_err()
        );
    }

    #[test]
    fn bundles_imports_through_the_upstream_file_provider() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        std::fs::create_dir_all(src.join("styles")).unwrap();
        std::fs::write(
            src.join("style.css"),
            "@import \"styles/base.css\"; .page { color: red; }",
        )
        .unwrap();
        std::fs::write(src.join("styles/base.css"), ".base { color: blue; }").unwrap();

        let session = session(temp.path());
        let source_root = std::fs::canonicalize(&src).unwrap();
        let bundled = bundle_file(
            session.project_files(),
            &src.join("style.css"),
            &source_root,
        )
        .unwrap();

        assert!(bundled.contains(".base"));
        assert!(bundled.contains(".page"));
    }

    #[test]
    fn memoization_tracks_entry_and_transitive_imports() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        let styles = src.join("styles");
        std::fs::create_dir_all(&styles).unwrap();
        let entry = src.join("style.css");
        let dependency = styles.join("base.css");
        std::fs::write(&entry, "@import \"styles/base.css\"; .page { color: red; }").unwrap();
        std::fs::write(&dependency, ".base { color: blue; }").unwrap();

        let mut session = session(temp.path());
        let source_root = std::fs::canonicalize(&src).unwrap();

        let first = bundle_file(session.project_files(), &entry, &source_root).unwrap();
        assert!(!comemo::testing::last_was_hit());

        session.reset();
        let repeated = bundle_file(session.project_files(), &entry, &source_root).unwrap();
        assert!(comemo::testing::last_was_hit());
        assert_eq!(repeated, first);
        let dependencies = session.dependencies();
        assert!(dependencies.contains(&std::fs::canonicalize(&entry).unwrap()));
        assert!(dependencies.contains(&std::fs::canonicalize(&dependency).unwrap()));

        std::fs::write(src.join("unrelated.css"), ".unused { color: black; }").unwrap();
        session.reset();
        let unrelated = bundle_file(session.project_files(), &entry, &source_root).unwrap();
        assert!(comemo::testing::last_was_hit());
        assert_eq!(unrelated, first);

        std::fs::write(&dependency, ".base { color: green; }").unwrap();
        session.reset();
        let imported = bundle_file(session.project_files(), &entry, &source_root).unwrap();
        assert!(!comemo::testing::last_was_hit());
        assert_ne!(imported, first);

        std::fs::write(
            &entry,
            "@import \"styles/base.css\"; .page { color: purple; }",
        )
        .unwrap();
        session.reset();
        let changed = bundle_file(session.project_files(), &entry, &source_root).unwrap();
        assert!(!comemo::testing::last_was_hit());
        assert_ne!(changed, imported);
    }

    #[test]
    fn memoization_rechecks_missing_imports() {
        let temp = tempfile::tempdir().unwrap();
        let src = temp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let entry = src.join("style.css");
        let dependency = src.join("missing.css");
        std::fs::write(&entry, "@import \"missing.css\";").unwrap();

        let mut session = session(temp.path());
        let source_root = std::fs::canonicalize(&src).unwrap();

        assert!(bundle_file(session.project_files(), &entry, &source_root).is_err());
        assert!(!comemo::testing::last_was_hit());

        session.reset();
        assert!(bundle_file(session.project_files(), &entry, &source_root).is_err());
        assert!(comemo::testing::last_was_hit());
        assert!(session.dependencies().contains(&dependency));

        std::fs::write(&dependency, ".created { color: green; }").unwrap();
        session.reset();
        let bundled = bundle_file(session.project_files(), &entry, &source_root).unwrap();
        assert!(!comemo::testing::last_was_hit());
        assert!(bundled.contains(".created"));
    }
}
