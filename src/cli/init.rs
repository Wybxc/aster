use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use include_dir::{Dir, include_dir};

use crate::cli::diag;

static PROJECT_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/default");

const DEFAULT_PROJECT_NAME: &str = "aster-site";

pub fn run(destination: PathBuf) -> Result<()> {
    let destination =
        std::path::absolute(destination).context("failed to make destination path absolute")?;
    prepare_destination(&destination)?;

    PROJECT_TEMPLATE.extract(&destination).with_context(|| {
        format!(
            "failed to extract project template to {}",
            destination.display()
        )
    })?;
    set_project_name(&destination)?;

    diag::emit_initialized(&destination);
    Ok(())
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
