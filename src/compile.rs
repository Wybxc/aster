use std::path::Path;

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
    fonts: FontStore,
    files: FileStore<SystemFiles>,
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
        let files = {
            let downloader = SystemDownloader::new("aster/0.1.0");
            let packages = SystemPackages::new(downloader);
            let fs_root = FsRoot::new(project.root().to_owned());
            FileStore::new(SystemFiles::new(fs_root, packages))
        };
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
            project_root: self.project.root(),
            library,
            fonts: &self.fonts,
            files: &self.files,
            main,
        })
    }
}

#[comemo::memoize]
fn compile_html(world: Tracked<dyn World + '_>) -> Warned<SourceResult<typst_html::HtmlDocument>> {
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
    files: &'a FileStore<SystemFiles>,
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
            .loader()
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn page_compilation_is_reused_and_invalidated_by_dependency_changes() {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("aster-compile-test-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let entry = root.join("src/index.typ");
        let dependency = root.join("src/data.typ");
        std::fs::write(&entry, "#import \"data.typ\": marker\n#let value = marker").unwrap();
        std::fs::write(&dependency, format!("#let marker = \"first-{id}\"")).unwrap();

        let project = ProjectRoot::new(root.clone()).unwrap();
        let mut session = TypstSession::new(project);
        let library = session.library(Dict::new());

        session.compile_page(&entry, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());

        session.reset();
        session.compile_page(&entry, &library).unwrap();
        assert!(comemo::testing::last_was_hit());

        std::fs::write(&dependency, format!("#let marker = \"second-{id}\"")).unwrap();
        session.reset();
        session.compile_page(&entry, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());

        let _ = std::fs::remove_dir_all(root);
    }
}
