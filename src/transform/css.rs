use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use lightningcss::bundler::{Bundler, ResolveResult, SourceProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst_html::HtmlDocument;

use super::{ElementProcessor, WalkControl};
use crate::output::PagePublication;
use crate::utils::HtmlElementExt;

pub(super) struct CssProcessor;

impl ElementProcessor for CssProcessor {
    fn process(&self, doc: &mut HtmlDocument, page: &mut PagePublication<'_>) -> Result<()> {
        doc.root_mut().walk_mut(&mut |elem| {
            if !elem.is_tag(typst_html::tag::link) {
                return Ok(WalkControl::Continue);
            }
            if !elem.has_attr("rel", |value| value.as_str() == "css") {
                return Ok(WalkControl::Continue);
            }

            let Some(href) = elem.get_attr("href") else {
                return Ok(WalkControl::Continue);
            };
            let source = page.resolve_source(href.as_str())?;
            let css = bundle_file(&source, &page.source_root()?)?;
            let url = page.add_asset("css", "css", css.into_bytes())?;

            elem.update_attr("href", |value| *value = url.as_str().into());
            elem.update_attr("rel", |value| *value = "stylesheet".into());
            Ok(WalkControl::Continue)
        })
    }
}

/// Bundle a CSS entry point while confining every transitive import to `src/`.
///
/// This is deliberately not memoized: the provider reads external files directly,
/// so a path-only cache key cannot observe CSS or imported-file changes.
fn bundle_file(entry: &Path, source_root: &Path) -> Result<String> {
    let provider = ConfinedFileProvider::new(source_root.to_owned());
    let mut bundler = Bundler::new(&provider, None, ParserOptions::default());
    let mut stylesheet = bundler
        .bundle(entry)
        .map_err(|error| anyhow::anyhow!("failed to bundle {}: {error:#}", entry.display()))?;
    stylesheet
        .minify(MinifyOptions {
            targets: Browsers::default().into(),
            ..MinifyOptions::default()
        })
        .map_err(|error| anyhow::anyhow!("failed to minify CSS: {error:#}"))?;
    let result = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        })
        .map_err(|error| anyhow::anyhow!("failed to serialize CSS: {error:#}"))?;
    Ok(result.code)
}

struct ConfinedFileProvider {
    source_root: PathBuf,
    inputs: Mutex<Vec<*mut String>>,
}

impl ConfinedFileProvider {
    fn new(source_root: PathBuf) -> Self {
        Self {
            source_root,
            inputs: Mutex::new(Vec::new()),
        }
    }

    fn confined(&self, path: &Path) -> std::io::Result<PathBuf> {
        let path = std::fs::canonicalize(path)?;
        if !path.starts_with(&self.source_root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "CSS import {} escapes {}",
                    path.display(),
                    self.source_root.display()
                ),
            ));
        }
        Ok(path)
    }
}

// The pointers are owned by `inputs`, guarded by the mutex, and freed in Drop.
unsafe impl Send for ConfinedFileProvider {}
unsafe impl Sync for ConfinedFileProvider {}

impl SourceProvider for ConfinedFileProvider {
    type Error = std::io::Error;

    fn read<'a>(&'a self, file: &Path) -> std::result::Result<&'a str, Self::Error> {
        let file = self.confined(file)?;
        let source = std::fs::read_to_string(file)?;
        let pointer = Box::into_raw(Box::new(source));
        self.inputs.lock().unwrap().push(pointer);
        // SAFETY: the allocation remains owned by this provider until Drop. Entries
        // are never removed and all mutation of the pointer list is synchronized.
        Ok(unsafe { &*pointer })
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

impl Drop for ConfinedFileProvider {
    fn drop(&mut self) {
        for pointer in self.inputs.get_mut().unwrap().drain(..) {
            // SAFETY: each pointer was allocated once in `read`, remains unique,
            // and is removed exactly once while the provider is being dropped.
            drop(unsafe { Box::from_raw(pointer) });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn rejects_transitive_import_outside_source_root() {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("aster-css-test-{}-{id}", std::process::id()));
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("style.css"), "@import \"../secret.css\";").unwrap();
        std::fs::write(root.join("secret.css"), ".secret { color: red; }").unwrap();

        let source_root = std::fs::canonicalize(&src).unwrap();
        assert!(bundle_file(&src.join("style.css"), &source_root).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
