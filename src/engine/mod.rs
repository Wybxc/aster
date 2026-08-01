//! Engine layer: pure domain logic with no filesystem access.
//!
//! This layer mirrors the `typst` core crate: route planning and the content
//! protocol operate purely on Typst values and paths. It has no dependencies
//! on the foundation or build layers.

pub mod content;
pub mod route;
