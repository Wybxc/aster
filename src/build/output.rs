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

pub(crate) struct PublishedOutput {
    pub pages: Vec<PathBuf>,
}

/// Collects an entire successful build before replacing `dist/`.
///
/// Asset identity, browser references, output confinement, stale-file removal,
/// and filesystem publication all live behind this module's interface.
pub(crate) struct OutputPublication {
    src_dir: PathBuf,
    output_dir: PathBuf,
    files: BTreeMap<RoutePath, Vec<u8>>,
    pages: Vec<RoutePath>,
}

impl OutputPublication {
    pub fn new(project: &Project) -> Self {
        Self {
            src_dir: project.src_dir(),
            output_dir: project.output_dir(),
            files: BTreeMap::new(),
            pages: Vec::new(),
        }
    }

    /// Register generated content under a deterministic, order-independent name.
    pub fn add_asset(
        &mut self,
        kind: &str,
        extension: &str,
        content: Vec<u8>,
    ) -> Result<AssetPath> {
        ensure!(
            valid_name_part(kind) && valid_name_part(extension),
            "asset kind and extension must be ASCII letters, digits, or hyphens"
        );

        let hash = content_hash(&content);
        let path =
            RoutePath::new(PathBuf::from("_assets").join(format!("{kind}.{hash}.{extension}")))?;
        self.insert(path.clone(), content)?;
        Ok(AssetPath(path))
    }

    pub fn page<'a>(
        &'a mut self,
        template: &Path,
        output: &'a RoutePath,
    ) -> Result<PagePublication<'a>> {
        let template = VirtualPath::virtualize(&self.src_dir, template).with_context(|| {
            format!(
                "page template {} is outside {}",
                template.display(),
                self.src_dir.display()
            )
        })?;

        Ok(PagePublication {
            publication: self,
            template,
            output,
        })
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
        write_output_files(&self.output_dir, &self.files)?;

        let pages = self
            .pages
            .into_iter()
            .map(|path| self.output_dir.join(path.as_path()))
            .collect();
        Ok(PublishedOutput { pages })
    }

    fn insert(&mut self, path: RoutePath, content: Vec<u8>) -> Result<()> {
        if let Some(existing) = self.files.get(&path) {
            ensure!(
                existing == &content,
                "two generated files selected the same output path {}",
                path.as_path().display()
            );
            return Ok(());
        }
        self.files.insert(path, content);
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
    /// Resolve a source reference relative to the template with lexical `src/` confinement.
    pub fn resolve_source(&self, reference: &Path) -> Result<PathBuf> {
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
        let virtual_path = template_dir.join(reference).with_context(|| {
            format!(
                "source reference {reference} escapes {}",
                self.publication.src_dir.display()
            )
        })?;
        virtual_path
            .realize(&self.publication.src_dir)
            .context("failed to realize source reference")
    }

    pub fn source_root(&self) -> &Path {
        &self.publication.src_dir
    }

    /// Register an asset and return its browser-facing URL from this page.
    pub fn add_asset(
        &mut self,
        kind: &str,
        extension: &str,
        content: Vec<u8>,
    ) -> Result<EcoString> {
        let asset = self.publication.add_asset(kind, extension, content)?;
        self.reference(&asset)
    }

    /// Return a browser-facing URL from this page to an existing generated asset.
    pub fn reference(&self, asset: &AssetPath) -> Result<EcoString> {
        let asset = virtualize_route(&asset.0)?;
        let output = virtualize_route(self.output)?;
        let page_dir = output.parent().context("output page has no parent")?;
        Ok(asset.relative_from(&page_dir))
    }

    /// Add the final serialized page to this publication.
    pub fn add_html(self, html: String) -> Result<()> {
        let output = self.output.clone();
        self.publication.insert(output.clone(), html.into_bytes())?;
        self.publication.pages.push(output);
        Ok(())
    }
}

fn write_output_files(output_dir: &Path, files: &BTreeMap<RoutePath, Vec<u8>>) -> Result<()> {
    for (relative, content) in files {
        write_file(&output_dir.join(relative.as_path()), content)?;
    }
    Ok(())
}

fn valid_name_part(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn virtualize_route(path: &RoutePath) -> Result<VirtualPath> {
    VirtualPath::virtualize(Path::new(""), path.as_path())
        .context("generated output path is not a valid virtual path")
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
            .add_asset("css", "css", b"body{}".to_vec())
            .unwrap();
        let output = RoutePath::new("blog/post.html").unwrap();
        let template = project.src_dir().join("blog/[slug].typ");
        std::fs::write(&template, "").unwrap();
        let page = publication.page(&template, &output).unwrap();

        assert!(
            page.reference(&asset)
                .unwrap()
                .starts_with("../_assets/css.")
        );
    }

    #[test]
    fn source_resolution_uses_template_directory_and_is_confined() {
        let (_temp, project) = fixture();
        std::fs::write(project.src_dir().join("style.css"), "body{}").unwrap();
        let template = project.src_dir().join("blog/[slug].typ");
        std::fs::write(&template, "").unwrap();
        let mut publication = OutputPublication::new(&project);
        let output = RoutePath::new("blog/post.html").unwrap();
        let page = publication.page(&template, &output).unwrap();

        assert_eq!(
            page.resolve_source(Path::new("../style.css")).unwrap(),
            project.src_dir().join("style.css")
        );
        assert!(page.resolve_source(Path::new("../../aster.toml")).is_err());
    }

    #[test]
    fn publication_is_idempotent_and_removes_stale_output() {
        let (temp, project) = fixture();
        std::fs::create_dir_all(project.output_dir()).unwrap();
        std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

        let mut publication = OutputPublication::new(&project);
        let first = publication
            .add_asset("css", "css", b"body{}".to_vec())
            .unwrap();
        let second = publication
            .add_asset("css", "css", b"body{}".to_vec())
            .unwrap();
        assert_eq!(first, second);

        let template = project.src_dir().join("index.typ");
        std::fs::write(&template, "").unwrap();
        let output = RoutePath::new("index.html").unwrap();
        publication
            .page(&template, &output)
            .unwrap()
            .add_html("new".into())
            .unwrap();
        publication.publish().unwrap();

        let expected = snapshot_tree(&project.output_dir());
        std::fs::write(project.output_dir().join("stale.html"), "old").unwrap();

        let mut repeated = OutputPublication::new(&project);
        assert_eq!(
            repeated
                .add_asset("css", "css", b"body{}".to_vec())
                .unwrap(),
            first
        );
        repeated
            .page(&template, &output)
            .unwrap()
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
