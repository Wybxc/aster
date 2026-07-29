use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst_html::HtmlDocument;

use super::{ElementProcessor, ProcessingContext, WalkControl, html_util};

pub(super) struct CssProcessor;

impl ElementProcessor for CssProcessor {
    fn process(&self, doc: &mut HtmlDocument, ctx: &ProcessingContext<'_>) -> Result<()> {
        super::walk_mut(doc.root_mut(), &mut |elem| {
            if !html_util::is_tag(elem, typst_html::tag::link) {
                return Ok(WalkControl::Continue);
            }
            if !html_util::has_attr(elem, "rel", |v| v.as_str() == "css") {
                return Ok(WalkControl::Continue);
            }

            let href = match html_util::get_attr(elem, "href") {
                Some(h) => h,
                None => return Ok(WalkControl::Continue),
            };

            // Resolve the source CSS file relative to the template's subdirectory.
            let source = normalize(
                &ctx.src_dir()
                    .join(ctx.template_subdir())
                    .join(href.as_str()),
            );
            let css = bundle_file(&source)?;

            let hash = format!("{:016x}", seahash::hash(css.as_bytes()));
            let h = href.as_str();
            let stem = Path::new(h)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let ext = Path::new(h)
                .extension()
                .unwrap_or_default()
                .to_string_lossy();
            let hashed_name = format!("{stem}.{hash}.{ext}");

            // Write the bundled CSS to dist_dir.
            let css_output = ctx.output_dir().join(&hashed_name);
            if let Some(parent) = css_output.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            std::fs::write(&css_output, &css)
                .with_context(|| format!("failed to write {}", css_output.display()))?;

            // Compute relative path from the page to the CSS file.
            let page_dir = ctx.page_path.parent().expect("page has a parent");
            let relative = relative_path(page_dir, &css_output);

            html_util::update_attr(elem, "href", |v| *v = relative.to_string_lossy().into());
            html_util::update_attr(elem, "rel", |v| *v = "stylesheet".into());
            Ok(WalkControl::Continue)
        })
    }
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

/// Resolve `..` components in a path without requiring the file to exist.
fn normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => result.push(comp),
        }
    }
    result
}

/// Compute a relative path from `from` (a directory) to `to` (a file).
fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut result = PathBuf::new();
    for _ in common..from.len() {
        result.push("..");
    }
    for comp in &to[common..] {
        result.push(comp);
    }
    result
}
