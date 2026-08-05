use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use comemo::{Track, Tracked};
use termcolor::NoColor;
use typst::diag::{FileError, SourceDiagnostic, SourceResult, Warned};
use typst::ecow::{EcoString, EcoVec};
use typst::foundations::{Bytes, Datetime, Dict, Duration};
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

/// A reusable build session bound to one Aster project.
///
/// The project invariant, shared resources, input libraries, world construction,
/// HTML compilation and source-aware diagnostics live here. Callers
/// never construct or track a Typst world themselves.
pub struct BuildSession {
    project: Project,
    font_config: Option<FontConfig>,
    fonts: FontStore,
    files: ProjectFiles,
    now: Time,
}

impl BuildSession {
    /// Create a reusable session bound to a validated project.
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

    /// Return the project bound to this session.
    pub fn project(&self) -> &Project {
        &self.project
    }

    pub(crate) fn reset(&mut self) {
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

    pub(crate) fn observe_watch_paths(
        &self,
        layout: &ProjectLayout,
    ) -> Result<(), FileAccessError> {
        for path in layout.watch_paths() {
            self.files.watch(path)?;
        }
        Ok(())
    }

    pub(crate) fn route_templates(
        &self,
        layout: &ProjectLayout,
    ) -> Result<EcoVec<VirtualPath>, FileAccessError> {
        list_typst_files(self.project_files(), layout.pages(), true)
    }

    pub(crate) fn content_files(
        &self,
        layout: &ProjectLayout,
    ) -> Result<EcoVec<VirtualPath>, FileAccessError> {
        list_typst_files(self.project_files(), layout.content(), false)
    }

    /// Return the inputs observed by the latest build attempt.
    pub fn dependencies(&mut self) -> Vec<FilesystemDependency> {
        self.files.dependencies()
    }

    pub(crate) fn library(&self, inputs: Dict) -> LazyHash<Library> {
        LazyHash::new(
            Library::builder()
                .with_inputs(inputs)
                .with_features([Feature::Html].into_iter().collect())
                .build(),
        )
    }

    pub(crate) fn compile_document(
        &self,
        entry: &VirtualPath,
        library: &LazyHash<Library>,
    ) -> Result<(typst_html::HtmlDocument, Vec<BuildWarning>)> {
        let world = self.world(entry, library);
        let warned = compile_html((&world as &dyn World).track());
        let document = warned
            .output
            .map_err(|diagnostics| diagnostic_error(&world, "compilation failed", &diagnostics))?;

        const TYPST_HTML_WARNING: &str = "html export is under active development and incomplete";
        let warnings = warned
            .warnings
            .iter()
            .filter(|warning| warning.message.as_str() != TYPST_HTML_WARNING)
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
) -> EcoString {
    format_diagnostics_impl(world, source_diagnostics, "")
}

fn format_diagnostics_impl(
    world: &impl DiagnosticWorld,
    source_diagnostics: &[SourceDiagnostic],
    prefix: &str,
) -> EcoString {
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
    let output = String::from_utf8_lossy(&buffer);
    let output = output.trim_end();
    output.strip_prefix(prefix).unwrap_or(output).into()
}

fn format_warning(world: &impl DiagnosticWorld, warning: &SourceDiagnostic) -> BuildWarning {
    BuildWarning::new(format_diagnostics_impl(
        world,
        std::slice::from_ref(warning),
        "warning: ",
    ))
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
