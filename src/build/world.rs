use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use comemo::{Track, Tracked};
use termcolor::NoColor;
use typst::diag::{FileError, SourceDiagnostic, SourceResult, Warned};
use typst::engine::{Route, Sink, Traced};
use typst::foundations::{Bytes, Content, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
use typst_kit::fonts::FontStore;

use crate::foundation::Project;
use crate::foundation::files::{FileAccessError, ProjectFiles, list_typst_files};

/// A project-bound Typst build session.
///
/// The project invariant, shared resources, input libraries, world construction,
/// evaluation, HTML compilation, and source-aware diagnostics live here. Callers
/// never construct or track a Typst world themselves.
pub struct TypstSession {
    project: Project,
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
    pub fn new(project: Project) -> Self {
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

    pub fn project(&self) -> &Project {
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
            .map(|warning| format_warning(&world, warning))
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
            .map(|warning| format_warning(&world, warning))
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
    anyhow::anyhow!("{context}\n{}", format_diagnostics(world, diagnostics))
}

fn format_diagnostics(
    world: &impl DiagnosticWorld,
    source_diagnostics: &[SourceDiagnostic],
) -> String {
    let mut buffer = Vec::new();
    {
        let mut writer = NoColor::new(&mut buffer);
        if diagnostics::emit(
            &mut writer,
            world,
            source_diagnostics.iter(),
            DiagnosticFormat::Human,
        )
        .is_err()
        {
            for diagnostic in source_diagnostics {
                let _ = writeln!(writer, "error: {diagnostic:?}");
            }
        }
    }
    String::from_utf8_lossy(&buffer).trim_end().to_owned()
}

fn format_warning(world: &impl DiagnosticWorld, warning: &SourceDiagnostic) -> String {
    let formatted = format_diagnostics(world, std::slice::from_ref(warning));
    formatted
        .strip_prefix("warning: ")
        .unwrap_or(&formatted)
        .to_owned()
}

struct CompileWorld<'a> {
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
        match id.root() {
            VirtualRoot::Project => id.vpath().get_without_slash().into(),
            VirtualRoot::Package(package) => {
                format!("{package}{}", id.vpath().get_with_slash())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_compilation_is_reused_and_invalidated_by_dependency_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let marker = root.file_name().unwrap().to_string_lossy();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = Project::open(root.to_owned()).unwrap();
        let entry = project.src_dir().join("index.typ");
        let dependency = project.src_dir().join("data.typ");
        std::fs::write(&entry, "#import \"data.typ\": marker\n#let value = marker").unwrap();
        std::fs::write(&dependency, format!("#let marker = \"first-{marker}\"")).unwrap();

        let mut session = TypstSession::new(project);
        let library = session.library(Dict::new());

        session.compile_page(&entry, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());

        session.reset();
        session.compile_page(&entry, &library).unwrap();
        assert!(comemo::testing::last_was_hit());

        std::fs::write(&dependency, format!("#let marker = \"second-{marker}\"")).unwrap();
        session.reset();
        session.compile_page(&entry, &library).unwrap();
        assert!(!comemo::testing::last_was_hit());
    }
}
