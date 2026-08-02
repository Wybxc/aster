use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst::syntax::VirtualPath;
use walkdir::WalkDir;

/// A discovered Aster project rooted at an absolute lexical path.
#[derive(Clone)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    /// Find the nearest project at or above `dir`.
    pub fn find(dir: &Path) -> Option<Self> {
        let dir = std::path::absolute(dir).ok()?;
        let mut current = Some(dir.as_path());
        while let Some(path) = current {
            if path.join("aster.toml").is_file() {
                return Some(Self {
                    root: path.to_owned(),
                });
            }
            current = path.parent();
        }
        None
    }

    /// Open a directory containing an `aster.toml` project manifest.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root =
            std::path::absolute(root.into()).context("failed to make project root absolute")?;
        if !root.join("aster.toml").is_file() {
            bail!("no aster.toml found in {}", root.display());
        }
        Ok(Self { root })
    }

    /// Return the absolute project root without resolving symbolic links.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the page-template directory.
    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    /// Return the content collection directory.
    pub fn content_dir(&self) -> PathBuf {
        self.root.join("content")
    }

    /// Return the published output directory.
    pub fn output_dir(&self) -> PathBuf {
        self.root.join("dist")
    }

    /// Return the project manifest path.
    pub fn config_file(&self) -> PathBuf {
        self.root.join("aster.toml")
    }

    /// Return every structural and tracked build input that watch mode should
    /// observe, excluding the generated output tree.
    pub fn watch_paths<I>(&self, dependencies: I) -> Vec<PathBuf>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let output = self.output_dir();
        let mut paths = self.structural_watch_paths();
        paths.extend(
            dependencies
                .into_iter()
                .filter(|path| VirtualPath::virtualize(&output, path).is_err()),
        );
        paths.sort();
        paths.dedup();
        paths
    }

    fn structural_watch_paths(&self) -> Vec<PathBuf> {
        let directories = [self.src_dir(), self.content_dir()];
        let mut paths = vec![self.config_file()];
        paths.extend(directories.iter().cloned());
        for directory in directories {
            if !directory.is_dir() {
                continue;
            }
            // Builds report traversal errors; watching every reachable directory
            // lets a later filesystem change recover without exiting watch mode.
            paths.extend(
                WalkDir::new(directory)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().is_dir())
                    .map(|entry| entry.into_path()),
            );
        }
        paths
    }
}
