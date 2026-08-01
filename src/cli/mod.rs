//! CLI layer: command-line application.
//!
//! This layer mirrors the `typst-cli` crate: terminal rendering, watch and
//! init commands, and process entry. It depends on the build layer.

pub mod diag;
pub mod init;
pub mod watch;
