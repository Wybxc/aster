use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use typst_kit::watcher::Watcher;

use crate::config::AsterConfig;
use crate::pipeline::BuildDriver;
use crate::project::ProjectRoot;
use crate::{diag, report_outcome};

pub fn run(project: ProjectRoot) -> Result<()> {
    let mut watcher = Watcher::new(None)
        .map_err(anyhow::Error::msg)
        .context("failed to initialize file watcher")?;
    let mut driver = BuildDriver::new(project.clone());
    let mut dependencies = Vec::new();

    watcher
        .update(watch_paths(&project, &dependencies))
        .map_err(anyhow::Error::msg)
        .context("failed to watch project inputs")?;
    diag::emit_watching(project.root());

    loop {
        match AsterConfig::load(&project.config_file()).context("failed to parse aster.toml") {
            Ok(config) => {
                let result = driver.build(config);
                dependencies = driver.dependencies();
                match result {
                    Ok(outcome) => report_outcome(&outcome),
                    Err(error) => diag::emit_error(&format!("{error:#}")),
                }
            }
            Err(error) => diag::emit_error(&format!("{error:#}")),
        }

        watcher
            .update(watch_paths(&project, &dependencies))
            .map_err(anyhow::Error::msg)
            .context("failed to update watched inputs")?;
        watcher
            .wait()
            .map_err(anyhow::Error::msg)
            .context("failed while watching project inputs")?;
        diag::emit_rebuilding();
    }
}

fn watch_paths(project: &ProjectRoot, dependencies: &[PathBuf]) -> Vec<PathBuf> {
    let output = project.output_dir();
    let canonical_output = std::fs::canonicalize(project.root())
        .ok()
        .map(|root| root.join("dist"));
    let mut paths = project.structural_watch_paths();
    paths.extend(
        dependencies
            .iter()
            .filter(|path| !inside_output(path, &output, canonical_output.as_deref()))
            .cloned(),
    );
    paths.sort();
    paths.dedup();
    paths
}

fn inside_output(path: &Path, output: &Path, canonical_output: Option<&Path>) -> bool {
    path.starts_with(output)
        || canonical_output.is_some_and(|canonical| path.starts_with(canonical))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_paths_merge_dependencies_and_exclude_output() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src/blog")).unwrap();
        std::fs::create_dir_all(root.join("dist")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = ProjectRoot::new(root.to_owned()).unwrap();
        let theme = root.join("theme.tmTheme");
        let generated = project.output_dir().join("index.html");

        let paths = watch_paths(&project, &[theme.clone(), generated.clone()]);

        assert!(paths.contains(&theme));
        assert!(paths.contains(&project.src_dir().join("blog")));
        assert!(!paths.contains(&generated));
        assert!(
            !paths
                .iter()
                .any(|path| path.starts_with(project.output_dir()))
        );
    }
}
