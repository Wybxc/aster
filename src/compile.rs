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

/// Shared resources (fonts, file store) reused across multiple compile
/// calls for the same project.
///
/// Font scanning (~1.75 s) is the bottleneck — done once here.
///
/// The [`Library`] (which carries `sys.inputs`) is *not* included because
/// content entries and pages need different inputs.  Callers construct the
/// appropriate library and pass it down.
pub struct SharedCompile {
    fonts: Arc<FontStore>,
    files: Arc<FileStore<SystemFiles>>,
}

impl SharedCompile {
    /// Build shared resources for one project root.
    pub fn new(project_root: &Path) -> Self {
        let fonts = Arc::new(build_font_store());
        let files = Arc::new(build_file_store(project_root));
        Self { fonts, files }
    }

    /// Create a per-entry world recycling fonts/files.
    pub(crate) fn world(
        &self,
        entry: &Path,
        project_root: &Path,
        library: &LazyHash<Library>,
    ) -> CompileWorld {
        let vpath = VirtualPath::virtualize(project_root, entry)
            .expect("entry must be inside project root");
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();
        CompileWorld {
            library: LazyHash::new((**library).clone()),
            fonts: Arc::clone(&self.fonts),
            files: Arc::clone(&self.files),
            main,
        }
    }

    /// Compile a source file to raw [`Content`].
    pub fn compile_content(
        &self,
        entry: &Path,
        project_root: &Path,
        library: &LazyHash<Library>,
    ) -> Result<Content> {
        compile_content_with(self, entry, project_root, library)
    }

    /// Compile a source file to an [`HtmlDocument`].
    pub fn compile_document(
        &self,
        entry: &Path,
        project_root: &Path,
        library: &LazyHash<Library>,
    ) -> Result<HtmlDocument> {
        compile_document_with(self, entry, project_root, library)
    }
}

// ---------------------------------------------------------------------------
// World adapter
// ---------------------------------------------------------------------------

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

/// Build library with the HTML feature enabled and the given `sys.inputs`.
pub fn build_library(inputs: Dict) -> Library {
    let features: Features = [Feature::Html].into_iter().collect();
    Library::builder()
        .with_inputs(inputs)
        .with_features(features)
        .build()
}

fn build_file_store(project_root: &Path) -> FileStore<SystemFiles> {
    let downloader = SystemDownloader::new("aster/0.1.0");
    let packages = SystemPackages::new(downloader);
    let project = FsRoot::new(project_root.to_owned());
    let system_files = SystemFiles::new(project, packages);
    FileStore::new(system_files)
}

fn build_font_store() -> FontStore {
    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::system());
    fonts
}

// ---------------------------------------------------------------------------
// Low-level: compile a single file into an HtmlDocument.
// ---------------------------------------------------------------------------

fn compile_document_with(
    shared: &SharedCompile,
    entry: &Path,
    project_root: &Path,
    library: &LazyHash<Library>,
) -> Result<HtmlDocument> {
    let world = shared.world(entry, project_root, library);
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

fn compile_content_with(
    shared: &SharedCompile,
    entry: &Path,
    project_root: &Path,
    library: &LazyHash<Library>,
) -> Result<Content> {
    let world = shared.world(entry, project_root, library);

    let source = world
        .source(world.main())
        .map_err(|e| anyhow::anyhow!("failed to load source: {e}"))?;

    let mut sink = Sink::new();
    let traced = Traced::default();

    let module = typst_eval::eval(
        (&world as &dyn World).track(),
        library,
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

pub fn run(
    entry: &Path,
    project_root: &Path,
    shared: &SharedCompile,
    library: &LazyHash<Library>,
) -> Result<String> {
    let mut doc = shared.compile_document(entry, project_root, library)?;

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
