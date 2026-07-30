use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

/// A discovered Aster project with one normalized layout policy.
#[derive(Clone)]
pub struct ProjectRoot {
    root: PathBuf,
}

impl ProjectRoot {
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

    pub fn new(root: PathBuf) -> Result<Self> {
        let root = normalize(&root);
        if !root.join("aster.toml").is_file() {
            bail!("no aster.toml found in {}", root.display());
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    pub fn content_dir(&self) -> PathBuf {
        self.root.join("content")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join("dist")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("aster.toml")
    }

    pub fn source_files(&self) -> Result<Vec<PathBuf>> {
        walk_files(&self.src_dir(), true)
    }

    pub fn content_files(&self) -> Result<Vec<PathBuf>> {
        walk_files(&self.content_dir(), false)
    }
}

fn normalize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn walk_files(directory: &Path, required: bool) -> Result<Vec<PathBuf>> {
    match std::fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("{} must not be a symlink", directory.display())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => {
            return Ok(Vec::new());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            bail!("{} directory not found", directory.display())
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect {}", directory.display()));
        }
    }
    if !directory.is_dir() {
        bail!("{} is not a directory", directory.display());
    }

    let entries = WalkDir::new(directory)
        .into_iter()
        .map(|entry| entry.with_context(|| format!("failed to traverse {}", directory.display())))
        .collect::<Result<Vec<_>>>()?;
    for entry in &entries {
        if entry.file_type().is_symlink() {
            bail!(
                "symlink {} is not allowed in {}",
                entry.path().display(),
                directory.display()
            );
        }
    }
    let mut files = entries
        .into_iter()
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}
