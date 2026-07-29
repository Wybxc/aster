use std::sync::Arc;

use termcolor::{ColorChoice, StandardStream};
use typst::diag::{FileError, Severity, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration};
use typst::syntax::{FileId, Span};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
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

/// Print Typst diagnostics to stderr using the given [`DiagnosticWorld`].
pub fn emit_diags(world: &impl DiagnosticWorld, diags: &[SourceDiagnostic]) {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if diagnostics::emit(&mut writer, world, diags.iter(), DiagnosticFormat::Human).is_err() {
        for diag in diags {
            eprintln!("error: {diag:?}");
        }
    }
}

/// A [`DiagnosticWorld`] backed by a [`FileStore`], usable for emitting
/// diagnostics that may have source-level span information.
pub struct FileStoreWorld {
    files: Arc<FileStore<SystemFiles>>,
}

impl FileStoreWorld {
    pub fn new(files: Arc<FileStore<SystemFiles>>) -> Self {
        Self { files }
    }
}

impl World for FileStoreWorld {
    fn library(&self) -> &LazyHash<typst::Library> {
        panic!("FileStoreWorld does not provide a library")
    }
    fn book(&self) -> &LazyHash<FontBook> {
        panic!("FileStoreWorld does not provide a font book")
    }
    fn main(&self) -> FileId {
        panic!("FileStoreWorld does not have a main file")
    }
    fn source(&self, id: FileId) -> std::result::Result<typst::syntax::Source, FileError> {
        self.files.source(id)
    }
    fn file(&self, id: FileId) -> std::result::Result<Bytes, FileError> {
        self.files.file(id)
    }
    fn font(&self, _index: usize) -> Option<Font> {
        None
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

impl DiagnosticWorld for FileStoreWorld {
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

/// Emit a simple message as a [`SourceDiagnostic`] without source location.
pub fn emit_message(world: &impl DiagnosticWorld, severity: Severity, message: &str) {
    let diag = SourceDiagnostic {
        severity,
        span: Span::detached().into(),
        message: message.into(),
        trace: typst::ecow::eco_vec![],
        hints: typst::ecow::eco_vec![],
    };
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    let _ = diagnostics::emit(&mut writer, world, Some(&diag), DiagnosticFormat::Human);
}
