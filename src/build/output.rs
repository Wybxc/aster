use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use comemo::Tracked;
use typst::ecow::EcoString;
use typst::foundations::Bytes;
use typst::syntax::VirtualPath;

use crate::build::files::ProjectFiles;
use crate::engine::route::RoutePath;
use crate::foundation::{Project, ProjectLayout};

/// Compute a compact 64-bit content fingerprint for generated asset URLs.
fn content_hash(data: &[u8]) -> u64 {
    seahash::hash(data)
}

/// The stable output location of a generated asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPath(RoutePath);

/// The publication-layer result returned to the build pipeline.
///
/// Publication writes pages, generated files, and internal assets. Only the two
/// user-authored route kinds are surfaced as build results.
pub struct PublishedOutput {
    /// Root directory containing the complete published site.
    pub output_dir: PathBuf,
    /// Published page paths in deterministic route order; generated assets are omitted.
    pub pages: Vec<PathBuf>,
    /// Published generator output paths in deterministic route order.
    pub generated: Vec<PathBuf>,
}

#[derive(Eq, PartialEq)]
enum OutputFile {
    Page(Vec<u8>),
    Generated(Bytes),
    Postprocessed(Bytes),
    Asset(Bytes),
    Public(Bytes),
}

impl OutputFile {
    fn content(&self) -> &[u8] {
        match self {
            Self::Page(content) => content,
            Self::Generated(content)
            | Self::Postprocessed(content)
            | Self::Asset(content)
            | Self::Public(content) => content.as_slice(),
        }
    }

    fn is_page(&self) -> bool {
        matches!(self, Self::Page(_))
    }

    fn is_generated(&self) -> bool {
        matches!(self, Self::Generated(_))
    }
}

/// Collects an entire successful build before replacing `dist/`.
///
/// Asset identity, browser references, output confinement, stale-file removal,
/// and filesystem publication all live behind this module's interface.
pub struct OutputPublication {
    project_root: PathBuf,
    output_dir: PathBuf,
    assets_dir: RoutePath,
    files: BTreeMap<RoutePath, OutputFile>,
}

impl OutputPublication {
    pub fn new(project: &Project, layout: &ProjectLayout) -> Result<Self> {
        let assets_dir = RoutePath::new(layout.generated_assets().get_without_slash())
            .context("invalid output assets directory")?;
        Ok(Self {
            project_root: project.root().to_owned(),
            output_dir: project.realize(layout.output()),
            assets_dir,
            files: BTreeMap::new(),
        })
    }

    /// Register the generated highlight stylesheet.
    pub fn add_highlight_stylesheet(&mut self, content: Bytes) -> Result<AssetPath> {
        self.add_asset(Some("highlight"), Some("css"), content)
    }

    /// Collect the tracked public tree at the output root.
    pub fn add_public_tree(
        &mut self,
        project_files: Tracked<ProjectFiles>,
        root: &VirtualPath,
    ) -> Result<()> {
        let source_root = Path::new(root.get_without_slash());
        let paths = project_files.list(root, false)?;
        let count = paths.len();
        for source in paths {
            let relative = Path::new(source.get_without_slash())
                .strip_prefix(source_root)
                .context("public file is outside configured public directory")?;
            let content = project_files.read(&source).with_context(|| {
                format!("failed to read public file {}", source.get_with_slash())
            })?;
            let path = RoutePath::new(relative).context("invalid public file path")?;
            self.insert(path, OutputFile::Public(content))?;
        }
        tracing::debug!(
            files = count,
            "collected {count} public file{}",
            if count == 1 { "" } else { "s" }
        );
        Ok(())
    }

    /// Register a generator result at its exact output route.
    pub fn add_generator_output(&mut self, path: RoutePath, content: Bytes) -> Result<()> {
        self.insert(path, OutputFile::Generated(content))
    }

    fn add_asset(
        &mut self,
        name: Option<&str>,
        extension: Option<&str>,
        content: Bytes,
    ) -> Result<AssetPath> {
        let hash = content_hash(content.as_slice());
        let filename = match (name, extension) {
            (Some(name), Some(extension)) => format!("{name}.{hash:016x}.{extension}"),
            (Some(name), None) => format!("{name}.{hash:016x}"),
            (None, Some(extension)) => format!("{hash:016x}.{extension}"),
            (None, None) => format!("{hash:016x}"),
        };
        let path =
            PathBuf::from(self.assets_dir.as_virtual_path().get_without_slash()).join(filename);
        let path = RoutePath::new(path)?;
        self.insert(path.clone(), OutputFile::Asset(content))?;
        Ok(AssetPath(path))
    }

    pub fn page<'a>(
        &'a mut self,
        template: &VirtualPath,
        output: &'a RoutePath,
    ) -> PagePublication<'a> {
        PagePublication {
            publication: self,
            template: template.clone(),
            output,
        }
    }

    /// Write the complete build into a temporary tree beside the destination.
    pub fn stage(self) -> Result<StagedPublication> {
        let file_count = self.files.len();
        let byte_count = self
            .files
            .values()
            .map(|file| file.content().len())
            .sum::<usize>();
        tracing::debug!(
            files = file_count,
            bytes = byte_count,
            destination = %self.output_dir.display(),
            "staging {file_count} file{} ({byte_count} bytes)",
            if file_count == 1 { "" } else { "s" }
        );
        let parent = self
            .output_dir
            .parent()
            .context("output directory has no parent")?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        let staging = tempfile::Builder::new()
            .prefix(".aster-stage-")
            .tempdir_in(parent)
            .context("failed to create output staging directory")?;
        for (relative, file) in &self.files {
            let path = realize_output_path(staging.path(), relative)?;
            write_file(&path, file.content())?;
        }

        Ok(StagedPublication {
            output_dir: self.output_dir,
            staging,
            files: self.files,
        })
    }

    fn insert(&mut self, path: RoutePath, file: OutputFile) -> Result<()> {
        insert_file(&mut self.files, path, file)
    }
}

/// A complete, unpublished site available to external postprocessors.
pub struct StagedPublication {
    output_dir: PathBuf,
    staging: tempfile::TempDir,
    files: BTreeMap<RoutePath, OutputFile>,
}

impl StagedPublication {
    /// Return the temporary site root passed to external tools.
    pub fn root(&self) -> &Path {
        self.staging.path()
    }

    /// Import one private postprocessor output tree under `mount`.
    pub fn import(&mut self, mount: &str, source: &Path) -> Result<()> {
        ensure!(
            source.is_dir(),
            "postprocessor did not create its output directory"
        );
        let mount = RoutePath::new(mount).context("invalid postprocessor mount")?;
        let mount = Path::new(mount.as_virtual_path().get_without_slash());
        for entry in walkdir::WalkDir::new(source).sort_by_file_name() {
            let entry = entry.context("failed to inspect postprocessor output")?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(source)
                .context("postprocessor output escaped its directory")?;
            let route = RoutePath::new(mount.join(relative))?;
            let destination = realize_output_path(self.root(), &route)?;
            ensure!(
                !destination.exists(),
                "postprocessor output conflicts with existing staged file {}",
                route
            );
            let content = Bytes::new(std::fs::read(entry.path()).with_context(|| {
                format!(
                    "failed to read postprocessor output {}",
                    entry.path().display()
                )
            })?);
            insert_file(
                &mut self.files,
                route.clone(),
                OutputFile::Postprocessed(content.clone()),
            )?;
            write_file(&destination, content.as_slice())?;
        }
        Ok(())
    }

    /// Select this staged tree as the new published output.
    pub fn publish(self) -> Result<PublishedOutput> {
        let page_routes = self
            .files
            .iter()
            .filter(|(_, file)| file.is_page())
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let generated_routes = self
            .files
            .iter()
            .filter(|(_, file)| file.is_generated())
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();

        remove_if_exists(&self.output_dir)?;
        let staging = self.staging.keep();
        std::fs::rename(&staging, &self.output_dir).with_context(|| {
            format!(
                "failed to publish {} to {}",
                staging.display(),
                self.output_dir.display()
            )
        })?;

        let pages = page_routes
            .iter()
            .map(|path| realize_output_path(&self.output_dir, path))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|path| path.is_file())
            .collect();
        let generated = generated_routes
            .iter()
            .map(|path| realize_output_path(&self.output_dir, path))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|path| path.is_file())
            .collect();

        Ok(PublishedOutput {
            output_dir: self.output_dir,
            pages,
            generated,
        })
    }
}

fn insert_file(
    files: &mut BTreeMap<RoutePath, OutputFile>,
    path: RoutePath,
    file: OutputFile,
) -> Result<()> {
    if let Some(existing) = files.get(&path) {
        ensure!(
            existing == &file,
            "two published files selected the same output path {}",
            path
        );
        return Ok(());
    }
    if let Some(existing) = files.keys().find(|existing| existing.conflicts_with(&path)) {
        anyhow::bail!(
            "published files selected conflicting output paths {} and {}",
            existing,
            path
        );
    }
    files.insert(path, file);
    Ok(())
}

/// Per-page access to output publication policy.
pub struct PagePublication<'a> {
    publication: &'a mut OutputPublication,
    template: VirtualPath,
    output: &'a RoutePath,
}

impl PagePublication<'_> {
    pub fn template(&self) -> &VirtualPath {
        &self.template
    }

    /// Resolve a source reference relative to the template within the project virtual root.
    pub fn resolve_source(&self, reference: &str) -> Result<VirtualPath> {
        self.resolve_source_from(&self.template, reference)
    }

    /// Resolve a source reference relative to a known file in the project virtual root.
    pub fn resolve_source_from(
        &self,
        origin: &VirtualPath,
        reference: &str,
    ) -> Result<VirtualPath> {
        if reference.starts_with('/') {
            return VirtualPath::new(reference)
                .with_context(|| format!("invalid project-root source reference {reference}"));
        }

        let origin_dir = origin.parent().context("source file has no parent")?;
        origin_dir.join(reference).with_context(|| {
            format!(
                "source reference {reference} escapes project root {}",
                self.publication.project_root.display()
            )
        })
    }

    pub fn project_root(&self) -> &Path {
        &self.publication.project_root
    }

    pub fn page_url_path(&self) -> EcoString {
        self.output.page_url_path()
    }

    /// Register the styles used by this page's highlighted code.
    pub fn add_highlight_stylesheet(&mut self, content: Bytes) -> Result<EcoString> {
        let asset = self.publication.add_highlight_stylesheet(content)?;
        self.reference(&asset)
    }

    /// Register a bundled stylesheet under the entry file's name.
    pub fn add_bundled_stylesheet(
        &mut self,
        entry: &VirtualPath,
        content: Bytes,
    ) -> Result<EcoString> {
        let name = entry
            .file_stem()
            .context("stylesheet entry has no file name")?;
        let asset = self
            .publication
            .add_asset(Some(name), Some("css"), content)?;
        self.reference(&asset)
    }

    /// Register a local `url()` dependency and return its URL from generated CSS.
    pub fn add_css_asset(&mut self, source: &VirtualPath, content: Bytes) -> Result<EcoString> {
        let name = source.file_stem().context("CSS asset has no file name")?;
        let asset = self
            .publication
            .add_asset(Some(name), source.extension(), content)?;
        Ok(asset
            .0
            .as_virtual_path()
            .relative_from(self.publication.assets_dir.as_virtual_path()))
    }

    /// Register a project resource under its original file name and return its page URL.
    pub fn add_asset(&mut self, source: &VirtualPath, content: Bytes) -> Result<EcoString> {
        let name = source.file_stem().context("asset has no file name")?;
        let asset = self
            .publication
            .add_asset(Some(name), source.extension(), content)?;
        self.reference(&asset)
    }

    /// Register a component script under the source file's name.
    pub fn add_script(&mut self, source: &VirtualPath, content: Bytes) -> Result<EcoString> {
        let name = source
            .file_stem()
            .context("script entry has no file name")?;
        let asset = self
            .publication
            .add_asset(Some(name), Some("js"), content)?;
        self.reference(&asset)
    }

    /// Register an extracted data URL and return its browser-facing URL from this page.
    pub fn add_data_asset(&mut self, extension: Option<&str>, content: Bytes) -> Result<EcoString> {
        let asset = self.publication.add_asset(None, extension, content)?;
        self.reference(&asset)
    }

    /// Return a browser-facing URL from this page to an existing generated asset.
    pub fn reference(&self, asset: &AssetPath) -> Result<EcoString> {
        let page_dir = self
            .output
            .as_virtual_path()
            .parent()
            .context("output page has no parent")?;
        Ok(asset.0.as_virtual_path().relative_from(&page_dir))
    }

    /// Return a browser-facing URL from this page to a site-root path.
    pub fn site_reference(&self, path: &str) -> EcoString {
        debug_assert!(path.starts_with('/') && !path.starts_with("//"));
        let page_dir = self
            .output
            .as_virtual_path()
            .parent()
            .expect("output page must have a parent");
        let mut reference = EcoString::new();
        for _ in page_dir
            .get_without_slash()
            .split('/')
            .filter(|segment| !segment.is_empty())
        {
            reference.push_str("../");
        }
        reference.push_str(
            path.strip_prefix('/')
                .expect("site-root path must begin with a slash"),
        );
        if reference.is_empty() {
            reference.push_str("./");
        }
        reference
    }

    /// Add the final serialized page to this publication.
    pub fn add_html(self, html: String) -> Result<()> {
        self.publication
            .insert(self.output.clone(), OutputFile::Page(html.into_bytes()))
    }
}

fn realize_output_path(root: &Path, path: &RoutePath) -> Result<PathBuf> {
    path.as_virtual_path()
        .realize(root)
        .with_context(|| format!("failed to realize generated output path {path}"))
}

fn write_file(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove {}", path.display())),
        Ok(_) => std::fs::remove_file(path)
            .with_context(|| format!("failed to remove {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to inspect {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::config::AsterConfig;

    fn fixture() -> (tempfile::TempDir, Project, ProjectLayout) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("pages/blog")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = Project::open(root.to_owned()).unwrap();
        let layout = ProjectLayout::new(&AsterConfig::default()).unwrap();
        (temp, project, layout)
    }

    fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut snapshot = BTreeMap::new();
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let virtual_path = VirtualPath::virtualize(root, entry.path()).unwrap();
                let relative = PathBuf::from(virtual_path.get_without_slash());
                snapshot.insert(relative, std::fs::read(entry.path()).unwrap());
            }
        }
        snapshot
    }

    #[test]
    fn asset_urls_follow_the_page_output_layout() {
        let (_temp, project, layout) = fixture();
        let mut publication = OutputPublication::new(&project, &layout).unwrap();
        let asset = publication
            .add_highlight_stylesheet(Bytes::from_string("body{}"))
            .unwrap();
        let template = VirtualPath::new("/pages/blog/[slug].typ").unwrap();
        let directory_output = RoutePath::new("blog/post/index.html").unwrap();
        let page = publication.page(&template, &directory_output);

        assert!(
            page.reference(&asset)
                .unwrap()
                .starts_with("../../_assets/highlight.")
        );

        let file_output = RoutePath::new("blog/post.html").unwrap();
        let page = publication.page(&template, &file_output);
        assert!(
            page.reference(&asset)
                .unwrap()
                .starts_with("../_assets/highlight.")
        );
    }

    #[test]
    fn source_resolution_supports_template_and_project_root_references() {
        let (_temp, project, layout) = fixture();
        std::fs::write(project.root().join("style.css"), "body{}").unwrap();
        std::fs::create_dir(project.root().join("styles")).unwrap();
        std::fs::write(project.root().join("styles/site.css"), "body{}").unwrap();
        let template = VirtualPath::new("/pages/blog/[slug].typ").unwrap();
        let mut publication = OutputPublication::new(&project, &layout).unwrap();
        let output = RoutePath::new("blog/post/index.html").unwrap();
        let page = publication.page(&template, &output);

        assert_eq!(
            page.resolve_source("../../style.css").unwrap(),
            VirtualPath::new("/style.css").unwrap()
        );
        assert_eq!(
            page.resolve_source("/styles/site.css").unwrap(),
            VirtualPath::new("/styles/site.css").unwrap()
        );
        assert!(page.resolve_source("../../../outside.css").is_err());
    }

    #[test]
    fn publication_is_idempotent_and_removes_stale_output() {
        let (temp, project, layout) = fixture();
        let output_dir = project.realize(layout.output());
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(output_dir.join("stale.html"), "old").unwrap();

        let mut publication = OutputPublication::new(&project, &layout).unwrap();
        let first = publication
            .add_highlight_stylesheet(Bytes::from_string("body{}"))
            .unwrap();
        let second = publication
            .add_highlight_stylesheet(Bytes::from_string("body{}"))
            .unwrap();
        assert_eq!(first, second);

        let template = VirtualPath::new("/pages/index.typ").unwrap();
        let output = RoutePath::new("index.html").unwrap();
        publication
            .page(&template, &output)
            .add_html("new".into())
            .unwrap();
        let published = publication.stage().unwrap().publish().unwrap();

        assert_eq!(published.pages, vec![output_dir.join("index.html")]);
        assert!(published.generated.is_empty());

        let expected = snapshot_tree(&output_dir);
        std::fs::write(output_dir.join("stale.html"), "old").unwrap();

        let mut repeated = OutputPublication::new(&project, &layout).unwrap();
        assert_eq!(
            repeated
                .add_highlight_stylesheet(Bytes::from_string("body{}"))
                .unwrap(),
            first
        );
        repeated
            .page(&template, &output)
            .add_html("new".into())
            .unwrap();
        repeated.stage().unwrap().publish().unwrap();

        assert_eq!(snapshot_tree(&output_dir), expected);
        assert!(!temp.path().join(".dist.aster-lock").exists());
        assert_eq!(expected.len(), 2);
    }

    #[test]
    fn empty_publication_replaces_output_with_empty_directory() {
        let (_temp, project, layout) = fixture();
        let output_dir = project.realize(layout.output());
        std::fs::create_dir_all(&output_dir).unwrap();
        std::fs::write(output_dir.join("stale.html"), "old").unwrap();

        OutputPublication::new(&project, &layout)
            .unwrap()
            .stage()
            .unwrap()
            .publish()
            .unwrap();

        assert!(output_dir.is_dir());
        assert_eq!(std::fs::read_dir(output_dir).unwrap().count(), 0);
    }
}
