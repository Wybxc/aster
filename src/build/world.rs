use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use comemo::{Track, Tracked};
use termcolor::NoColor;
use typst::diag::{FileError, SourceDiagnostic, SourceResult, Warned};
use typst::ecow::EcoVec;
use typst::engine::{Route, Sink, Traced};
use typst::foundations::{Bytes, Content, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_kit::datetime::Time;
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
use typst_kit::fonts::FontStore;

use crate::build::BuildWarning;
use crate::foundation::config::FontConfig;
use crate::foundation::files::{FileAccessError, ProjectFiles, list_typst_files};
use crate::foundation::{FilesystemDependency, Project, ProjectLayout};

/// A project-bound Typst build session.
///
/// The project invariant, shared resources, input libraries, world construction,
/// evaluation, HTML compilation, and source-aware diagnostics live here. Callers
/// never construct or track a Typst world themselves.
pub struct TypstSession {
    project: Project,
    font_config: Option<FontConfig>,
    fonts: FontStore,
    files: ProjectFiles,
    now: Time,
}

impl TypstSession {
    pub fn new(project: Project) -> Self {
        let files = ProjectFiles::new(&project);
        Self {
            project,
            font_config: None,
            fonts: FontStore::new(),
            files,
            now: Time::system(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn reset(&mut self) {
        self.files.reset();
        self.now.reset();
    }

    pub(crate) fn project_files(&self) -> Tracked<'_, ProjectFiles> {
        self.files.track()
    }

    pub(crate) fn configure_fonts(
        &mut self,
        config: &FontConfig,
        layout: &ProjectLayout,
    ) -> Result<()> {
        let directories = layout
            .font_dirs()
            .map(|path| self.files.directory(path))
            .collect::<Result<Vec<_>, _>>()?;
        if self.font_config.as_ref() != Some(config) || !config.paths.is_empty() {
            self.fonts = discover_fonts(config, directories.iter().map(PathBuf::as_path));
            self.font_config = Some(config.clone());
        }
        Ok(())
    }

    pub(crate) fn source_files(
        &self,
        layout: &ProjectLayout,
    ) -> Result<EcoVec<VirtualPath>, FileAccessError> {
        list_typst_files(self.project_files(), layout.source(), true)
    }

    pub(crate) fn content_files(
        &self,
        layout: &ProjectLayout,
    ) -> Result<EcoVec<VirtualPath>, FileAccessError> {
        list_typst_files(self.project_files(), layout.content(), false)
    }

    pub(crate) fn dependencies(&mut self) -> Vec<FilesystemDependency> {
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

    pub fn evaluate(
        &self,
        entry: &VirtualPath,
        library: &LazyHash<Library>,
    ) -> Result<(Content, Vec<BuildWarning>)> {
        let world = self.world(entry, library);
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
        Ok((module.content(), warnings))
    }

    pub fn compile_page(
        &self,
        entry: &VirtualPath,
        library: &LazyHash<Library>,
    ) -> Result<(typst_html::HtmlDocument, Vec<BuildWarning>)> {
        let world = self.world(entry, library);
        let warned = compile_html((&world as &dyn World).track());
        let document = warned
            .output
            .map_err(|diagnostics| diagnostic_error(&world, "compilation failed", &diagnostics))?;
        let warnings = warned
            .warnings
            .iter()
            .map(|warning| format_warning(&world, warning))
            .collect();
        Ok((document, warnings))
    }

    fn world<'a>(
        &'a self,
        entry: &VirtualPath,
        library: &'a LazyHash<Library>,
    ) -> CompileWorld<'a> {
        let main = RootedPath::new(VirtualRoot::Project, entry.clone()).intern();
        CompileWorld {
            library,
            fonts: &self.fonts,
            files: &self.files,
            main,
            now: &self.now,
        }
    }
}

fn discover_fonts<'a>(
    config: &FontConfig,
    directories: impl IntoIterator<Item = &'a std::path::Path>,
) -> FontStore {
    let mut fonts = FontStore::new();
    if config.system {
        fonts.extend(typst_kit::fonts::system());
    }
    for directory in directories {
        fonts.extend(typst_kit::fonts::scan(directory));
    }
    fonts
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

fn format_warning(world: &impl DiagnosticWorld, warning: &SourceDiagnostic) -> BuildWarning {
    let formatted = format_diagnostics(world, std::slice::from_ref(warning));
    BuildWarning::new(formatted.strip_prefix("warning: ").unwrap_or(&formatted))
}

struct CompileWorld<'a> {
    library: &'a LazyHash<Library>,
    fonts: &'a FontStore,
    files: &'a ProjectFiles,
    main: FileId,
    now: &'a Time,
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

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.now.today(offset)
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
