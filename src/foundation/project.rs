use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use walkdir::WalkDir;

/// A discovered Aster project with one normalized layout policy.
#[derive(Clone)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    /// Find the nearest project at or above `dir`.
    pub fn find(dir: &Path) -> Option<Self> {
        let mut current = Some(dir);
        while let Some(path) = current {
            if path.join("aster.toml").is_file() {
                return Some(Self {
                    root: normalize(path),
                });
            }
            current = path.parent();
        }
        None
    }

    /// Open a directory containing an `aster.toml` project manifest.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let root = normalize(&root);
        if !root.join("aster.toml").is_file() {
            bail!("no aster.toml found in {}", root.display());
        }
        Ok(Self { root })
    }

    /// Return the normalized project root.
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
    pub fn watch_paths(&self, dependencies: &[PathBuf]) -> Vec<PathBuf> {
        let output = self.output_dir();
        let mut paths = self.structural_watch_paths();
        paths.extend(
            dependencies
                .iter()
                .filter(|path| !path.starts_with(&output))
                .cloned(),
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
        paths.sort();
        paths.dedup();
        paths
    }
}

fn normalize(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    std::fs::canonicalize(&absolute).unwrap_or(absolute)
}
