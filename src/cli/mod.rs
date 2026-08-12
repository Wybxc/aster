//! CLI layer: command-line application.
//!
//! This layer owns terminal rendering and command loops. Build behavior and
//! filesystem watching remain behind the library APIs it drives.

use std::path::PathBuf;

use anyhow::{Context, Result};
use aster::Project;

pub mod build;
pub mod dev;
pub mod init;
pub mod telemetry;
pub mod watch;

fn resolve_project(project_dir: Option<PathBuf>) -> Result<Project> {
    match project_dir {
        Some(dir) => Project::open(dir),
        None => {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            Project::find(&cwd).context("no aster.toml found in current or parent directories")
        }
    }
}
