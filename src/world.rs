use std::path::Path;

use typst::diag::FileError;
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

/// A World that compiles a single project with package support.
pub struct CompileWorld {
    pub library: LazyHash<Library>,
    pub fonts: FontStore,
    pub files: FileStore<SystemFiles>,
    pub main: FileId,
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

/// Build library with the HTML feature enabled.
pub fn build_library() -> Library {
    let features: Features = [Feature::Html].into_iter().collect();
    Library::builder().with_features(features).build()
}

/// Build a fresh world for the given entry point.
pub fn build_world(entry: &Path, project_root: &Path, library: &Library) -> CompileWorld {
    let vpath = VirtualPath::virtualize(project_root, entry)
        .expect("entry must be inside project root");
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
