use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use comemo::{Track, Tracked};
use typst::diag::{FileError, SourceDiagnostic, SourceResult, Warned};
use typst::engine::{Route, Sink, Traced};
use typst::foundations::{Bytes, Content, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::fonts::FontStore;

use crate::cli::diag;
use crate::foundation::files::{FileAccessError, ProjectFiles, list_typst_files};
use crate::foundation::project::ProjectRoot;

/// A project-bound Typst build session.
///
/// The project invariant, shared resources, input libraries, world construction,
/// evaluation, HTML compilation, and source-aware diagnostics live here. Callers
/// never construct or track a Typst world themselves.
pub struct TypstSession {
    project: ProjectRoot,
    fonts: FontStore,
    files: ProjectFiles,
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
        let fonts = {
            let mut fonts = FontStore::new();
            fonts.extend(typst_kit::fonts::system());
            fonts
        };
        let files = ProjectFiles::new(&project);
        Self {
            project,
            fonts,
            files,
        }
    }

    pub fn project(&self) -> &ProjectRoot {
        &self.project
    }

    pub fn reset(&mut self) {
        self.files.reset();
    }

    pub(crate) fn project_files(&self) -> Tracked<'_, ProjectFiles> {
        self.files.track()
    }

    pub(crate) fn source_files(&self) -> Result<Vec<PathBuf>, FileAccessError> {
        list_typst_files(self.project_files(), &self.project.src_dir(), true)
    }

    pub(crate) fn content_files(&self) -> Result<Vec<PathBuf>, FileAccessError> {
        list_typst_files(self.project_files(), &self.project.content_dir(), false)
    }

    pub(crate) fn dependencies(&mut self) -> Vec<PathBuf> {
        self.files.dependencies()
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
        let warned = compile_html((&world as &dyn World).track());
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

    fn world<'a>(
        &'a self,
        entry: &Path,
        library: &'a LazyHash<Library>,
    ) -> Result<CompileWorld<'a>> {
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
            project_root: self.files.root(),
            library,
            fonts: &self.fonts,
            files: &self.files,
            main,
        })
    }
}

#[comemo::memoize]
fn compile_html(world: Tracked<dyn World + '_>) -> Warned<SourceResult<typst_html::HtmlDocument>> {
    #[cfg(not(test))]
    diag::emit_built_page(Path::new(world.main().vpath().get_without_slash()));
    typst::compile::<typst_html::HtmlDocument>(&*world)
}

fn diagnostic_error(
    world: &CompileWorld<'_>,
    context: &str,
    diagnostics: &[SourceDiagnostic],
) -> anyhow::Error {
    anyhow::anyhow!("{context}\n{}", diag::format_diags(world, diagnostics))
}

struct CompileWorld<'a> {
    project_root: &'a Path,
    library: &'a LazyHash<Library>,
    fonts: &'a FontStore,
    files: &'a ProjectFiles,
    main: FileId,
}

impl World for CompileWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.library
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

impl DiagnosticWorld for CompileWorld<'_> {
    fn name(&self, id: FileId) -> String {
        self.files
            .resolve(id)
            .ok()
            .map(|path| {
                path.strip_prefix(self.project_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string()
            })
            .unwrap_or_else(|| id.vpath().get_with_slash().to_string())
    }
}
