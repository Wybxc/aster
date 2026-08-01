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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::content;

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
    fn tracked_lists_are_reused_and_invalidated_by_directory_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src/blog")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        std::fs::write(root.join("src/index.typ"), "").unwrap();

        let project = ProjectRoot::new(root.to_owned()).unwrap();
        let mut session = TypstSession::new(project);
        let first = session.source_files().unwrap();
        assert!(!comemo::testing::last_was_hit());
        assert_eq!(first.len(), 1);

        session.reset();
        let repeated = session.source_files().unwrap();
        assert!(comemo::testing::last_was_hit());
        assert_eq!(repeated, first);

        std::fs::write(root.join("src/blog/post.typ"), "").unwrap();
        session.reset();
        let changed = session.source_files().unwrap();
        assert!(!comemo::testing::last_was_hit());
        assert_eq!(changed.len(), 2);
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

        session.compile_page(&entry, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());

        session.reset();
        session.compile_page(&entry, &library).unwrap();
        assert!(comemo::testing::last_was_hit());
        let dependencies = session.dependencies();
        assert!(dependencies.contains(&std::fs::canonicalize(&entry).unwrap()));
        assert!(dependencies.contains(&std::fs::canonicalize(&dependency).unwrap()));

        std::fs::write(&dependency, format!("#let marker = \"second-{marker}\"")).unwrap();
        session.reset();
        session.compile_page(&entry, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());
    }

    #[test]
    fn dynamic_content_imports_only_invalidate_dependent_pages() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let marker = root.file_name().unwrap().to_string_lossy();
        std::fs::create_dir_all(root.join("content/blog")).unwrap();
        std::fs::create_dir_all(root.join("lib/aster")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        std::fs::write(
            root.join("lib/aster/content.typ"),
            include_str!("../../templates/default/lib/aster/content.typ"),
        )
        .unwrap();
        let content_entry = root.join("content/blog/post.typ");
        std::fs::write(&content_entry, format!("= First {marker}")).unwrap();
        let dependent = root.join("src/dependent.typ");
        std::fs::write(
            &dependent,
            concat!(
                "#import \"/lib/aster/content.typ\": get-entry\n",
                "#let post = get-entry(\"blog\", \"post\")\n",
                "#let rendered = post.render()\n",
                "#html.elem(\"p\")[#rendered.content]\n",
            ),
        )
        .unwrap();
        let independent = root.join("src/independent.typ");
        std::fs::write(&independent, format!("#html.elem(\"p\")[{marker}]")).unwrap();

        let project = ProjectRoot::new(root.to_owned()).unwrap();
        let mut session = TypstSession::new(project.clone());
        let inputs = content::install(Dict::new(), content::load(&session).unwrap()).unwrap();
        let library = session.library(inputs);
        session.compile_page(&dependent, &library).unwrap();
        session.compile_page(&independent, &library).unwrap();

        std::fs::write(&content_entry, format!("= Second {marker}")).unwrap();
        session.reset();
        let inputs = content::install(Dict::new(), content::load(&session).unwrap()).unwrap();
        let library = session.library(inputs);

        session.compile_page(&independent, &library).unwrap();
        assert!(comemo::testing::last_was_hit());

        let compiled = session.compile_page(&dependent, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());
        let html =
            typst_html::html(&compiled.document, &typst_html::HtmlOptions::default()).unwrap();
        assert!(html.contains(&format!("Second {marker}")));
    }
}
