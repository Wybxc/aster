use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
    files: ProjectFiles,
}

pub(crate) struct ProjectFiles {
    root: PathBuf,
    store: FileStore<SystemFiles>,
    tracked_paths: Mutex<BTreeSet<PathBuf>>,
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

    pub fn compile_page(
        &self,
        entry: &Path,
        output: &Path,
        library: &LazyHash<Library>,
    ) -> Result<CompiledPage> {
        let world = self.world(entry, library)?;
        let warned = compile_html((&world as &dyn World).track(), output);
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

impl ProjectFiles {
    fn new(project: &ProjectRoot) -> Self {
        let root =
            std::fs::canonicalize(project.root()).unwrap_or_else(|_| project.root().to_owned());
        let downloader = SystemDownloader::new("aster/0.1.0");
        let packages = SystemPackages::new(downloader);
        let fs_root = FsRoot::new(root.clone());
        let store = FileStore::new(SystemFiles::new(fs_root, packages));
        Self {
            root,
            store,
            tracked_paths: Mutex::new(BTreeSet::new()),
        }
    }

    fn reset(&mut self) {
        self.store.reset();
        self.tracked_paths
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    fn dependencies(&mut self) -> Vec<PathBuf> {
        let mut paths = self
            .tracked_paths
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let (loader, dependencies) = self.store.dependencies();
        paths.extend(dependencies.filter_map(|id| loader.resolve(id).ok()));
        paths.sort();
        paths.dedup();
        paths
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn source(&self, id: FileId) -> Result<typst::syntax::Source, FileError> {
        self.store.source(id)
    }

    fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        self.store.file(id)
    }

    fn resolve(&self, id: FileId) -> Result<PathBuf, FileError> {
        self.store.loader().resolve(id)
    }

    fn track_path(&self, path: &Path) {
        self.tracked_paths
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(path.to_owned());
    }
}

#[comemo::track]
impl ProjectFiles {
    pub(crate) fn canonicalize(&self, path: &Path) -> Result<PathBuf, String> {
        self.track_path(path);
        std::fs::canonicalize(path)
            .map_err(|error| format!("failed to resolve {}: {error}", path.display()))
    }

    pub(crate) fn read(&self, path: &Path) -> Result<Bytes, String> {
        self.track_path(path);
        let virtual_path = VirtualPath::virtualize(&self.root, path).map_err(|error| {
            format!(
                "{} is outside {}: {error}",
                path.display(),
                self.root.display()
            )
        })?;
        let id = RootedPath::new(VirtualRoot::Project, virtual_path).intern();
        self.store
            .file(id)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))
    }
}

#[comemo::memoize]
fn compile_html(
    world: Tracked<dyn World + '_>,
    _output: &Path,
) -> Warned<SourceResult<typst_html::HtmlDocument>> {
    // Keep this inside the memoized body so cache hits remain quiet.
    #[cfg(not(test))]
    diag::emit_built_page(&_output.to_string_lossy());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependencies_include_missing_tracked_paths_and_reset_between_builds() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let missing = root.join("missing-theme.tmTheme");

        let project = ProjectRoot::new(root.to_owned()).unwrap();
        let mut session = TypstSession::new(project);
        assert!(session.project_files().canonicalize(&missing).is_err());
        assert!(session.dependencies().contains(&missing));

        session.reset();
        assert!(session.dependencies().is_empty());
    }

    #[test]
    fn page_compilation_is_reused_and_invalidated_by_dependency_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let marker = root.file_name().unwrap().to_string_lossy();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let entry = root.join("src/index.typ");
        let dependency = root.join("src/data.typ");
        std::fs::write(&entry, "#import \"data.typ\": marker\n#let value = marker").unwrap();
        std::fs::write(&dependency, format!("#let marker = \"first-{marker}\"")).unwrap();

        let project = ProjectRoot::new(root.to_owned()).unwrap();
        let mut session = TypstSession::new(project);
        let library = session.library(Dict::new());

        session
            .compile_page(&entry, Path::new("index.html"), &library)
            .unwrap();
        assert!(!comemo::testing::last_was_hit());

        session.reset();
        session
            .compile_page(&entry, Path::new("index.html"), &library)
            .unwrap();
        assert!(comemo::testing::last_was_hit());
        let dependencies = session.dependencies();
        assert!(dependencies.contains(&std::fs::canonicalize(&entry).unwrap()));
        assert!(dependencies.contains(&std::fs::canonicalize(&dependency).unwrap()));

        std::fs::write(&dependency, format!("#let marker = \"second-{marker}\"")).unwrap();
        session.reset();
        session
            .compile_page(&entry, Path::new("index.html"), &library)
            .unwrap();
        assert!(!comemo::testing::last_was_hit());
    }
}
