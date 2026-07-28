use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst_html::HtmlElement;

use super::{ElementProcessor, ProcessingContext, WalkControl};

/// Processor that bundles `<link rel="css">` references through lightningcss.
pub(super) struct CssProcessor;

impl ElementProcessor for CssProcessor {
    fn matches(&self, elem: &HtmlElement) -> bool {
        if elem.tag != typst_html::tag::link {
            return false;
        }
        elem.attrs
            .0
            .iter()
            .any(|(a, v)| *a.resolve() == *"rel" && v.as_str() == "css")
    }

    fn process(&self, elem: &mut HtmlElement, ctx: &ProcessingContext) -> Result<WalkControl> {
        let href = elem
            .attrs
            .0
            .iter()
            .find_map(|(a, v)| (*a.resolve() == *"href").then(|| v.clone()));
        let Some(href) = href else {
            return Ok(WalkControl::Continue);
        };

        // Resolve the source CSS file relative to the template's subdirectory.
        let source = normalize(&ctx.src_dir.join(ctx.template_subdir()).join(href.as_str()));
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
        let css_output = ctx.dist_dir.join(&hashed_name);
        if let Some(parent) = css_output.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        std::fs::write(&css_output, &css)
            .with_context(|| format!("failed to write {}", css_output.display()))?;

        // Compute relative path from the page to the CSS file.
        let page_dir = ctx.page_path.parent().expect("page has a parent");
        let relative = relative_path(page_dir, &css_output);

        for (a, v) in elem.attrs.0.make_mut().iter_mut() {
            if *a.resolve() == *"href" {
                *v = relative.to_string_lossy().into();
            } else if *a.resolve() == *"rel" {
                *v = "stylesheet".into();
            }
        }
        Ok(WalkControl::Continue)
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
