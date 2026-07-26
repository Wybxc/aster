use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, bail};
use termcolor::{ColorChoice, StandardStream};
use typst::comemo::Track;
use typst::diag::{FileError, SourceDiagnostic};
use typst::engine::{Route, Sink, Traced};
use typst::foundations::{Bytes, Content, Datetime, Dict, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Features, Library, LibraryExt, World};
use typst_html::{HtmlDocument, HtmlElement, HtmlNode};
use typst_kit::diagnostics::{self, DiagnosticFormat, DiagnosticWorld};
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::FontStore;
use typst_kit::packages::SystemPackages;

// ---------------------------------------------------------------------------
// World builder
// ---------------------------------------------------------------------------

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
    pub fn new(project_root: &Path) -> Self {
        let fonts = Arc::new({
            let mut fonts = FontStore::new();
            fonts.extend(typst_kit::fonts::system());
            fonts
        });
        let files = Arc::new({
            let downloader = SystemDownloader::new("aster/0.1.0");
            let packages = SystemPackages::new(downloader);
            let project = FsRoot::new(project_root.to_owned());
            let system_files = SystemFiles::new(project, packages);
            FileStore::new(system_files)
        });
        Self { fonts, files }
    }

    /// Create a per-entry world recycling fonts/files.
    pub(crate) fn world(
        &self,
        entry: &Path,
        project_root: &Path,
        library: &LazyHash<Library>,
    ) -> CompileWorld {
        let vpath = VirtualPath::virtualize(project_root, entry)
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
        project_root: &Path,
        library: &LazyHash<Library>,
    ) -> Result<Content> {
        let world = self.world(entry, project_root, library);

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
        project_root: &Path,
        library: &LazyHash<Library>,
    ) -> Result<HtmlDocument> {
        let world = self.world(entry, project_root, library);
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
}

/// Walk all `<link>` elements in the document.  For each one, read the `rel`
/// and `href` attributes and call [`crate::loader::try_bundle`].  Elements
/// whose `rel` is not recognised are left untouched.
///
/// # Dedup
///
/// Currently each page that references `style.css` bundles it independently.
/// A dedup layer (cache the hashed name per href) can be added later.
pub fn process_css_refs(doc: &mut HtmlDocument, src_dir: &Path, dist_dir: &Path) -> Result<()> {
    walk_and_bundle(doc.root_mut(), src_dir, dist_dir)
}

fn walk_and_bundle(elem: &mut HtmlElement, src_dir: &Path, dist_dir: &Path) -> Result<()> {
    if elem.tag == typst_html::tag::link {
        // Read rel and href attributes (immutable pass first).
        let rel = elem
            .attrs
            .0
            .iter()
            .find(|(a, _)| *a.resolve() == *"rel")
            .map(|(_, v)| v.clone());

        let href = elem
            .attrs
            .0
            .iter()
            .find(|(a, _)| *a.resolve() == *"href")
            .map(|(_, v)| v.clone());

        let Some(rel) = rel else { return Ok(()) };
        let Some(href) = href else { return Ok(()) };

        // Try to bundle; unrecognised rel values (e.g. "stylesheet")
        // return `None` and the element is left untouched.
        if let Some(result) = crate::loader::try_bundle(&rel, &href, src_dir, dist_dir)? {
            for (a, v) in elem.attrs.0.make_mut().iter_mut() {
                if *a.resolve() == *"href" {
                    *v = result.href.clone().into();
                } else if *a.resolve() == *"rel" {
                    *v = result.rel.clone().into();
                }
            }
        }
    }
    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(e) = child {
            walk_and_bundle(e, src_dir, dist_dir)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// World adapter
// ---------------------------------------------------------------------------

pub(crate) struct CompileWorld {
    library: LazyHash<Library>,
    fonts: Arc<FontStore>,
    files: Arc<FileStore<SystemFiles>>,
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
// Construction helpers
// ---------------------------------------------------------------------------

/// Build library with the HTML feature enabled and the given `sys.inputs`.
pub fn build_library(inputs: Dict) -> Library {
    let features: Features = [Feature::Html].into_iter().collect();
    Library::builder()
        .with_inputs(inputs)
        .with_features(features)
        .build()
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
