//! Build orchestration and output publication.
//!
//! This layer combines the foundation and engine layers behind [`BuildSession`]
//! and the one-shot [`build`] function.

mod output;
mod pipeline;
mod transform;
mod world;

pub use pipeline::{BuildOutcome, BuildSession, build};
