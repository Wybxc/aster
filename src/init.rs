use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use include_dir::{Dir, include_dir};

static PROJECT_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/default");

const DEFAULT_PROJECT_NAME: &str = "aster-site";

#[derive(Debug)]
pub struct InitOutcome {
    pub project: PathBuf,
}

impl InitOutcome {
    pub fn report(&self) {
        crate::diag::emit_initialized(&self.project);
    }
}

pub fn run(destination: PathBuf) -> Result<InitOutcome> {
    let destination = absolute(destination)?;
    prepare_destination(&destination)?;

    PROJECT_TEMPLATE.extract(&destination).with_context(|| {
        format!(
            "failed to extract project template to {}",
            destination.display()
        )
    })?;
    set_project_name(&destination)?;

    Ok(InitOutcome {
        project: destination,
    })
}

fn absolute(path: PathBuf) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .context("failed to get current directory")?
            .join(path)
    };
    Ok(path.components().collect())
}

fn prepare_destination(destination: &Path) -> Result<()> {
    match std::fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!("refusing to initialize symlink {}", destination.display())
        }
        Ok(metadata) if !metadata.is_dir() => {
            bail!("{} is not a directory", destination.display())
        }
        Ok(_) => {
            let mut entries = std::fs::read_dir(destination)
                .with_context(|| format!("failed to read {}", destination.display()))?;
            ensure!(
                entries.next().transpose()?.is_none(),
                "{} is not empty",
                destination.display()
            );
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(destination)
                .with_context(|| format!("failed to create {}", destination.display()))
        }
        Err(error) => {
            Err(error).with_context(|| format!("failed to inspect {}", destination.display()))
        }
    }
}

fn set_project_name(destination: &Path) -> Result<()> {
    let project_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_PROJECT_NAME);
    let config_path = destination.join("aster.toml");
    let config = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let default = format!("name = {DEFAULT_PROJECT_NAME:?}");
    let replacement = format!("name = {}", toml::Value::String(project_name.into()));
    let updated = config.replacen(&default, &replacement, 1);
    ensure!(updated != config, "project template is missing its name");
    std::fs::write(&config_path, updated)
        .with_context(|| format!("failed to write {}", config_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AsterConfig;
    use crate::pipeline::BuildDriver;
    use crate::project::ProjectRoot;

    #[test]
    fn initializes_a_buildable_project_with_a_real_library_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("my-site");

        run(destination.clone()).unwrap();

        assert!(destination.join("src/index.typ").is_file());
        assert!(destination.join("lib/aster/content.typ").is_file());
        assert!(!destination.join("lib").is_symlink());
        let config = std::fs::read_to_string(destination.join("aster.toml")).unwrap();
        assert!(config.contains("name = \"my-site\""));

        let project = ProjectRoot::new(destination).unwrap();
        let config = AsterConfig::load(&project.config_file()).unwrap();
        let outcome = BuildDriver::new(project).build(config).unwrap();
        assert_eq!(outcome.outputs.len(), 1);
    }

    #[test]
    fn initializes_an_existing_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("empty");
        std::fs::create_dir(&destination).unwrap();

        run(destination.clone()).unwrap();

        assert!(destination.join("aster.toml").is_file());
    }

    #[test]
    fn refuses_to_overwrite_a_nonempty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("existing");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("keep.txt"), "keep").unwrap();

        let error = run(destination.clone()).unwrap_err();

        assert!(error.to_string().contains("is not empty"));
        assert_eq!(
            std::fs::read_to_string(destination.join("keep.txt")).unwrap(),
            "keep"
        );
        assert!(!destination.join("aster.toml").exists());
    }
}
