use std::path::PathBuf;

use clap::{Parser, Subcommand};
use typst::diag::{FileError, SourceResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::{HtmlDocument, HtmlOptions};
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

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

#[derive(Parser)]
#[command(name = "aster", version, about = "Aster build system")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a typst file to HTML
    Build {
        /// The typst file to compile
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { file } => {
            // Canonicalize the input file path to turn it into an absolute path
            // from which we can derive the project root.
            let file = std::path::absolute(&file).unwrap_or_else(|e| {
                eprintln!("error: failed to resolve '{}': {}", file.display(), e);
                std::process::exit(1);
            });

            let project_root = file.parent().unwrap();
            let vpath = VirtualPath::virtualize(project_root, &file).unwrap_or_else(
                |e| {
                    eprintln!("error: invalid path: {e}");
                    std::process::exit(1);
                },
            );
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
            let project = FsRoot::new(project_root.to_owned());
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
                    let html = typst_html::html(&doc, &HtmlOptions::default()).unwrap_or_else(
                        |e| {
                            eprintln!("error: failed to encode HTML: {e:?}");
                            std::process::exit(1);
                        },
                    );
                    println!("{html}");
                }
                Err(errors) => {
                    for err in &errors {
                        eprintln!("error: {err:?}");
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}
