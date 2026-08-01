//! Build layer: assembling foundation infrastructure into a build.
//!
//! This layer mirrors the driving part of the `typst-cli` crate: the Typst
//! build session, output publication, build pipeline, and document
//! transforms. It depends on the foundation and engine layers.

pub mod output;
pub mod pipeline;
pub mod transform;
pub mod world;
