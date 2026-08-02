//! Tracked filesystem access for a build session.
//!
//! File content accesses are recorded by the upstream `FileStore`; configured
//! directory accesses are recorded here at the same boundary that validates
//! them. Both are surfaced as build dependencies.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use comemo::Tracked;
use typst::diag::FileError;
use typst::ecow::{EcoString, EcoVec, eco_format};
use typst::foundations::Bytes;
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::packages::SystemPackages;
use walkdir::WalkDir;

use crate::foundation::project::Project;

/// The tracked filesystem surface of a Typst build session.
///
/// File content accesses use the upstream `FileStore` slot state machine.
/// Directory accesses use a small access journal rather than a second cache.
pub(crate) struct ProjectFiles {
    root: PathBuf,
    store: FileStore<SystemFiles>,
    directories: Mutex<HashSet<VirtualPath>>,
}

/// A filesystem input observed through tracked project access.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FilesystemDependency {
    /// A file whose contents were accessed.
    File(PathBuf),
    /// A directory whose recursive membership was accessed.
    Tree(PathBuf),
}

impl FilesystemDependency {
    /// Return the dependency's filesystem path.
    pub fn path(&self) -> &Path {
        match self {
            Self::File(path) | Self::Tree(path) => path,
        }
    }
}

/// A configured project directory whose access has been recorded.
pub(crate) struct ProjectDirectory(PathBuf);

impl ProjectDirectory {
    pub(crate) fn path(&self) -> &Path {
        &self.0
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
            message: eco_format!("{error}"),
        }
    }
}

impl ProjectFiles {
    pub(crate) fn new(project: &Project) -> Self {
        let root = project.root().to_owned();
        let downloader = SystemDownloader::new(concat!("aster/", env!("CARGO_PKG_VERSION")));
        let packages = SystemPackages::new(downloader);
        let fs_root = FsRoot::new(root.clone());
        let store = FileStore::new(SystemFiles::new(fs_root, packages));
        Self {
            root,
            store,
            directories: Mutex::new(HashSet::new()),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.store.reset();
        self.directories.get_mut().unwrap().clear();
    }

    pub(crate) fn dependencies(&mut self) -> Vec<FilesystemDependency> {
        let (loader, dependencies) = self.store.dependencies();
        let mut observed = dependencies
            .filter_map(|id| loader.resolve(id).ok().map(FilesystemDependency::File))
            .collect::<Vec<_>>();
        observed.extend(
            self.directories
                .get_mut()
                .unwrap()
                .iter()
                .filter_map(|path| {
                    path.realize(&self.root)
                        .ok()
                        .map(FilesystemDependency::Tree)
                }),
        );
        observed
    }

    pub(crate) fn source(&self, id: FileId) -> Result<typst::syntax::Source, FileError> {
        self.store.source(id)
    }

    pub(crate) fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        self.store.file(id)
    }

    pub(crate) fn directory(
        &self,
        path: &VirtualPath,
    ) -> Result<ProjectDirectory, FileAccessError> {
        let directory = path.realize(&self.root).map_err(|error| {
            FileAccessError::Other(eco_format!(
                "invalid project directory {}: {error}",
                path.get_with_slash()
            ))
        })?;
        self.directories.lock().unwrap().insert(path.clone());
        let result = std::fs::metadata(&directory);

        match result {
            Ok(metadata) if metadata.is_dir() => Ok(ProjectDirectory(directory)),
            Ok(_) => Err(FileAccessError::Other(eco_format!(
                "{} is not a directory",
                directory.display()
            ))),
            Err(error) => Err(FileAccessError::Inspect {
                path: directory.into(),
                kind: error.kind(),
                message: eco_format!("{error}"),
            }),
        }
    }
}

#[comemo::track]
impl ProjectFiles {
    pub(crate) fn list(
        &self,
        directory: &VirtualPath,
        required: bool,
    ) -> Result<EcoVec<VirtualPath>, FileAccessError> {
        let directory = match self.directory(directory) {
            Ok(directory) => directory,
            Err(FileAccessError::Inspect {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) if !required => {
                return Ok(EcoVec::new());
            }
            Err(error) => return Err(error),
        };

        let mut files = EcoVec::new();
        for entry in WalkDir::new(directory.path()).follow_links(true) {
            let entry = entry.map_err(|error| {
                let kind = error
                    .io_error()
                    .map(std::io::Error::kind)
                    .unwrap_or(std::io::ErrorKind::Other);
                FileAccessError::Inspect {
                    path: directory.path().into(),
                    kind,
                    message: eco_format!("{error}"),
                }
            })?;
            if entry.file_type().is_dir() {
                // Directory membership is represented by structural tree
                // dependencies; only file entries enter the listing.
            } else if entry.file_type().is_file() {
                let path = entry.into_path();
                let virtual_path = VirtualPath::virtualize(&self.root, &path).map_err(|error| {
                    FileAccessError::Outside {
                        path: path.into(),
                        root: self.root.clone().into(),
                        kind: std::io::ErrorKind::InvalidInput,
                        message: eco_format!("{error}"),
                    }
                })?;
                files.push(virtual_path);
            }
        }
        files
            .make_mut()
            .sort_by(|left, right| left.get_with_slash().cmp(right.get_with_slash()));
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
            message: eco_format!("{error}"),
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
) -> Result<EcoVec<VirtualPath>, FileAccessError> {
    let mut files = project_files.list(directory, required)?;
    files.retain(|path| path.extension().is_some_and(|extension| extension == "typ"));
    Ok(files)
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
