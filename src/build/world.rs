use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use comemo::{Track, Tracked};
use termcolor::NoColor;
use typst::diag::{FileError, SourceDiagnostic, SourceResult, Warned};
use typst::ecow::EcoString;
use typst::foundations::Duration;
use typst::foundations::{Bytes, Datetime, Label, Selector, Value};
use typst::introspection::{Introspector, MetadataElem};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_kit::datetime::Time;
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
use typst_kit::fonts::FontStore;

use crate::build::files::ProjectFiles;
use crate::build::{BuildSession, BuildWarning};
use crate::engine::content::Runtime;
use crate::foundation::ProjectLayout;
use crate::foundation::config::FontConfig;

pub fn configure_fonts(
    session: &mut BuildSession,
    config: &FontConfig,
    layout: &ProjectLayout,
) -> Result<()> {
    let directories = layout
        .font_dirs()
        .map(|path| session.files.directory(path))
        .collect::<Result<Vec<_>, _>>()?;
    let refresh = session.font_config.as_ref() != Some(config) || !config.paths.is_empty();
    if refresh {
        session.fonts = discover_fonts(config, directories.iter().map(PathBuf::as_path));
        session.font_config = Some(config.clone());
    }
    let count = session
        .fonts
        .book()
        .families()
        .map(|(_, variants)| variants.count())
        .sum::<usize>();
    if refresh {
        tracing::debug!(
            fonts = count,
            "discovered {count} font face{}",
            if count == 1 { "" } else { "s" }
        );
    } else {
        tracing::debug!(
            fonts = count,
            "reused {count} font face{}",
            if count == 1 { "" } else { "s" }
        );
    }
    Ok(())
}

pub fn compile_document(
    session: &BuildSession,
    entry: &VirtualPath,
    runtime: &Runtime,
) -> Result<(typst_html::HtmlDocument, Vec<BuildWarning>)> {
    let world = world(session, entry, runtime.library());
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

pub fn evaluate_generator(
    session: &BuildSession,
    entry: &VirtualPath,
    runtime: &Runtime,
) -> Result<(Bytes, Vec<BuildWarning>)> {
    let (document, warnings) = compile_document(session, entry, runtime)?;
    let selector = Selector::Label(
        Label::construct("aster-output".into()).expect("generator label is non-empty"),
    );
    let mut declarations = document
        .introspector()
        .query(&selector)
        .into_iter()
        .filter_map(|element| {
            element
                .to_packed::<MetadataElem>()
                .map(|metadata| metadata.value.clone())
        });
    let output = match (declarations.next(), declarations.next()) {
        (_, Some(_)) => {
            anyhow::bail!("generator must contain exactly one <aster-output> declaration")
        }
        (None, None) => anyhow::bail!("generator did not produce <aster-output>"),
        (Some(Value::Str(content)), None) => Bytes::from_string(content),
        (Some(Value::Bytes(content)), None) => content,
        (Some(_), None) => anyhow::bail!("generator output must be a string or bytes"),
    };
    Ok((output, warnings))
}

fn world<'a>(
    session: &'a BuildSession,
    entry: &VirtualPath,
    library: &'a LazyHash<Library>,
) -> CompileWorld<'a> {
    let main = RootedPath::new(VirtualRoot::Project, entry.clone()).intern();
    CompileWorld {
        library,
        fonts: &session.fonts,
        files: &session.files,
        main,
        now: &session.now,
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
