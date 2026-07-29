use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, bail};
use typst::comemo::Track;
use typst::engine::{Route, Sink, Traced};
use typst::foundations::Content;
use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_html::HtmlDocument;
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

use crate::diag::emit_diags;
use crate::project::ProjectRoot;
use crate::world::CompileWorld;

/// Reusable compilation context for a project.
///
/// Holds the expensive shared state (fonts, file store) so that per-entry
/// [`CompileWorld`] instances can be created cheaply.
///
/// Font scanning (~1.75 s) is the bottleneck — done once here.
///
/// The [`Library`] (which carries `sys.inputs`) is *not* included because
/// content entries and pages need different inputs.  Callers construct the
/// appropriate library and pass it to [`CompileContext::world`].
pub struct CompileContext {
    fonts: Arc<FontStore>,
    files: Arc<FileStore<SystemFiles>>,
}

impl CompileContext {
    /// Build shared resources for one project root.
    pub fn new(project: &ProjectRoot) -> Self {
        let fonts = Arc::new({
            let mut fonts = FontStore::new();
            fonts.extend(typst_kit::fonts::system());
            fonts
        });
        let files = Arc::new({
            let downloader = SystemDownloader::new("aster/0.1.0");
            let packages = SystemPackages::new(downloader);
            let fs_root = FsRoot::new(project.root().to_owned());
            let system_files = SystemFiles::new(fs_root, packages);
            FileStore::new(system_files)
        });
        Self { fonts, files }
    }

    /// Create a per-entry world recycling fonts/files.
    pub(crate) fn world(
        &self,
        entry: &Path,
        project: &ProjectRoot,
        library: &LazyHash<Library>,
    ) -> CompileWorld {
        let vpath = VirtualPath::virtualize(project.root(), entry)
            .expect("entry must be inside project root");
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();
        CompileWorld {
            library: library.clone(),
            fonts: Arc::clone(&self.fonts),
            files: Arc::clone(&self.files),
            main,
        }
    }

    /// Evaluate a source file to raw [`Content`].
    pub fn content(
        &self,
        entry: &Path,
        project: &ProjectRoot,
        library: &LazyHash<Library>,
    ) -> Result<Content> {
        let world = self.world(entry, project, library);

        let source = world
            .source(world.main())
            .map_err(|e| anyhow::anyhow!("failed to load source: {e}"))?;

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
        .map_err(|diags| {
            emit_diags(&world, &diags);
            anyhow::anyhow!("evaluation failed")
        })?;

        Ok(module.content())
    }

    /// Compile a source file to an [`HtmlDocument`].
    pub fn document(
        &self,
        entry: &Path,
        project: &ProjectRoot,
        library: &LazyHash<Library>,
    ) -> Result<HtmlDocument> {
        let world = self.world(entry, project, library);
        let warned = typst::compile::<HtmlDocument>(&world);

        match warned.output {
            Ok(doc) => Ok(doc),
            Err(errors) => {
                emit_diags(&world, &errors);
                bail!("compilation failed");
            }
        }
    }
}
