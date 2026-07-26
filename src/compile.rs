use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst::comemo::Track;
use typst::diag::{FileError, SourceDiagnostic};
use typst::engine::{Route, Sink, Traced};
use typst::foundations::{Bytes, Content, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::{HtmlDocument, HtmlOptions};
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

use crate::highlight;

// ---------------------------------------------------------------------------
// Shared compilation environment
// ---------------------------------------------------------------------------

/// Shared resources (fonts, file store, library) reused across multiple
/// compile calls for the same project.
///
/// Fonts are the most expensive part — discovered once.
pub struct SharedCompile {
    library: LazyHash<Library>,
    fonts: Arc<FontStore>,
    files: Arc<FileStore<SystemFiles>>,
}

impl SharedCompile {
    /// Build everything for one project root.
    pub fn new(inputs: Dict, project_root: &Path) -> Self {
        let library = build_library(inputs);
        let fonts = Arc::new(build_font_store());
        let files = Arc::new(build_file_store(project_root));
        let library = LazyHash::new(library);
        Self { library, fonts, files }
    }

    /// Create a per-entry world reusing fonts, files, and library.
    pub(crate) fn world(&self, entry: &Path, project_root: &Path) -> CompileWorld {
        let vpath = VirtualPath::virtualize(project_root, entry)
            .expect("entry must be inside project root");
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();
        CompileWorld {
            library: LazyHash::new((*self.library).clone()),
            fonts: Arc::clone(&self.fonts),
            files: Arc::clone(&self.files),
            main,
        }
    }

    /// Convenience: compile a source file to raw [`Content`].
    pub fn compile_content(&self, entry: &Path, project_root: &Path) -> Result<Content> {
        compile_content_with(self, entry, project_root)
    }

    /// Convenience: compile a source file to an [`HtmlDocument`].
    pub fn compile_document(&self, entry: &Path, project_root: &Path) -> Result<HtmlDocument> {
        compile_document_with(self, entry, project_root)
    }
}

// ---------------------------------------------------------------------------
// World adapter (private to this module)
// ---------------------------------------------------------------------------

/// A World that compiles a single file, sharing fonts/files via `Arc`.
pub(crate) struct CompileWorld {
    library: LazyHash<Library>,
    fonts: Arc<FontStore>,
    files: Arc<FileStore<SystemFiles>>,
    main: FileId,
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

    fn source(&self, id: FileId) -> Result<Source, FileError> {
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

// ---------------------------------------------------------------------------
// Construction helpers
// ---------------------------------------------------------------------------

/// Build library with the HTML feature enabled.
fn build_library(inputs: Dict) -> Library {
    let features: Features = [Feature::Html].into_iter().collect();
    Library::builder()
        .with_inputs(inputs)
        .with_features(features)
        .build()
}

/// Build a file store for the project root.
fn build_file_store(project_root: &Path) -> FileStore<SystemFiles> {
    let downloader = SystemDownloader::new("aster/0.1.0");
    let packages = SystemPackages::new(downloader);
    let project = FsRoot::new(project_root.to_owned());
    let system_files = SystemFiles::new(project, packages);
    FileStore::new(system_files)
}

/// Build and cache system fonts (expensive — scans ~2s).
fn build_font_store() -> FontStore {
    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::system());
    fonts
}

// ---------------------------------------------------------------------------
// Low-level: compile a single file into an HtmlDocument.
// ---------------------------------------------------------------------------

/// Compile a file to [`HtmlDocument`] using the shared environment.
fn compile_document_with(
    shared: &SharedCompile,
    entry: &Path,
    project_root: &Path,
) -> Result<HtmlDocument> {
    let world = shared.world(entry, project_root);
    let warned = typst::compile::<HtmlDocument>(&world);
    emit_diags(&world, &warned.warnings);

    match warned.output {
        Ok(doc) => Ok(doc),
        Err(errors) => {
            emit_diags(&world, &errors);
            bail!("compilation failed");
        }
    }
}

// ---------------------------------------------------------------------------
// Compile a file to raw Content (for content entries embedded in sys.inputs).
// ---------------------------------------------------------------------------

/// Compile a file and return its raw evaluated `Content` using the shared
/// environment.
fn compile_content_with(
    shared: &SharedCompile,
    entry: &Path,
    project_root: &Path,
) -> Result<Content> {
    let world = shared.world(entry, project_root);

    let source = world
        .source(world.main())
        .map_err(|e| anyhow::anyhow!("failed to load source: {e}"))?;

    let mut sink = Sink::new();
    let traced = Traced::default();

    let module = typst_eval::eval(
        (&world as &dyn World).track(),
        &shared.library,
        traced.track(),
        sink.track_mut(),
        Route::default().track(),
        &source,
    )
    .map_err(|diags| {
        emit_diags(&world, &diags);
        anyhow::anyhow!("evaluation failed")
    })?;

    Ok(module.content())
}

// ---------------------------------------------------------------------------
// High-level: compile a page → serialized HTML string.
// ---------------------------------------------------------------------------

pub fn run(entry: &Path, project_root: &Path, inputs: Dict) -> Result<String> {
    let shared = SharedCompile::new(inputs, project_root);
    let mut doc = shared.compile_document(entry, project_root)?;

    highlight::rehighlight(&mut doc);

    let raw = typst_html::html(&doc, &HtmlOptions::default())
        .map_err(|_| anyhow::anyhow!("failed to encode HTML"))?;

    Ok(raw.to_owned())
}

// ---------------------------------------------------------------------------
// Diagnostic printing
// ---------------------------------------------------------------------------

fn emit_diags(world: &impl DiagnosticWorld, diags: &[SourceDiagnostic]) {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if diagnostics::emit(&mut writer, world, diags.iter(), DiagnosticFormat::Human).is_err() {
        for diag in diags {
            eprintln!("error: {diag:?}");
        }
    }
}
