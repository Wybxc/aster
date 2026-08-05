//! Project discovery, configuration, and validated layout.
//!
//! This layer defines Aster's stable project model. It does not depend on the
//! engine, build, or CLI layers; build-time filesystem state belongs to the
//! build layer instead.

use std::path::{Path, PathBuf};

pub mod config;
mod project;

pub use config::{
    AssetsConfig, AsterConfig, CssConfig, FontConfig, HighlightConfig, OutputConfig, PathsConfig,
    ProjectManifest, Themes, TypstConfig, WatchConfig,
};
pub use project::{Project, ProjectLayout};

/// A filesystem input observed or explicitly configured for the current build.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FilesystemDependency {
    /// A file whose contents were accessed or whose path was configured.
    File(PathBuf),
    /// A directory whose recursive membership was accessed or configured.
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
