//! Pure Aster domain logic with no filesystem access.
//!
//! Route patterns and the content protocol operate purely on Typst values and
//! paths. This layer has no dependencies on the foundation or build layers.

pub(crate) mod content;
pub(crate) mod endpoint;
pub mod route;
