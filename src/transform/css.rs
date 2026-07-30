use std::path::Path;

use anyhow::{Context, Result};
use lightningcss::bundler::{Bundler, FileProvider};
use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions};
use lightningcss::targets::Browsers;
use typst_html::HtmlDocument;

use super::{ElementProcessor, ProcessingContext, WalkControl};
use crate::utils::{AssetCollector, HtmlElementExt};

pub(super) struct CssProcessor;

impl ElementProcessor for CssProcessor {
    fn process(
        &self,
        doc: &mut HtmlDocument,
        assets: &mut AssetCollector,
        ctx: &ProcessingContext<'_>,
    ) -> Result<()> {
        doc.root_mut().walk_mut(&mut |elem| {
            if !elem.is_tag(typst_html::tag::link) {
                return Ok(WalkControl::Continue);
            }
            if !elem.has_attr("rel", |v| v.as_str() == "css") {
                return Ok(WalkControl::Continue);
            }

            let href = match elem.get_attr("href") {
                Some(h) => h,
                None => return Ok(WalkControl::Continue),
            };

            // Resolve the source CSS file relative to the template's subdirectory.
            let source = ctx
                .src_dir()
                .join(ctx.template_subdir())
                .join(href.as_str());
            let source = std::fs::canonicalize(&source)
                .with_context(|| format!("failed to resolve {}", source.display()))?;
            let css = bundle_file(&source)?;
            let css_bytes = css.into_bytes();

            let h = href.as_str();
            let stem = Path::new(h)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let ext = Path::new(h)
                .extension()
                .unwrap_or_default()
                .to_string_lossy();
            let hashed_path = assets.add(Path::new(""), &stem, &ext, css_bytes);

            // Compute relative path from the page to the CSS file.
            let page_dir = ctx.page_path.parent().expect("page has a parent");
            let css_output = ctx.output_dir().join(&hashed_path);
            let relative =
                pathdiff::diff_paths(&css_output, page_dir).expect("both paths under output_dir");

            elem.update_attr("href", |v| *v = relative.to_string_lossy().into());
            elem.update_attr("rel", |v| *v = "stylesheet".into());
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
