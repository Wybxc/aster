//! Tracked filesystem access for a build session.
//!
//! Mirrors the `typst-kit` `files` module: file content accesses, including
//! missing files, are recorded by the upstream `FileStore` slot state machine
//! and surfaced as watch dependencies.

use std::path::{Path, PathBuf};
use std::sync::Arc;

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

use crate::foundation::project::Project;

/// The tracked filesystem surface of a Typst build session.
///
/// File content accesses (including missing files) are recorded by the
/// upstream `FileStore` slot state machine and become watch dependencies.
pub(crate) struct ProjectFiles {
    root: PathBuf,
    store: FileStore<SystemFiles>,
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
    pub(crate) fn new(project: &Project) -> Self {
        let root = project.root().to_owned();
        let downloader = SystemDownloader::new("aster/0.1.0");
        let packages = SystemPackages::new(downloader);
        let fs_root = FsRoot::new(root.clone());
        let store = FileStore::new(SystemFiles::new(fs_root, packages));
        Self { root, store }
    }

    pub(crate) fn reset(&mut self) {
        self.store.reset();
    }

    pub(crate) fn dependencies(&mut self) -> Vec<PathBuf> {
        let (loader, dependencies) = self.store.dependencies();
        let mut paths = dependencies
            .filter_map(|id| loader.resolve(id).ok())
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }

    pub(crate) fn source(&self, id: FileId) -> Result<typst::syntax::Source, FileError> {
        self.store.source(id)
    }

    pub(crate) fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        self.store.file(id)
    }
}

#[comemo::track]
impl ProjectFiles {
    pub(crate) fn list(
        &self,
        directory: &VirtualPath,
        required: bool,
    ) -> Result<Vec<VirtualPath>, FileAccessError> {
        let directory = directory.realize(&self.root).map_err(|error| {
            FileAccessError::Other(
                format!(
                    "invalid project directory {}: {error}",
                    directory.get_with_slash()
                )
                .into(),
            )
        })?;
        match std::fs::metadata(&directory) {
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
        for entry in WalkDir::new(&directory).follow_links(true) {
            let entry = entry.map_err(|error| {
                let kind = error
                    .io_error()
                    .map(std::io::Error::kind)
                    .unwrap_or(std::io::ErrorKind::Other);
                FileAccessError::Inspect {
                    path: directory.as_path().into(),
                    kind,
                    message: error.to_string().into(),
                }
            })?;
            if entry.file_type().is_dir() {
                // Directory membership is covered by the structural watch
                // paths; only file entries enter the listing.
            } else if entry.file_type().is_file() {
                let path = entry.into_path();
                let virtual_path = VirtualPath::virtualize(&self.root, &path).map_err(|error| {
                    FileAccessError::Outside {
                        path: path.into(),
                        root: self.root.clone().into(),
                        kind: std::io::ErrorKind::InvalidInput,
                        message: error.to_string().into(),
                    }
                })?;
                files.push(virtual_path);
            }
        }
        files.sort_by(|left, right| left.get_with_slash().cmp(right.get_with_slash()));
        Ok(files)
    }

    pub(crate) fn read(&self, path: &VirtualPath) -> Result<Bytes, FileAccessError> {
        let id = RootedPath::new(VirtualRoot::Project, path.clone()).intern();
        self.store.file(id).map_err(|error| FileAccessError::Io {
            path: path
                .realize(&self.root)
                .unwrap_or_else(|_| PathBuf::from(path.get_with_slash()))
                .into(),
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
    directory: &VirtualPath,
    required: bool,
) -> Result<Vec<VirtualPath>, FileAccessError> {
    Ok(project_files
        .list(directory, required)?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "typ"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn lists_and_reads_through_a_symlinked_source_directory() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path().join("project");
        let external = temp.path().join("external");
        std::fs::create_dir_all(&project_root).unwrap();
        std::fs::create_dir_all(&external).unwrap();
        std::fs::write(project_root.join("aster.toml"), "").unwrap();
        std::fs::write(external.join("index.typ"), "external").unwrap();
        symlink(&external, project_root.join("src")).unwrap();

        let project = Project::open(&project_root).unwrap();
        let files = ProjectFiles::new(&project);
        let source = VirtualPath::new("/src/index.typ").unwrap();

        assert_eq!(
            files
                .list(&VirtualPath::new("/src").unwrap(), true)
                .unwrap(),
            vec![source.clone()]
        );
        assert_eq!(files.read(&source).unwrap().as_slice(), b"external");
    }
}
