use std::path::Path;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::{FileError, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Dict, Duration};
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
// World adapter (private to this module)
// ---------------------------------------------------------------------------

/// A World that compiles a single project with package support.
struct CompileWorld {
    library: LazyHash<Library>,
    fonts: FontStore,
    files: FileStore<SystemFiles>,
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
// Library & world construction
// ---------------------------------------------------------------------------

/// Build library with the HTML feature enabled and optional inputs from
/// `aster.toml`.
fn build_library(inputs: Dict) -> Library {
    let features: Features = [Feature::Html].into_iter().collect();
    Library::builder()
        .with_inputs(inputs)
        .with_features(features)
        .build()
}

/// Build a fresh world for the given entry point.
fn build_world(entry: &Path, project_root: &Path, library: &Library) -> CompileWorld {
    let vpath =
        VirtualPath::virtualize(project_root, entry).expect("entry must be inside project root");
    let main = RootedPath::new(VirtualRoot::Project, vpath).intern();

    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::system());

    let downloader = SystemDownloader::new("aster/0.1.0");
    let packages = SystemPackages::new(downloader);
    let project = FsRoot::new(project_root.to_owned());
    let system_files = SystemFiles::new(project, packages);

    CompileWorld {
        library: LazyHash::new(library.clone()),
        fonts,
        files: FileStore::new(system_files),
        main,
    }
}

// ---------------------------------------------------------------------------
// Low-level: compile a single file into an HtmlDocument (shared by pages and
// content entries).  Diagnostics are printed to stderr automatically.
// ---------------------------------------------------------------------------

pub fn compile_document(entry: &Path, project_root: &Path, inputs: Dict) -> Result<HtmlDocument> {
    let library = build_library(inputs);
    let world = build_world(entry, project_root, &library);

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
// High-level: compile a page → serialized HTML string.
// ---------------------------------------------------------------------------

pub fn run(entry: &Path, project_root: &Path, inputs: Dict) -> Result<String> {
    let mut doc = compile_document(entry, project_root, inputs)?;

    highlight::rehighlight(&mut doc);

    let raw = typst_html::html(&doc, &HtmlOptions::default())
        .map_err(|_| anyhow::anyhow!("failed to encode HTML"))?;

    Ok(raw
        .strip_prefix("<!DOCTYPE html>")
        .unwrap_or(&raw)
        .to_owned())
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
