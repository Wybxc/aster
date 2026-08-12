//! Execution boundary for explicit external site postprocessors.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, ensure};

use crate::build::output::StagedPublication;
use crate::foundation::PostprocessConfig;

pub fn run(
    processors: &[PostprocessConfig],
    project_root: &Path,
    site: &mut StagedPublication,
) -> Result<()> {
    for processor in processors {
        run_one(processor, project_root, site)
            .with_context(|| format!("postprocessor `{}` failed", processor.name))?;
    }
    Ok(())
}

fn run_one(
    processor: &PostprocessConfig,
    project_root: &Path,
    site: &mut StagedPublication,
) -> Result<()> {
    let (executable, arguments) = processor
        .command
        .split_first()
        .context("postprocessor command cannot be empty")?;
    ensure!(
        !processor.name.is_empty(),
        "postprocessor name cannot be empty"
    );
    let uses_output = arguments.iter().any(|argument| argument == "{output}");
    ensure!(
        uses_output == processor.mount.is_some(),
        "postprocessor `{{output}}` and `mount` must be configured together"
    );
    let output = uses_output
        .then(tempfile::tempdir)
        .transpose()
        .context("failed to create private postprocessor output")?;
    let stage = tracing::info_span!(
        "postprocess",
        tool = %processor.name,
        message = "ran postprocessor"
    )
    .entered();
    let mut command = Command::new(executable.as_str());
    command.current_dir(project_root);
    for argument in arguments {
        match argument.as_str() {
            "{site}" => command.arg(site.root()),
            "{output}" => command.arg(
                output
                    .as_ref()
                    .expect("validated private output exists")
                    .path(),
            ),
            value => command.arg(value),
        };
    }
    let result = command.output().with_context(|| {
        format!(
            "failed to run `{}`; install it and ensure it is available on PATH",
            executable
        )
    })?;
    ensure!(
        result.status.success(),
        "command exited with {}{}",
        result.status,
        if result.stderr.is_empty() {
            String::new()
        } else {
            format!(": {}", String::from_utf8_lossy(&result.stderr).trim())
        }
    );
    drop(stage);

    if let (Some(mount), Some(output)) = (&processor.mount, output) {
        site.import(mount, output.path())?;
    }
    Ok(())
}
