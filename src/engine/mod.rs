//! Pure Aster domain logic with no filesystem access.
//!
//! Routes, generated files, and the runtime protocol operate purely
//! on Typst values and paths. This layer has no dependencies on the foundation
//! or build layers.

pub mod content;
pub mod route;
