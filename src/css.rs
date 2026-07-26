use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;

/// Bundle each CSS entry point, compute a content hash, write the result to
/// `dist_dir` with the hash embedded in the filename (e.g. `style.a1b2c3d4.css`),
/// and return a mapping from original relative path → hashed filename.
///
/// Only files whose `src_dir`-relative path appears in `allowed_refs` are
/// processed (they must be directly referenced by page templates).
pub fn run(
    src_dir: &Path,
    dist_dir: &Path,
    allowed_refs: &[String],
) -> Result<HashMap<String, String>> {
    let mut mapping = HashMap::new();

    let all_files = collect_css_files(src_dir);
    if all_files.is_empty() {
        return Ok(mapping);
    }

    for entry in &all_files {
        let relative = entry
            .strip_prefix(src_dir)
            .expect("file must be under src/")
            .to_string_lossy()
            .into_owned();

        if !allowed_refs.contains(&relative) {
            continue;
        }

        let css = bundle_file(entry)?;

        // Content hash for cache busting (using seahash, same approach as Trunk).
        let hash = format!("{:016x}", seahash::hash(css.as_bytes()));

        // Insert hash before extension: style.css → style.{hash}.css
        let stem = entry.file_stem().unwrap_or_default();
        let ext = entry.extension().unwrap_or_default();
        let hashed_name = format!("{stem}.{hash}.{ext}", stem = stem.to_string_lossy(), ext = ext.to_string_lossy());

        let output = dist_dir.join(&hashed_name);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        std::fs::write(&output, &css)
            .with_context(|| format!("failed to write {}", output.display()))?;

        mapping.insert(relative, hashed_name);
    }

    Ok(mapping)
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
