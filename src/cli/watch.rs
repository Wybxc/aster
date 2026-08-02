use anyhow::{Context, Result};
use aster::{BuildSession, Project};
use typst_kit::watcher::Watcher;

use crate::cli::diag;

pub fn run(project: Project) -> Result<()> {
    let mut watcher = Watcher::new(None)
        .map_err(anyhow::Error::msg)
        .context("failed to initialize file watcher")?;
    let mut session = BuildSession::new(project.clone());

    watcher
        .update(project.watch_paths(std::iter::empty()))
        .map_err(anyhow::Error::msg)
        .context("failed to watch project inputs")?;
    diag::emit_watching(project.root());

    loop {
        let result = session.build();
        match result {
            Ok(outcome) => diag::report_build(&outcome),
            Err(error) => diag::emit_error(&format!("{error:#}")),
        }

        watcher
            .update(project.watch_paths(session.dependencies()))
            .map_err(anyhow::Error::msg)
            .context("failed to update watched inputs")?;
        watcher
            .wait()
            .map_err(anyhow::Error::msg)
            .context("failed while watching project inputs")?;
        diag::emit_rebuilding();
    }
}
