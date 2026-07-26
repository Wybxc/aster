use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;

/// Process CSS files referenced by page templates.
///
/// Scans `src_dir` for `.css` files, filters to those referenced by
/// `allowed_refs` (relative paths like `"style.css"`), bundles each with
/// lightningcss (`@import` resolution), minifies, and writes to `dist_dir`.
///
/// Returns the list of written output paths.
pub fn run(src_dir: &Path, dist_dir: &Path) -> Result<Vec<PathBuf>> {
    //
    // Collect CSS references from page HTML
    //
    // Look for <link rel="stylesheet" href="..."> in every HTML file under
    // dist_dir so we only bundle CSS that pages actually use.
    let allowed_refs = {
        let mut refs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dist_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "html") {
                    let html = match std::fs::read_to_string(&path) {
                        Ok(h) => h,
                        Err(_) => continue,
                    };
                    for (i, _) in html.match_indices("rel=\"stylesheet\"") {
                        // Walk backwards to find the enclosing <link.
                        let before = &html[..i];
                        if let Some(link_start) = before.rfind('<') {
                            let link = &html[link_start..];
                            // Find href="..." inside this <link>.
                            if let Some(href_pos) = link.find("href=\"") {
                                let start = href_pos + 6;
                                if let Some(end) = link[start..].find('"') {
                                    refs.push(link[start..start + end].to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        refs
    };

    let mut outputs = Vec::new();

    let all_files = collect_css_files(src_dir);
    if all_files.is_empty() {
        return Ok(outputs);
    }

    for entry in &all_files {
        let relative = entry
            .strip_prefix(src_dir)
            .expect("file must be under src/")
            .to_string_lossy()
            .into_owned();

        // Skip files not referenced by any page template.
        if !allowed_refs.contains(&relative) {
            continue;
        }

        let output = dist_dir.join(&relative);
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
