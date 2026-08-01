use anyhow::{Context, Result};
use typst_kit::watcher::Watcher;

use crate::build::pipeline::BuildDriver;
use crate::cli::diag;
use crate::foundation::config::AsterConfig;
use crate::foundation::project::ProjectRoot;

pub fn run(project: ProjectRoot) -> Result<()> {
    let mut watcher = Watcher::new(None)
        .map_err(anyhow::Error::msg)
        .context("failed to initialize file watcher")?;
    let mut driver = BuildDriver::new(project.clone());
    let mut dependencies = Vec::new();

    watcher
        .update(project.watch_paths(&dependencies))
        .map_err(anyhow::Error::msg)
        .context("failed to watch project inputs")?;
    diag::emit_watching(project.root());

    loop {
        match AsterConfig::load(&project.config_file()).context("failed to parse aster.toml") {
            Ok(config) => {
                let result = driver.build(config);
                dependencies = driver.dependencies();
                match result {
                    Ok(outcome) => outcome.report(),
                    Err(error) => diag::emit_error(&format!("{error:#}")),
                }
            }
            Err(error) => diag::emit_error(&format!("{error:#}")),
        }

        watcher
            .update(project.watch_paths(&dependencies))
            .map_err(anyhow::Error::msg)
            .context("failed to update watched inputs")?;
        watcher
            .wait()
            .map_err(anyhow::Error::msg)
            .context("failed while watching project inputs")?;
        diag::emit_rebuilding();
    }
}
