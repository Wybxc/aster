use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use walkdir::WalkDir;

/// A discovered Aster project root, managing all project paths.
///
/// Before building, the root is located by searching upward for
/// `aster.toml` (via [`find`](Self::find)) or by explicit path
/// (via [`new`](Self::new)).
#[derive(Clone)]
pub struct ProjectRoot {
    root: PathBuf,
}

impl ProjectRoot {
    /// Locate the nearest ancestor of `dir` that contains `aster.toml`.
    pub fn find(dir: &Path) -> Option<Self> {
        let mut current = Some(dir);
        while let Some(path) = current {
            if path.join("aster.toml").exists() {
                return Some(Self {
                    root: path.to_owned(),
                });
            }
            current = path.parent();
        }
        None
    }

    /// Create from an explicit path that must contain `aster.toml`.
    pub fn new(root: PathBuf) -> Result<Self> {
        if !root.join("aster.toml").exists() {
            bail!("no aster.toml found in {:?}", root);
        }
        Ok(Self { root })
    }

    /// The project root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The page templates directory (`<root>/src`).
    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    /// The content collections directory (`<root>/content`).
    pub fn content_dir(&self) -> PathBuf {
        self.root.join("content")
    }

    /// The build output directory (`<root>/dist`).
    pub fn output_dir(&self) -> PathBuf {
        self.root.join("dist")
    }

    /// Path to the project config file.
    pub fn config_file(&self) -> PathBuf {
        self.root.join("aster.toml")
    }

    /// Compute the output HTML path for a page template.
    ///
    /// Returns `Some(<root>/dist/<relative>.html)` when `page` is inside
    /// `src_dir`, or `None` if it isn't.
    pub fn page_output_path(&self, page: &Path) -> Option<PathBuf> {
        let src = self.src_dir();
        let relative = page.strip_prefix(&src).ok()?;
        Some(self.output_dir().join(relative).with_extension("html"))
    }

    /// Iterate files in the `src/` directory.
    pub fn walk_src(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(self.src_dir())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
    }

    /// Iterate files in the `content/` directory.
    pub fn walk_content(&self) -> impl Iterator<Item = PathBuf> {
        WalkDir::new(self.content_dir())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
    }
}
