use std::path::Path;

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;

/// Bundle a single CSS file referenced by its `href` (relative to `src_dir`),
/// write the result to `dist_dir` with a content hash in the filename,
/// and return the hashed filename.
///
/// The existence check and dispatch happen in [`super::LinkAsset::from_link`],
/// so this function can assume the file exists.
pub(super) fn bundle_relative(href: &str, src_dir: &Path, dist_dir: &Path) -> Result<String> {
    let entry = src_dir.join(href);
    let css = bundle_file(&entry)?;

    let hash = format!("{:016x}", seahash::hash(css.as_bytes()));

    let path = Path::new(href);
    let stem = path.file_stem().unwrap_or_default();
    let ext = path.extension().unwrap_or_default();
    let hashed_name = format!(
        "{stem}.{hash}.{ext}",
        stem = stem.to_string_lossy(),
        ext = ext.to_string_lossy(),
    );

    let output = dist_dir.join(&hashed_name);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    std::fs::write(&output, &css)
        .with_context(|| format!("failed to write {}", output.display()))?;

    Ok(hashed_name)
}

/// Bundle a single CSS entry point (resolve `@import`, prefix, minify).
fn bundle_file(entry: &Path) -> Result<String> {
    let fs = FileProvider::new();
    let mut bundler = Bundler::new(&fs, None, ParserOptions::default());

    let mut stylesheet = bundler
        .bundle(entry)
        .map_err(|e| anyhow::anyhow!("failed to bundle {}: {e:#}", entry.display()))?;

    stylesheet
        .minify(MinifyOptions {
            targets: Browsers::default().into(),
            ..MinifyOptions::default()
        })
        .map_err(|e| anyhow::anyhow!("failed to minify CSS: {e:#}"))?;

    let result = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        })
        .map_err(|e| anyhow::anyhow!("failed to serialize CSS: {e:#}"))?;

    Ok(result.code)
}
