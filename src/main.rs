use std::path::PathBuf;

use clap::{Parser, Subcommand};
use typst::diag::{FileError, SourceResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::{HtmlDocument, HtmlOptions};

/// A minimal World that compiles a single file.
struct CompileWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    main: FileId,
    source: Source,
    fonts: Vec<Font>,
}

impl World for CompileWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> Result<Source, FileError> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(PathBuf::new()))
        }
    }

    fn file(&self, _id: FileId) -> Result<Bytes, FileError> {
        Err(FileError::NotFound(PathBuf::new()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
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
            let content = std::fs::read_to_string(&file).unwrap_or_else(|e| {
                eprintln!("error: failed to read '{}': {}", file.display(), e);
                std::process::exit(1);
            });

            // Build library with the HTML feature enabled.
            let features: Features = [Feature::Html].into_iter().collect();
            let library = Library::builder().with_features(features).build();

            // Discover system fonts using fontdb.
            let mut fontdb = fontdb::Database::new();
            fontdb.load_system_fonts();

            let mut fonts = Vec::new();
            let mut infos = Vec::new();

            for face in fontdb.faces() {
                let face_data: Option<Vec<u8>> = match &face.source {
                    fontdb::Source::Binary(arc) => {
                        let slice: &[u8] = (**arc).as_ref().as_ref();
                        Some(slice.to_vec())
                    }
                    fontdb::Source::File(path) => std::fs::read(path).ok(),
                    fontdb::Source::SharedFile(_, arc) => {
                        let slice: &[u8] = (**arc).as_ref().as_ref();
                        Some(slice.to_vec())
                    }
                };

                if let Some(data) = face_data {
                    let bytes = Bytes::new(data);
                    for font in Font::iter(bytes) {
                        infos.push(font.info().clone());
                        fonts.push(font);
                    }
                }
            }

            let book = FontBook::from_infos(infos);

            // Set up the main source file.
            let vpath = VirtualPath::new(file.to_string_lossy()).unwrap_or_else(|e| {
                eprintln!("error: invalid path: {e}");
                std::process::exit(1);
            });
            let rooted = RootedPath::new(VirtualRoot::Project, vpath);
            let id = rooted.intern();
            let source = Source::new(id, content);

            let world = CompileWorld {
                library: LazyHash::new(library),
                book: LazyHash::new(book),
                main: id,
                source,
                fonts,
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
                    for err in &errors {
                        eprintln!("error: {err:?}");
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}
