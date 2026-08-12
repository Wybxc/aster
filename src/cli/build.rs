use std::path::PathBuf;

use anyhow::Result;
use aster::BuildSession;

use crate::cli::{resolve_project, telemetry};

pub fn run(project_dir: Option<PathBuf>) -> Result<()> {
    let project = resolve_project(project_dir)?;
    let outcome = BuildSession::new(project).build()?;
    telemetry::report_build(&outcome);
    Ok(())
}
