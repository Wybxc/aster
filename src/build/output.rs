use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use typst::ecow::EcoString;
use typst::syntax::VirtualPath;

use crate::engine::route::RoutePath;
use crate::foundation::Project;

/// Compute a compact 64-bit content fingerprint for generated asset URLs.
fn content_hash(data: &[u8]) -> String {
    format!("{:016x}", seahash::hash(data))
}

/// The stable output location of a generated asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AssetPath(RoutePath);

/// The file format used for an extracted image asset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Svg,
    Webp,
    Avif,
    Binary,
}

impl ImageFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Svg => "svg",
            Self::Webp => "webp",
            Self::Avif => "avif",
            Self::Binary => "bin",
        }
    }
}

/// The publication-layer result returned to the build pipeline.
///
/// Publication writes both pages and generated assets, but only page paths are
/// part of the build outcome. Keeping that distinction here prevents the
/// pipeline from depending on the publication's internal file representation.
pub(crate) struct PublishedOutput {
    /// Published page paths in deterministic route order; generated assets are omitted.
    pub pages: Vec<PathBuf>,
}

#[derive(Eq, PartialEq)]
enum OutputFile {
    Page(Vec<u8>),
    Asset(Vec<u8>),
}

impl OutputFile {
    fn content(&self) -> &[u8] {
        match self {
            Self::Page(content) | Self::Asset(content) => content,
        }
    }

    fn is_page(&self) -> bool {
        matches!(self, Self::Page(_))
    }
}

/// Collects an entire successful build before replacing `dist/`.
///
/// Asset identity, browser references, output confinement, stale-file removal,
/// and filesystem publication all live behind this module's interface.
pub(crate) struct OutputPublication {
    project_root: PathBuf,
    output_dir: PathBuf,
    files: BTreeMap<RoutePath, OutputFile>,
}

impl OutputPublication {
    pub fn new(project: &Project) -> Self {
        Self {
            project_root: project.root().to_owned(),
            output_dir: project.output_dir(),
            files: BTreeMap::new(),
        }
    }

    /// Register the generated highlight stylesheet.
    pub fn add_highlight_stylesheet(&mut self, content: Vec<u8>) -> Result<AssetPath> {
        self.add_asset("highlight", "css", content)
    }

    fn add_asset(&mut self, name: &str, extension: &str, content: Vec<u8>) -> Result<AssetPath> {
        let hash = content_hash(&content);
        let filename = format!("{name}.{hash}.{extension}");
        let path = RoutePath::new(PathBuf::from("_assets").join(filename))?;
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

    /// Replace the prior output tree with this complete build.
    ///
    /// The complete file set is collected before publication. Clearing the
    /// output directory removes every stale page and content-addressed asset,
    /// so publishing the same build repeatedly produces the same tree.
    pub fn publish(self) -> Result<PublishedOutput> {
        remove_if_exists(&self.output_dir)?;
        std::fs::create_dir_all(&self.output_dir)
            .with_context(|| format!("failed to create {}", self.output_dir.display()))?;
        for (relative, file) in &self.files {
            let path = realize_output_path(&self.output_dir, relative)?;
            write_file(&path, file.content())?;
        }

        let pages = self
            .files
            .into_iter()
            .filter_map(|(path, file)| file.is_page().then_some(path))
            .map(|path| realize_output_path(&self.output_dir, &path))
            .collect::<Result<Vec<_>>>()?;
        Ok(PublishedOutput { pages })
    }

    fn insert(&mut self, path: RoutePath, file: OutputFile) -> Result<()> {
        if let Some(existing) = self.files.get(&path) {
            ensure!(
                existing == &file,
                "two generated files selected the same output path {}",
                path
            );
            return Ok(());
        }
        self.files.insert(path, file);
        Ok(())
    }
}

/// Per-page access to output publication policy.
pub struct PagePublication<'a> {
    publication: &'a mut OutputPublication,
    template: VirtualPath,
    output: &'a RoutePath,
}

impl PagePublication<'_> {
    /// Resolve a source reference relative to the template within the project virtual root.
    pub fn resolve_source(&self, reference: &Path) -> Result<VirtualPath> {
        ensure!(
            !reference.is_absolute(),
            "source reference must be relative"
        );

        let reference = reference
            .to_str()
            .context("source reference is not valid UTF-8")?;
        let template_dir = self
            .template
            .parent()
            .context("page template has no parent")?;
        template_dir.join(reference).with_context(|| {
            format!(
                "source reference {reference} escapes project root {}",
                self.publication.project_root.display()
            )
        })
    }

    pub fn project_root(&self) -> &Path {
        &self.publication.project_root
    }

    /// Register a bundled stylesheet under the entry file's name.
    pub fn add_bundled_stylesheet(
        &mut self,
        entry: &VirtualPath,
        content: Vec<u8>,
    ) -> Result<EcoString> {
        let name = entry
            .file_stem()
            .context("stylesheet entry has no file name")?;
        let asset = self.publication.add_asset(name, "css", content)?;
        self.reference(&asset)
    }

    /// Register an extracted image and return its browser-facing URL from this page.
    pub fn add_image(&mut self, format: ImageFormat, content: Vec<u8>) -> Result<EcoString> {
        let asset = self
            .publication
            .add_asset("img", format.extension(), content)?;
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

    fn fixture() -> (tempfile::TempDir, Project) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src/blog")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = Project::open(root.to_owned()).unwrap();
        (temp, project)
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
    fn rejects_paths_outside_output() {
        assert!(RoutePath::new("../index.html").is_err());
        assert!(RoutePath::new("/index.html").is_err());
        assert!(RoutePath::new("").is_err());
    }

    #[test]
    fn nested_page_gets_relative_asset_url() {
        let (_temp, project) = fixture();
        let mut publication = OutputPublication::new(&project);
        let asset = publication
            .add_highlight_stylesheet(b"body{}".to_vec())
            .unwrap();
        let output = RoutePath::new("blog/post.html").unwrap();
        let template = VirtualPath::new("/src/blog/[slug].typ").unwrap();
        let page = publication.page(&template, &output);

        assert!(
            page.reference(&asset)
                .unwrap()
                .starts_with("../_assets/highlight.")
        );
    }

    #[test]
    fn source_resolution_uses_template_directory_and_project_confinement() {
        let (_temp, project) = fixture();
        std::fs::write(project.root().join("style.css"), "body{}").unwrap();
        let template = VirtualPath::new("/src/blog/[slug].typ").unwrap();
        let mut publication = OutputPublication::new(&project);
        let output = RoutePath::new("blog/post.html").unwrap();
        let page = publication.page(&template, &output);

        assert_eq!(
            page.resolve_source(Path::new("../../style.css")).unwrap(),
            VirtualPath::new("/style.css").unwrap()
        );
        assert!(
            page.resolve_source(Path::new("../../../outside.css"))
                .is_err()
        );
    }

    #[test]
    fn publication_is_idempotent_and_removes_stale_output() {
        let (temp, project) = fixture();
        std::fs::create_dir_all(project.output_dir()).unwrap();
        std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

        let mut publication = OutputPublication::new(&project);
        let first = publication
            .add_highlight_stylesheet(b"body{}".to_vec())
            .unwrap();
        let second = publication
            .add_highlight_stylesheet(b"body{}".to_vec())
            .unwrap();
        assert_eq!(first, second);

        let template = VirtualPath::new("/src/index.typ").unwrap();
        let output = RoutePath::new("index.html").unwrap();
        publication
            .page(&template, &output)
            .add_html("new".into())
            .unwrap();
        let published = publication.publish().unwrap();

        assert_eq!(
            published.pages,
            vec![project.output_dir().join("index.html")]
        );

        let expected = snapshot_tree(&project.output_dir());
        std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

        let mut repeated = OutputPublication::new(&project);
        assert_eq!(
            repeated
                .add_highlight_stylesheet(b"body{}".to_vec())
                .unwrap(),
            first
        );
        repeated
            .page(&template, &output)
            .add_html("new".into())
            .unwrap();
        repeated.publish().unwrap();

        assert_eq!(snapshot_tree(&project.output_dir()), expected);
        assert!(!temp.path().join(".dist.aster-lock").exists());
        assert_eq!(expected.len(), 2);
    }

    #[test]
    fn empty_publication_replaces_output_with_empty_directory() {
        let (_temp, project) = fixture();
        std::fs::create_dir_all(project.output_dir()).unwrap();
        std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

        OutputPublication::new(&project).publish().unwrap();

        assert!(project.output_dir().is_dir());
        assert_eq!(std::fs::read_dir(project.output_dir()).unwrap().count(), 0);
    }
}
