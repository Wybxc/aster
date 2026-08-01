//! Foundation layer: filesystem infrastructure.
//!
//! This layer mirrors the `typst-kit` crate: project layout, configuration,
//! and the tracked file stores. It depends on the engine layer's types but
//! never on the build or CLI layers.

pub mod config;
pub mod files;
pub mod project;
