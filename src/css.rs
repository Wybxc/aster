use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;

/// Discover every `.css` file under `src_dir`, bundle + prefix + minify each,
/// and write results to `dist_dir` preserving relative paths.
///
/// Returns the list of written output paths.
pub fn run(src_dir: &Path, dist_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut outputs = Vec::new();

    let entries = collect_css_files(src_dir);
    if entries.is_empty() {
        return Ok(outputs);
    }

    for entry in &entries {
        let relative = entry
            .strip_prefix(src_dir)
            .expect("file must be under src/");
        let output = dist_dir.join(relative);

        let css = bundle_file(entry)?;

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        std::fs::write(&output, &css)
            .with_context(|| format!("failed to write {}", output.display()))?;
        outputs.push(output);
    }

    Ok(outputs)
}

/// Bundle a single CSS entry point (resolve `@import`, prefix, minify).
fn bundle_file(entry: &Path) -> Result<String> {
    let fs = FileProvider::new();
    let mut bundler = Bundler::new(&fs, None, ParserOptions::default());

    let mut stylesheet = bundler.bundle(entry)
        .map_err(|e| anyhow::anyhow!("failed to bundle {}: {e:#}", entry.display()))?;

    stylesheet.minify(MinifyOptions {
        targets: Browsers::default().into(),
        ..MinifyOptions::default()
    }).map_err(|e| anyhow::anyhow!("failed to minify CSS: {e:#}"))?;

    let result = stylesheet.to_css(PrinterOptions {
        minify: true,
        ..PrinterOptions::default()
    }).map_err(|e| anyhow::anyhow!("failed to serialize CSS: {e:#}"))?;

    Ok(result.code)
}

/// Recursively collect all `.css` files under a directory.
fn collect_css_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(mut stack) = dir.canonicalize().map(|p| vec![p]) else {
        return files;
    };

    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "css") {
                files.push(path);
            }
        }
    }
    files
}
