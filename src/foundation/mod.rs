//! Project discovery and filesystem infrastructure.
//!
//! This layer mirrors the `typst-kit` crate: project layout, configuration,
//! and tracked file stores. It does not depend on the engine, build, or CLI
//! layers.

pub(crate) mod config;
pub(crate) mod files;
mod project;

pub use project::Project;
pub(crate) use project::ProjectLayout;
