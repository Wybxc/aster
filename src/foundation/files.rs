//! Tracked filesystem access for a build session.
//!
//! Mirrors the `typst-kit` `files` module: file content accesses are recorded
//! by the upstream `FileStore` slot state machine, and path-level accesses
//! that the `FileStore` cannot express are recorded by a [`PathStore`]. Both
//! are reset between builds and combined into the watch dependency list.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use comemo::Tracked;
use typst::diag::FileError;
use typst::ecow::EcoString;
use typst::foundations::Bytes;
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::packages::SystemPackages;
use walkdir::WalkDir;

use crate::foundation::project::ProjectRoot;

/// The tracked filesystem surface of a Typst build session.
///
/// File content accesses (including missing files) are recorded by the
/// upstream `FileStore` slot state machine. Path-level accesses that the
/// `FileStore` cannot express (canonicalization of arbitrary paths) are
/// recorded by a small [`PathStore`]. Both are reset between builds and
/// combined into the watch dependency list.
pub(crate) struct ProjectFiles {
    root: PathBuf,
    store: FileStore<SystemFiles>,
    paths: PathStore,
}

/// Records path-level accesses that `FileStore` cannot express.
///
/// Unlike the `FileStore`, whose slot state machine tracks accesses by file
/// id, this store tracks plain paths: canonicalization targets that may not
/// exist yet. Tracking is implicit: every operation through this store
/// records its path, so callers never invoke tracking explicitly. Its
/// contents feed the watch dependency list so that a later appearance of
/// such a path triggers a rebuild.
struct PathStore {
    paths: Mutex<BTreeSet<PathBuf>>,
}

impl PathStore {
    fn new() -> Self {
        Self {
            paths: Mutex::new(BTreeSet::new()),
        }
    }

    /// Canonicalize a path, recording it as a tracked dependency.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FileAccessError> {
        self.record(path);
        std::fs::canonicalize(path).map_err(|error| FileAccessError::Io {
            path: path.into(),
            kind: error.kind(),
            message: error.to_string().into(),
        })
    }

    fn record(&self, path: &Path) {
        self.paths
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(path.to_owned());
    }

    fn reset(&self) {
        self.paths
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    fn paths(&self) -> Vec<PathBuf> {
        self.paths
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect()
    }
}

/// A cheaply cloneable filesystem access error at the memoization seam.
///
/// This mirrors how Typst models file errors: a small set of structural
/// variants carrying the essential path and message, with everything else
/// falling back to [`FileAccessError::Other`]. Paths are reference-counted
/// and messages use [`EcoString`] so cloning the error is O(1); this matters
/// because comemo records the result hash of every tracked call and clones
/// cached outputs.
#[derive(Debug, Clone, Hash, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FileAccessError {
    #[error("failed to access {path}: {kind:?} ({message})")]
    Io {
        path: Arc<Path>,
        kind: std::io::ErrorKind,
        message: EcoString,
    },
    #[error("failed to inspect {path}: {kind:?} ({message})")]
    Inspect {
        path: Arc<Path>,
        kind: std::io::ErrorKind,
        message: EcoString,
    },
    #[error("{path} is outside {root}: {kind:?} ({message})")]
    Outside {
        path: Arc<Path>,
        root: Arc<Path>,
        kind: std::io::ErrorKind,
        message: EcoString,
    },
    #[error("{0}")]
    Other(EcoString),
}

impl FileAccessError {
    /// Project the stable classification and message out of an `std::io::Error`.
    pub(crate) fn io(path: Arc<Path>, error: std::io::Error) -> Self {
        Self::Io {
            path,
            kind: error.kind(),
            message: error.to_string().into(),
        }
    }
}

impl ProjectFiles {
    pub(crate) fn new(project: &ProjectRoot) -> Self {
        let root =
            std::fs::canonicalize(project.root()).unwrap_or_else(|_| project.root().to_owned());
        let downloader = SystemDownloader::new("aster/0.1.0");
        let packages = SystemPackages::new(downloader);
        let fs_root = FsRoot::new(root.clone());
        let store = FileStore::new(SystemFiles::new(fs_root, packages));
        Self {
            root,
            store,
            paths: PathStore::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.store.reset();
        self.paths.reset();
    }

    pub(crate) fn dependencies(&mut self) -> Vec<PathBuf> {
        let mut paths = self.paths.paths();
        let (loader, dependencies) = self.store.dependencies();
        paths.extend(dependencies.filter_map(|id| loader.resolve(id).ok()));
        paths.sort();
        paths.dedup();
        paths
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn source(&self, id: FileId) -> Result<typst::syntax::Source, FileError> {
        self.store.source(id)
    }

    pub(crate) fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        self.store.file(id)
    }

    pub(crate) fn resolve(&self, id: FileId) -> Result<PathBuf, FileError> {
        self.store.loader().resolve(id)
    }
}

#[comemo::track]
impl ProjectFiles {
    pub(crate) fn list(
        &self,
        directory: &Path,
        required: bool,
    ) -> Result<Vec<PathBuf>, FileAccessError> {
        match std::fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(FileAccessError::Other(
                    format!("{} must not be a symlink", directory.display()).into(),
                ));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(FileAccessError::Other(
                    format!("{} is not a directory", directory.display()).into(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => {
                return Ok(Vec::new());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(FileAccessError::Other(
                    format!("{} directory not found", directory.display()).into(),
                ));
            }
            Err(error) => {
                return Err(FileAccessError::Inspect {
                    path: directory.into(),
                    kind: error.kind(),
                    message: error.to_string().into(),
                });
            }
        }

        let mut files = Vec::new();
        for entry in WalkDir::new(directory) {
            let entry = entry.map_err(|error| {
                let kind = error
                    .io_error()
                    .map(std::io::Error::kind)
                    .unwrap_or(std::io::ErrorKind::Other);
                FileAccessError::Inspect {
                    path: directory.into(),
                    kind,
                    message: error.to_string().into(),
                }
            })?;
            if entry.file_type().is_symlink() {
                return Err(FileAccessError::Other(
                    format!(
                        "symlink {} is not allowed in {}",
                        entry.path().display(),
                        directory.display()
                    )
                    .into(),
                ));
            }
            if entry.file_type().is_dir() {
                // Directory membership is covered by the structural watch
                // paths; only file entries enter the listing.
            } else if entry.file_type().is_file() {
                files.push(entry.into_path());
            }
        }
        files.sort();
        Ok(files)
    }

    pub(crate) fn canonicalize(&self, path: &Path) -> Result<PathBuf, FileAccessError> {
        self.paths.canonicalize(path)
    }

    pub(crate) fn read(&self, path: &Path) -> Result<Bytes, FileAccessError> {
        let virtual_path = VirtualPath::virtualize(&self.root, path).map_err(|error| {
            FileAccessError::Outside {
                path: path.into(),
                root: self.root.clone().into(),
                kind: std::io::ErrorKind::InvalidInput,
                message: error.to_string().into(),
            }
        })?;
        let id = RootedPath::new(VirtualRoot::Project, virtual_path).intern();
        self.store.file(id).map_err(|error| FileAccessError::Io {
            path: path.into(),
            kind: file_error_kind(&error),
            message: error.to_string().into(),
        })
    }
}

/// Project the stable classification out of a `typst::diag::FileError`.
fn file_error_kind(error: &FileError) -> std::io::ErrorKind {
    match error {
        FileError::NotFound(_) => std::io::ErrorKind::NotFound,
        FileError::AccessDenied => std::io::ErrorKind::PermissionDenied,
        FileError::IsDirectory => std::io::ErrorKind::IsADirectory,
        FileError::InvalidUtf8 => std::io::ErrorKind::InvalidData,
        FileError::NotSource | FileError::Realize(_) | FileError::Package(_) => {
            std::io::ErrorKind::InvalidInput
        }
        FileError::Other(_) => std::io::ErrorKind::Other,
    }
}

#[comemo::memoize]
pub(crate) fn list_typst_files(
    project_files: Tracked<ProjectFiles>,
    directory: &Path,
    required: bool,
) -> Result<Vec<PathBuf>, FileAccessError> {
    Ok(project_files
        .list(directory, required)?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "typ"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_store_records_missing_paths_and_resets() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing-theme.tmTheme");
        let paths = PathStore::new();

        assert!(paths.canonicalize(&missing).is_err());
        assert_eq!(paths.paths(), vec![missing]);

        paths.reset();
        assert!(paths.paths().is_empty());
    }
}
