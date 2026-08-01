//! CLI layer: command-line application.
//!
//! This layer mirrors the `typst-cli` crate: terminal rendering, watch and
//! init commands, and process entry. It depends on the build layer.

pub(crate) mod diag;
pub(crate) mod init;
pub(crate) mod watch;
