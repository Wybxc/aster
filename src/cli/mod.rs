//! CLI layer: command-line application.
//!
//! This layer mirrors the `typst-cli` crate: terminal rendering, build, watch,
//! and init commands, and process entry. It depends on the build layer.

use std::path::PathBuf;

use anyhow::{Context, Result};
use aster::Project;

pub(crate) mod build;
pub(crate) mod diag;
pub(crate) mod init;
pub(crate) mod watch;

pub(crate) fn resolve_project(project_dir: Option<PathBuf>) -> Result<Project> {
    match project_dir {
        Some(dir) => Project::open(dir),
        None => {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            Project::find(&cwd).context("no aster.toml found in current or parent directories")
        }
    }
}
