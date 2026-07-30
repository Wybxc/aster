use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use fs2::FileExt;

use crate::project::ProjectRoot;
use crate::utils::content_hash;

/// A validated path inside an Aster build output directory.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OutputPath(PathBuf);

impl OutputPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        ensure!(!path.as_os_str().is_empty(), "output path cannot be empty");

        for component in path.components() {
            match component {
                Component::Normal(_) => {}
                Component::CurDir => bail!("output path cannot contain `.`"),
                Component::ParentDir => bail!("output path cannot contain `..`"),
                Component::RootDir | Component::Prefix(_) => {
                    bail!("output path must be relative")
                }
            }
        }

        Ok(Self(path))
    }

    pub fn from_template(relative_template: &Path) -> Result<Self> {
        let mut output = relative_template.to_path_buf();
        output.set_extension("html");
        Self::new(output)
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// The stable output location of a generated asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetPath(OutputPath);

pub struct PublishedOutput {
    pub pages: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

/// Collects an entire successful build before replacing `dist/`.
///
/// Asset identity, browser references, output confinement, stale-file removal,
/// and filesystem publication all live behind this module's interface.
pub struct OutputPublication {
    src_dir: PathBuf,
    output_dir: PathBuf,
    files: BTreeMap<OutputPath, Vec<u8>>,
    pages: Vec<OutputPath>,
}

impl OutputPublication {
    pub fn new(project: &ProjectRoot) -> Self {
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
            OutputPath::new(PathBuf::from("_assets").join(format!("{kind}.{hash}.{extension}")))?;
        self.insert(path.clone(), content)?;
        Ok(AssetPath(path))
    }

    pub fn page<'a>(
        &'a mut self,
        template: &'a Path,
        output: &'a OutputPath,
    ) -> Result<PagePublication<'a>> {
        ensure!(
            template.starts_with(&self.src_dir),
            "page template {} is outside {}",
            template.display(),
            self.src_dir.display()
        );

        Ok(PagePublication {
            publication: self,
            template,
            output,
        })
    }

    /// Replace the prior output tree with this complete build.
    ///
    /// Building in a sibling directory means a failed build leaves the prior
    /// `dist/` untouched. Replacing the directory removes every stale page and
    /// content-addressed asset from earlier builds.
    pub fn publish(self) -> Result<PublishedOutput> {
        let parent = self
            .output_dir
            .parent()
            .context("output directory has no parent")?;
        let name = self
            .output_dir
            .file_name()
            .context("output directory has no file name")?
            .to_string_lossy();
        let lock_path = parent.join(format!(".{name}.aster-lock"));
        let mut lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open build lock {}", lock_path.display()))?;
        lock.try_lock_exclusive().with_context(|| {
            format!(
                "another build is publishing {} (lock {})",
                self.output_dir.display(),
                lock_path.display()
            )
        })?;
        let build_id = next_build_id(&mut lock)?;
        cleanup_staging_dirs(parent, &format!(".{name}.aster-staging-"))?;
        let staging = parent.join(format!(".{name}.aster-staging-{build_id}"));
        let backup = parent.join(format!(".{name}.aster-backup"));

        if !self.output_dir.exists() && backup.exists() {
            std::fs::rename(&backup, &self.output_dir).with_context(|| {
                format!("failed to recover prior output from {}", backup.display())
            })?;
        } else if self.output_dir.exists() && backup.exists() {
            remove_if_exists(&backup)?;
        }
        std::fs::create_dir(&staging)
            .with_context(|| format!("failed to create {}", staging.display()))?;

        if let Err(error) = write_staging_files(&staging, &self.files) {
            let _ = remove_if_exists(&staging);
            return Err(error);
        }

        let had_previous = self.output_dir.exists();
        if had_previous {
            std::fs::rename(&self.output_dir, &backup).with_context(|| {
                format!(
                    "failed to move existing output {} aside",
                    self.output_dir.display()
                )
            })?;
        }

        if let Err(publish_error) = std::fs::rename(&staging, &self.output_dir) {
            let message = format!(
                "failed to publish output directory {}: {publish_error}",
                self.output_dir.display()
            );
            if had_previous && let Err(restore_error) = std::fs::rename(&backup, &self.output_dir) {
                bail!(
                    "{message}; also failed to restore prior output from {}: {restore_error}",
                    backup.display()
                );
            }
            bail!("{message}");
        }

        let mut warnings = Vec::new();
        if had_previous && let Err(error) = remove_if_exists(&backup) {
            warnings.push(format!(
                "published output but failed to remove backup {}: {error:#}",
                backup.display()
            ));
        }

        let pages = self
            .pages
            .into_iter()
            .map(|path| self.output_dir.join(path.as_path()))
            .collect();
        Ok(PublishedOutput { pages, warnings })
    }

    fn insert(&mut self, path: OutputPath, content: Vec<u8>) -> Result<()> {
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
    template: &'a Path,
    output: &'a OutputPath,
}

impl PagePublication<'_> {
    /// Resolve a source reference relative to the actual template, confined to `src/`.
    pub fn resolve_source(&self, reference: &str) -> Result<PathBuf> {
        let reference = Path::new(reference);
        ensure!(
            !reference.is_absolute(),
            "source reference must be relative"
        );

        let template_dir = self
            .template
            .parent()
            .context("page template has no parent")?;
        let source = std::fs::canonicalize(template_dir.join(reference)).with_context(|| {
            format!(
                "failed to resolve {} from {}",
                reference.display(),
                self.template.display()
            )
        })?;
        let src_dir = std::fs::canonicalize(&self.publication.src_dir).with_context(|| {
            format!(
                "failed to resolve source directory {}",
                self.publication.src_dir.display()
            )
        })?;
        ensure!(
            source.starts_with(&src_dir),
            "source reference {} escapes {}",
            reference.display(),
            src_dir.display()
        );
        Ok(source)
    }

    pub fn source_root(&self) -> Result<PathBuf> {
        std::fs::canonicalize(&self.publication.src_dir).with_context(|| {
            format!(
                "failed to resolve source directory {}",
                self.publication.src_dir.display()
            )
        })
    }

    /// Register an asset and return its browser-facing URL from this page.
    pub fn add_asset(&mut self, kind: &str, extension: &str, content: Vec<u8>) -> Result<String> {
        let asset = self.publication.add_asset(kind, extension, content)?;
        self.reference(&asset)
    }

    /// Return a browser-facing URL from this page to an existing generated asset.
    pub fn reference(&self, asset: &AssetPath) -> Result<String> {
        let page_dir = self
            .output
            .as_path()
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let relative = pathdiff::diff_paths(asset.0.as_path(), page_dir)
            .context("failed to compute generated asset reference")?;
        Ok(path_to_url(&relative))
    }

    /// Add the final serialized page to this publication.
    pub fn add_html(self, html: String) -> Result<()> {
        let output = self.output.clone();
        self.publication.insert(output.clone(), html.into_bytes())?;
        self.publication.pages.push(output);
        Ok(())
    }
}

fn write_staging_files(staging: &Path, files: &BTreeMap<OutputPath, Vec<u8>>) -> Result<()> {
    for (relative, content) in files {
        write_file(&staging.join(relative.as_path()), content)?;
    }
    Ok(())
}

fn cleanup_staging_dirs(parent: &Path, prefix: &str) -> Result<()> {
    for entry in
        std::fs::read_dir(parent).with_context(|| format!("failed to list {}", parent.display()))?
    {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(prefix) {
            remove_if_exists(&entry.path())?;
        }
    }
    Ok(())
}

fn next_build_id(lock: &mut File) -> Result<u64> {
    use std::io::{Read, Seek, SeekFrom};

    lock.seek(SeekFrom::Start(0))?;
    let mut current = String::new();
    lock.read_to_string(&mut current)?;
    let next = current.trim().parse::<u64>().unwrap_or(0).saturating_add(1);
    lock.set_len(0)?;
    lock.seek(SeekFrom::Start(0))?;
    write!(lock, "{next}")?;
    lock.sync_data()?;
    Ok(next)
}

fn valid_name_part(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn path_to_url(path: &Path) -> String {
    path.components()
        .map(|component| match component {
            Component::ParentDir => "..".to_owned(),
            Component::Normal(part) => part.to_string_lossy().into_owned(),
            Component::CurDir => ".".to_owned(),
            Component::RootDir | Component::Prefix(_) => unreachable!("relative path expected"),
        })
        .collect::<Vec<_>>()
        .join("/")
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn fixture() -> (PathBuf, ProjectRoot) {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("aster-output-test-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src/blog")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = ProjectRoot::new(root.clone()).unwrap();
        (root, project)
    }

    #[test]
    fn rejects_paths_outside_output() {
        assert!(OutputPath::new("../index.html").is_err());
        assert!(OutputPath::new("/index.html").is_err());
        assert!(OutputPath::new("").is_err());
    }

    #[test]
    fn nested_page_gets_relative_asset_url() {
        let (root, project) = fixture();
        let mut publication = OutputPublication::new(&project);
        let asset = publication
            .add_asset("css", "css", b"body{}".to_vec())
            .unwrap();
        let output = OutputPath::new("blog/post.html").unwrap();
        let template = project.src_dir().join("blog/[slug].typ");
        std::fs::write(&template, "").unwrap();
        let page = publication.page(&template, &output).unwrap();

        assert!(
            page.reference(&asset)
                .unwrap()
                .starts_with("../_assets/css.")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn source_resolution_uses_template_directory_and_is_confined() {
        let (root, project) = fixture();
        std::fs::write(project.src_dir().join("style.css"), "body{}").unwrap();
        let template = project.src_dir().join("blog/[slug].typ");
        std::fs::write(&template, "").unwrap();
        let mut publication = OutputPublication::new(&project);
        let output = OutputPath::new("blog/post.html").unwrap();
        let page = publication.page(&template, &output).unwrap();

        assert_eq!(
            page.resolve_source("../style.css").unwrap(),
            std::fs::canonicalize(project.src_dir().join("style.css")).unwrap()
        );
        assert!(page.resolve_source("../../aster.toml").is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn publication_replaces_stale_output_and_deduplicates_assets() {
        let (root, project) = fixture();
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
        let output = OutputPath::new("index.html").unwrap();
        publication
            .page(&template, &output)
            .unwrap()
            .add_html("new".into())
            .unwrap();
        publication.publish().unwrap();

        assert!(!project.output_dir().join("stale.html").exists());
        assert_eq!(
            std::fs::read_to_string(project.output_dir().join("index.html")).unwrap(),
            "new"
        );
        assert_eq!(
            std::fs::read_dir(project.output_dir().join("_assets"))
                .unwrap()
                .count(),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
