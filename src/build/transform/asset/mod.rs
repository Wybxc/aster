mod component;
mod css;
mod data_url;
mod image;
mod reference;
mod script;

use std::fmt::Write;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use comemo::Tracked;
use typst::ecow::EcoString;
use typst::foundations::Bytes;
use typst::syntax::{Span, VirtualPath};
use typst_html::{HtmlDocument, HtmlElement};

use crate::build::files::ProjectFiles;
use crate::build::output::PagePublication;
use crate::foundation::config::{AssetsConfig, CssConfig};

use self::{
    css::{BundledStylesheet, CssPipeline, StylesheetKind},
    data_url::decode_data_url,
    image::{ImageProcessor, ImageSizeLimit},
    reference::{resolve_project_reference, source_origin},
    script::ScriptPipeline,
};
use super::{
    Processor, WalkControl,
    dom::HtmlElementExt,
    url::{UrlReference, classify_url},
};

pub use self::component::ComponentResources;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ScriptKind {
    Classic,
    Module,
}

/// Discovers, builds, and publishes resources referenced by a page.
pub struct AssetProcessor<'a> {
    project_files: Tracked<'a, ProjectFiles>,
    inline_threshold: usize,
    images: ImageProcessor,
    css: CssPipeline<'a>,
    scripts: ScriptPipeline<'a>,
}

impl<'a> AssetProcessor<'a> {
    pub fn new(
        project_files: Tracked<'a, ProjectFiles>,
        project_root: &Path,
        assets: &AssetsConfig,
        css: &CssConfig,
    ) -> Result<Self> {
        let images = ImageProcessor::new(&assets.images)?;
        Ok(Self {
            project_files,
            inline_threshold: assets.image_inline_threshold,
            images,
            css: CssPipeline::new(project_files, css)?,
            scripts: ScriptPipeline::new(project_files, project_root),
        })
    }

    fn add_stylesheet_file(
        &mut self,
        source: &VirtualPath,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        self.add_stylesheet(StylesheetKind::Css, source, page)
    }

    fn add_stylesheet_raw(
        &mut self,
        origin: &VirtualPath,
        name: &VirtualPath,
        code: EcoString,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        let stylesheet = self.css.bundle_raw(origin, code, page.project_root())?;
        let content = self.resolve_stylesheet(stylesheet, page)?;
        page.add_bundled_stylesheet(name, content)
    }

    fn add_stylesheet(
        &mut self,
        kind: StylesheetKind,
        source: &VirtualPath,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        let stylesheet = self
            .css
            .bundle_stylesheet(kind, source, page.project_root())?;
        let content = self.resolve_stylesheet(stylesheet, page)?;
        page.add_bundled_stylesheet(source, content)
    }

    fn resolve_stylesheet(
        &self,
        stylesheet: BundledStylesheet,
        page: &mut PagePublication<'_>,
    ) -> Result<Bytes> {
        stylesheet.resolve_references(|source, content| {
            let content = self.images.optimize(content, ImageSizeLimit::default());
            page.add_css_asset(source, content)
        })
    }

    fn add_script_file(
        &mut self,
        kind: ScriptKind,
        source: &VirtualPath,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        self.scripts.add_file(kind, source, page)
    }

    fn add_script_raw(
        &mut self,
        kind: ScriptKind,
        origin: &VirtualPath,
        code: EcoString,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        self.scripts.add_raw(kind, origin, code, page)
    }

    fn process_stylesheet_link(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<bool> {
        if !element.is_tag(typst_html::tag::link) {
            return Ok(false);
        }
        let Some(relation) = element.get_attr("rel") else {
            return Ok(false);
        };
        let (kind, custom_relation) = if relation.eq_ignore_ascii_case("tailwind") {
            (StylesheetKind::Tailwind, true)
        } else if relation
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("stylesheet"))
        {
            (StylesheetKind::Css, false)
        } else {
            return Ok(false);
        };

        let href = element.get_attr("href").ok_or_else(|| {
            anyhow::anyhow!("link element with rel=\"{relation}\" is missing href attribute")
        })?;
        let Some(reference) = resolve_project_reference(page, element.span, classify_url(&href))?
        else {
            ensure!(
                !custom_relation,
                "link element with rel=\"{relation}\" must reference a project stylesheet"
            );
            return Ok(true);
        };
        let url = self.add_stylesheet(kind, &reference.source, page)?;
        let url = reference.with_url(url);
        element.update_attr("href", move |value| *value = url);
        if custom_relation {
            element.update_attr("rel", |value| *value = "stylesheet".into());
        }
        Ok(true)
    }

    fn process_module_preload(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<bool> {
        if !element.is_tag(typst_html::tag::link)
            || !element.get_attr("rel").is_some_and(|relation| {
                relation
                    .split_ascii_whitespace()
                    .any(|token| token.eq_ignore_ascii_case("modulepreload"))
            })
        {
            return Ok(false);
        }
        let Some(reference) = element.get_attr("href") else {
            return Ok(true);
        };
        let Some(reference) =
            resolve_project_reference(page, element.span, classify_url(&reference))?
        else {
            return Ok(true);
        };
        let url = self
            .scripts
            .add_file(ScriptKind::Module, &reference.source, page)?;
        let url = reference.with_url(url);
        element.update_attr("href", move |value| *value = url);
        Ok(true)
    }

    fn process_script(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<Option<WalkControl>> {
        if !element.is_tag(typst_html::tag::script) {
            return Ok(None);
        }
        let Some(kind) = script_kind(element) else {
            return Ok(Some(WalkControl::Continue));
        };
        let url = if let Some(reference) = element.get_attr("src") {
            let Some(reference) =
                resolve_project_reference(page, element.span, classify_url(&reference))?
            else {
                return Ok(Some(WalkControl::Continue));
            };
            let url = self.scripts.add_file(kind, &reference.source, page)?;
            reference.with_url(url)
        } else {
            if kind == ScriptKind::Classic {
                return Ok(Some(WalkControl::Continue));
            }
            let origin = source_origin(page, element.span)?;
            let code = element.inner_text();
            self.scripts.add_raw(kind, &origin, code, page)?
        };

        if element.get_attr("src").is_some() {
            element.update_attr("src", move |value| *value = url);
            Ok(Some(WalkControl::Continue))
        } else {
            element.attrs.push(typst_html::attr::src, url);
            element.children.clear();
            Ok(Some(WalkControl::SkipChildren))
        }
    }

    fn publish_attribute(
        &self,
        element: &mut HtmlElement,
        attribute: &str,
        limit: ImageSizeLimit,
        page: &mut PagePublication<'_>,
    ) -> Result<()> {
        let Some(reference) = element.get_attr(attribute) else {
            return Ok(());
        };
        let Some(url) = self
            .publish_reference(page, element.span, &reference, limit)
            .with_context(|| format!("invalid {attribute} resource reference {reference}"))?
        else {
            return Ok(());
        };
        element.update_attr(attribute, move |value| *value = url);
        Ok(())
    }

    fn publish_srcset(
        &self,
        element: &mut HtmlElement,
        attribute: &str,
        page: &mut PagePublication<'_>,
    ) -> Result<()> {
        let Some(value) = element.get_attr(attribute) else {
            return Ok(());
        };
        let element_limit = ImageSizeLimit::from_element(element);
        let mut candidates = parse_srcset::parse_srcset(&value);
        let mut changed = false;
        for candidate in &mut candidates {
            let limit =
                element_limit.for_candidate(candidate.width, candidate.height, candidate.density);
            let Some(url) = self
                .publish_reference(page, element.span, &candidate.url, limit)
                .with_context(|| {
                    format!("invalid {attribute} resource reference {}", candidate.url)
                })?
            else {
                continue;
            };
            candidate.url = url.into();
            changed = true;
        }
        if !changed {
            return Ok(());
        }

        let mut serialized = EcoString::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if index > 0 {
                serialized.push_str(", ");
            }
            serialized.push_str(&candidate.url);
            if let Some(width) = candidate.width {
                write!(serialized, " {width}w")?;
            }
            if let Some(height) = candidate.height {
                write!(serialized, " {height}h")?;
            }
            if let Some(density) = candidate.density {
                write!(serialized, " {density}x")?;
            }
        }
        element.update_attr(attribute, move |value| *value = serialized);
        Ok(())
    }

    fn publish_reference(
        &self,
        page: &mut PagePublication<'_>,
        span: Span,
        reference: &str,
        limit: ImageSizeLimit,
    ) -> Result<Option<EcoString>> {
        let reference = classify_url(reference);
        if let UrlReference::Data { url } = reference {
            let Some(asset) = decode_data_url(url, self.inline_threshold)? else {
                return Ok(None);
            };
            let operation = tracing::trace_span!(
                "asset",
                kind = "data-url",
                message = "processed data URL asset"
            )
            .entered();
            let content = self.images.optimize(Bytes::new(asset.content), limit);
            let result = page.add_data_asset(asset.extension, content).map(Some);
            drop(operation);
            return result;
        }

        let Some(reference) = resolve_project_reference(page, span, reference)? else {
            return Ok(None);
        };
        let operation = tracing::trace_span!(
            "asset",
            source = %reference.source.get_with_slash(),
            message = "processed asset"
        )
        .entered();
        let content = self
            .project_files
            .read(&reference.source)
            .with_context(|| {
                format!(
                    "failed to read HTML resource {}",
                    reference.source.get_with_slash()
                )
            })?;
        let content = self.images.optimize(content, limit);
        let url = page.add_asset(&reference.source, content)?;
        drop(operation);
        Ok(Some(reference.with_url(url)))
    }
}

impl Processor for AssetProcessor<'_> {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        if self.process_stylesheet_link(element, page)?
            || self.process_module_preload(element, page)?
        {
            return Ok(WalkControl::Continue);
        }
        if let Some(control) = self.process_script(element, page)? {
            return Ok(control);
        }

        let tag = element.tag.resolve();
        match tag.as_str() {
            "img" => {
                let limit = ImageSizeLimit::from_element(element);
                self.publish_attribute(element, "src", limit, page)?;
                self.publish_srcset(element, "srcset", page)?;
            }
            "source" => {
                let limit = ImageSizeLimit::from_element(element);
                self.publish_attribute(element, "src", limit, page)?;
                self.publish_srcset(element, "srcset", page)?;
            }
            "video" => {
                self.publish_attribute(element, "src", ImageSizeLimit::default(), page)?;
                self.publish_attribute(element, "poster", ImageSizeLimit::default(), page)?;
            }
            "audio" | "track" | "embed" => {
                self.publish_attribute(element, "src", ImageSizeLimit::default(), page)?;
            }
            "object" => {
                self.publish_attribute(element, "data", ImageSizeLimit::default(), page)?;
            }
            "input"
                if element
                    .get_attr("type")
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("image")) =>
            {
                let limit = ImageSizeLimit::from_element(element);
                self.publish_attribute(element, "src", limit, page)?;
            }
            "link" if publishes_link(element) => {
                self.publish_attribute(element, "href", ImageSizeLimit::default(), page)?;
                self.publish_srcset(element, "imagesrcset", page)?;
            }
            "meta" if publishes_meta(element) => {
                self.publish_attribute(element, "content", ImageSizeLimit::default(), page)?;
            }
            "a" | "area" if element.get_attr("download").is_some() => {
                self.publish_attribute(element, "href", ImageSizeLimit::default(), page)?;
            }
            "image" | "use" | "feImage" => {
                let limit = ImageSizeLimit::from_element(element);
                self.publish_attribute(element, "href", limit, page)?;
                self.publish_attribute(element, "xlink:href", limit, page)?;
            }
            _ => {}
        }
        Ok(WalkControl::Continue)
    }

    fn end_document(
        &mut self,
        document: &mut HtmlDocument,
        _page: &mut PagePublication<'_>,
    ) -> Result<()> {
        self.images.optimize_frames(document)
    }
}

fn script_kind(element: &HtmlElement) -> Option<ScriptKind> {
    let Some(kind) = element.get_attr("type") else {
        return Some(ScriptKind::Classic);
    };
    match kind.trim().to_ascii_lowercase().as_str() {
        "module" => Some(ScriptKind::Module),
        ""
        | "text/javascript"
        | "application/javascript"
        | "text/ecmascript"
        | "application/ecmascript" => Some(ScriptKind::Classic),
        _ => None,
    }
}

fn publishes_link(element: &HtmlElement) -> bool {
    let Some(relation) = element.get_attr("rel") else {
        return false;
    };
    if relation.split_ascii_whitespace().any(|token| {
        matches!(
            token.to_ascii_lowercase().as_str(),
            "icon" | "apple-touch-icon" | "apple-touch-icon-precomposed" | "mask-icon" | "manifest"
        )
    }) {
        return true;
    }
    relation
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("preload"))
        && element.get_attr("as").is_some_and(|kind| {
            matches!(
                kind.to_ascii_lowercase().as_str(),
                "image" | "font" | "audio" | "video" | "fetch"
            )
        })
}

fn publishes_meta(element: &HtmlElement) -> bool {
    let value = element
        .get_attr("property")
        .or_else(|| element.get_attr("name"));
    value.is_some_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "og:image"
                | "og:image:url"
                | "og:image:secure_url"
                | "twitter:image"
                | "twitter:image:src"
                | "msapplication-tileimage"
        )
    })
}
