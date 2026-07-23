use std::path::PathBuf;

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
fn find_project_root(dir: &std::path::Path) -> Option<PathBuf> {
    let mut current = Some(dir);
    while let Some(path) = current {
        if path.join("aster.toml").exists() {
            return Some(path.to_owned());
        }
        current = path.parent();
    }
    None
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

            // The entry point is src/index.typ relative to the project root.
            let entry = project_root.join("src").join("index.typ");
            if !entry.exists() {
                eprintln!("error: src/index.typ not found in project");
                std::process::exit(1);
            }

            let vpath = VirtualPath::virtualize(&project_root, &entry).unwrap_or_else(|e| {
                eprintln!("error: invalid path: {e}");
                std::process::exit(1);
            });
            let main = RootedPath::new(VirtualRoot::Project, vpath).intern();

            // Build library with the HTML feature enabled.
            let features: Features = [Feature::Html].into_iter().collect();
            let library = Library::builder().with_features(features).build();

            // Set up font discovery (system fonts, lazily loaded).
            let mut fonts = FontStore::new();
            fonts.extend(typst_kit::fonts::system());

            // Set up package resolution (local data → cache → universe).
            let downloader = SystemDownloader::new("aster/0.1.0");
            let packages = SystemPackages::new(downloader);
            let project = FsRoot::new(project_root);
            let system_files = SystemFiles::new(project, packages);

            let world = CompileWorld {
                library: LazyHash::new(library),
                fonts,
                files: FileStore::new(system_files),
                main,
            };

            // Compile to HTML.
            let warned = typst::compile::<HtmlDocument>(&world);
            let result: SourceResult<HtmlDocument> = warned.output;

            match result {
                Ok(doc) => {
                    let html =
                        typst_html::html(&doc, &HtmlOptions::default()).unwrap_or_else(|e| {
                            eprintln!("error: failed to encode HTML: {e:?}");
                            std::process::exit(1);
                        });
                    println!("{html}");
                }
                Err(errors) => {
                    let mut diagnostic_writer = StandardStream::stderr(ColorChoice::Auto);
                    if typst_kit::diagnostics::emit(
                        &mut diagnostic_writer,
                        &world,
                        &errors,
                        DiagnosticFormat::Human,
                    )
                    .is_err()
                    {
                        // Fallback: print raw diagnostic.
                        for err in &errors {
                            eprintln!("error: {err:?}");
                        }
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}
