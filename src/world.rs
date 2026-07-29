use std::sync::Arc;

use typst::diag::FileError;
use typst::foundations::{Bytes, Datetime, Dict, Duration};
use typst::syntax::FileId;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::files::{FileStore, SystemFiles};
use typst_kit::fonts::FontStore;

/// A Typst [`World`] backed by Aster's project resources.
///
/// Each instance wraps a specific entry file together with the shared fonts
/// and file store managed by [`CompileContext`](crate::compile::CompileContext).
pub(crate) struct CompileWorld {
    pub(crate) library: LazyHash<Library>,
    pub(crate) fonts: Arc<FontStore>,
    pub(crate) files: Arc<FileStore<SystemFiles>>,
    pub(crate) main: FileId,
}

impl World for CompileWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> Result<typst::syntax::Source, FileError> {
        self.files.source(id)
    }

    fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

impl DiagnosticWorld for CompileWorld {
    fn name(&self, id: FileId) -> String {
        let cwd = std::env::current_dir().ok();
        self.files
            .loader()
            .resolve(id)
            .ok()
            .and_then(|p| {
                cwd.as_ref()
                    .and_then(|cwd| p.strip_prefix(cwd).ok())
                    .map(|p| p.display().to_string())
                    .or_else(|| Some(p.display().to_string()))
            })
            .unwrap_or_else(|| id.vpath().get_with_slash().to_string())
    }
}

/// Build a [`Library`] with the HTML feature enabled and the given `sys.inputs`.
pub fn build_library(inputs: Dict) -> Library {
    let features: Features = [Feature::Html].into_iter().collect();
    Library::builder()
        .with_inputs(inputs)
        .with_features(features)
        .build()
}
