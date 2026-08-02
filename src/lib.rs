//! Build Typst-authored Aster projects into static sites.
//!
//! The root exports the common build Interface. The [`foundation`] and
//! [`engine`] modules expose reusable lower layers.

pub mod build;
pub mod engine;
pub mod foundation;

pub use build::{BuildOutcome, BuildSession, BuildWarning};
pub use foundation::{FilesystemDependency, Project};
