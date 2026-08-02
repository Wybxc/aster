use std::path::PathBuf;

use anyhow::Result;
use aster::BuildSession;

use crate::cli::{diag, resolve_project};

pub fn run(project_dir: Option<PathBuf>) -> Result<()> {
    let project = resolve_project(project_dir)?;
    let outcome = BuildSession::new(project)?.build()?;
    diag::report_build(&outcome);
    Ok(())
}
