use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::{FileError, SourceResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::{HtmlDocument, HtmlOptions};
use typst_kit::diagnostics::{DiagnosticFormat, DiagnosticWorld};
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

/// Search upward from `dir` for an `aster.toml` file.
fn find_project_root(dir: &Path) -> Option<PathBuf> {
    let mut current = Some(dir);
    while let Some(path) = current {
        if path.join("aster.toml").exists() {
            return Some(path.to_owned());
        }
        current = path.parent();
    }
    None
}

/// Recursively collect all `.typ` files under `dir`.
fn find_typ_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_typ_files(&path));
            } else if path.extension().map_or(false, |ext| ext == "typ") {
                files.push(path);
            }
        }
    }
    files
}

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

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the project
    Build,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            // Search upward from the current directory for an aster.toml.
            let cwd = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("error: failed to get current directory: {e}");
                std::process::exit(1);
            });
            let project_root = find_project_root(&cwd).unwrap_or_else(|| {
                eprintln!("error: no aster.toml found in current or parent directories");
                std::process::exit(1);
            });

            let src_dir = project_root.join("src");
            if !src_dir.is_dir() {
                eprintln!("error: src/ directory not found in project");
                std::process::exit(1);
            }

            let typ_files = find_typ_files(&src_dir);
            if typ_files.is_empty() {
                eprintln!("error: no .typ files found in src/");
                std::process::exit(1);
            }

            // Build library with the HTML feature enabled (shared across files).
            let features: Features = [Feature::Html].into_iter().collect();
            let library = Library::builder().with_features(features).build();

            let mut has_errors = false;

            for entry in &typ_files {
                let relative = entry
                    .strip_prefix(&src_dir)
                    .expect("file must be under src/");
                let output_relative = relative.with_extension("html");
                let output_path = project_root.join("dist").join(&output_relative);

                // Build a fresh world for this entry point.
                let world = build_world(entry, &project_root, &library);

                let warned = typst::compile::<HtmlDocument>(&world);
                let result: SourceResult<HtmlDocument> = warned.output;

                match result {
                    Ok(doc) => {
                        let html = typst_html::html(&doc, &HtmlOptions::default())
                            .unwrap_or_else(|e| {
                                eprintln!("error: failed to encode HTML: {e:?}");
                                std::process::exit(1);
                            });

                        // Ensure output directory exists.
                        if let Some(parent) = output_path.parent() {
                            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                                eprintln!(
                                    "error: failed to create directory {}: {e}",
                                    parent.display()
                                );
                                std::process::exit(1);
                            });
                        }

                        std::fs::write(&output_path, &html).unwrap_or_else(|e| {
                            eprintln!("error: failed to write {}: {e}", output_path.display());
                            std::process::exit(1);
                        });
                    }
                    Err(errors) => {
                        has_errors = true;
                        eprintln!("error: failed to build {}", output_relative.display());
                        let mut diagnostic_writer = StandardStream::stderr(ColorChoice::Auto);
                        if typst_kit::diagnostics::emit(
                            &mut diagnostic_writer,
                            &world,
                            &errors,
                            DiagnosticFormat::Human,
                        )
                        .is_err()
                        {
                            for diag in &errors {
                                eprintln!("error: {diag:?}");
                            }
                        }
                    }
                }
            }

            if has_errors {
                std::process::exit(1);
            }
        }
    }
}

fn build_world(entry: &Path, project_root: &Path, library: &Library) -> CompileWorld {
    let vpath = VirtualPath::virtualize(project_root, entry).unwrap_or_else(|e| {
        eprintln!("error: invalid path: {e}");
        std::process::exit(1);
    });
    let main = RootedPath::new(VirtualRoot::Project, vpath).intern();

    // Set up font discovery (system fonts, lazily loaded).
    let mut fonts = FontStore::new();
    fonts.extend(typst_kit::fonts::system());

    // Set up package resolution (local data → cache → universe).
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
