use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use comemo::Track;
use typst::diag::{FileError, SourceDiagnostic};
use typst::engine::{Route, Sink, Traced};
use typst::foundations::{Bytes, Content, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

use crate::diag;
use crate::project::ProjectRoot;

/// A project-bound Typst build session.
///
/// The project invariant, shared resources, input libraries, world construction,
/// evaluation, HTML compilation, and source-aware diagnostics live here. Callers
/// never construct or track a Typst world themselves.
pub struct TypstSession {
    project: ProjectRoot,
    fonts: Arc<FontStore>,
    files: Arc<FileStore<SystemFiles>>,
}

pub struct EvaluatedContent {
    pub content: Content,
    pub warnings: Vec<String>,
}

pub struct CompiledPage {
    pub document: typst_html::HtmlDocument,
    pub warnings: Vec<String>,
}

impl TypstSession {
    pub fn new(project: ProjectRoot) -> Self {
        let fonts = Arc::new({
            let mut fonts = FontStore::new();
            fonts.extend(typst_kit::fonts::system());
            fonts
        });
        let files = Arc::new({
            let downloader = SystemDownloader::new("aster/0.1.0");
            let packages = SystemPackages::new(downloader);
            let fs_root = FsRoot::new(project.root().to_owned());
            FileStore::new(SystemFiles::new(fs_root, packages))
        });
        Self {
            project,
            fonts,
            files,
        }
    }

    pub fn project(&self) -> &ProjectRoot {
        &self.project
    }

    pub fn library(&self, inputs: Dict) -> LazyHash<Library> {
        LazyHash::new(
            Library::builder()
                .with_inputs(inputs)
                .with_features([Feature::Html].into_iter().collect())
                .build(),
        )
    }

    pub fn evaluate(&self, entry: &Path, library: &LazyHash<Library>) -> Result<EvaluatedContent> {
        let world = self.world(entry, library)?;
        let source = world
            .source(world.main())
            .map_err(|error| anyhow::anyhow!("failed to load source: {error}"))?;
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
        .map_err(|diagnostics| diagnostic_error(&world, "evaluation failed", &diagnostics))?;
        let warnings = sink
            .warnings()
            .iter()
            .map(|warning| diag::format_warning(&world, warning))
            .collect();
        Ok(EvaluatedContent {
            content: module.content(),
            warnings,
        })
    }

    pub fn compile_page(&self, entry: &Path, library: &LazyHash<Library>) -> Result<CompiledPage> {
        let world = self.world(entry, library)?;
        let warned = typst::compile::<typst_html::HtmlDocument>(&world);
        let document = warned
            .output
            .map_err(|diagnostics| diagnostic_error(&world, "compilation failed", &diagnostics))?;
        let warnings = warned
            .warnings
            .iter()
            .map(|warning| diag::format_warning(&world, warning))
            .collect();
        Ok(CompiledPage { document, warnings })
    }

    fn world(&self, entry: &Path, library: &LazyHash<Library>) -> Result<CompileWorld> {
        let virtual_path =
            VirtualPath::virtualize(self.project.root(), entry).with_context(|| {
                format!(
                    "entry {} must be inside project {}",
                    entry.display(),
                    self.project.root().display()
                )
            })?;
        let main = RootedPath::new(VirtualRoot::Project, virtual_path).intern();
        Ok(CompileWorld {
            project_root: self.project.root().to_owned(),
            library: library.clone(),
            fonts: Arc::clone(&self.fonts),
            files: Arc::clone(&self.files),
            main,
        })
    }
}

fn diagnostic_error(
    world: &CompileWorld,
    context: &str,
    diagnostics: &[SourceDiagnostic],
) -> anyhow::Error {
    anyhow::anyhow!("{context}\n{}", diag::format_diags(world, diagnostics))
}

struct CompileWorld {
    project_root: PathBuf,
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
        self.files
            .loader()
            .resolve(id)
            .ok()
            .map(|path| {
                path.strip_prefix(&self.project_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
            })
            .unwrap_or_else(|| id.vpath().get_with_slash().to_string())
    }
}
