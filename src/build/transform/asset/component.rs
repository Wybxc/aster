use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use typst::ecow::EcoString;
use typst::foundations::{Content, Packed, Value};
use typst::introspection::{Introspector, MetadataElem};
use typst::model::ParbreakElem;
use typst::syntax::{FileId, Span, VirtualPath, VirtualRoot};
use typst::text::{LinebreakElem, RawElem, SpaceElem};
use typst_html::{HtmlDocument, HtmlElement};

use crate::build::output::PagePublication;

use super::{AssetProcessor, ScriptKind};
use crate::build::transform::dom::append_to_head;

/// Resources emitted by the component modules used to render one page.
pub(crate) struct ComponentResources {
    declarations: Vec<ResourceDeclaration>,
}

impl ComponentResources {
    /// Collect declarations before any postprocessor mutates the HTML DOM.
    pub fn collect(document: &HtmlDocument) -> Result<Self> {
        let mut declarations = Vec::<ResourceDeclaration>::new();
        let mut slots = HashMap::<(FileId, ResourceKind, Span), usize>::new();

        for content in document.introspector().query_labelled() {
            let Some(metadata) = content.to_packed::<MetadataElem>() else {
                continue;
            };
            let Some(label) = content.label() else {
                continue;
            };
            let kind = match label.resolve().as_str() {
                "aster-style" => ResourceKind::Style,
                "aster-script" => ResourceKind::Script,
                "aster-module" => ResourceKind::Module,
                _ => continue,
            };
            let span = content.span();
            let component = span.id().with_context(|| {
                format!(
                    "{} resource declaration has no source component",
                    kind.name()
                )
            })?;
            let resource = Resource::parse(kind, &metadata.value, span).with_context(|| {
                format!(
                    "invalid {} resource in component {:?}",
                    kind.name(),
                    component.get()
                )
            })?;
            let declaration = ResourceDeclaration {
                component,
                span,
                resource,
            };

            let slot = (component, kind, span);
            if let Some(&index) = slots.get(&slot) {
                ensure!(
                    declarations[index] == declaration,
                    "{} resource declaration in component {:?} produced different values",
                    kind.name(),
                    component.get()
                );
                continue;
            }
            slots.insert(slot, declarations.len());
            declarations.push(declaration);
        }

        Ok(Self { declarations })
    }

    /// Build declared resources and attach their final HTML elements.
    pub fn apply(
        self,
        document: &mut HtmlDocument,
        page: &mut PagePublication<'_>,
        assets: &mut AssetProcessor<'_>,
    ) -> Result<()> {
        for declaration in self.declarations {
            let ResourceDeclaration {
                component,
                span,
                resource,
            } = declaration;
            match resource {
                Resource::Style(source) => {
                    let url = match source {
                        StyleSource::File(reference) => {
                            let source = resolve_source(page, component, &reference)?;
                            assets.add_stylesheet_file(&source, page)?
                        }
                        StyleSource::Inline { code, origin } => {
                            let origin = origin.id().context("raw style has no source file")?;
                            ensure!(
                                matches!(origin.root(), VirtualRoot::Project),
                                "raw style originates outside the project"
                            );
                            let component = component_project_source(&component)?;
                            assets.add_stylesheet_raw(origin.vpath(), component, code, page)?
                        }
                    };
                    let link = HtmlElement::new(typst_html::tag::link)
                        .with_attr(typst_html::attr::rel, "stylesheet")
                        .with_attr(typst_html::attr::href, url)
                        .spanned(span);
                    append_to_head(document, link);
                }
                Resource::Script(kind, source) => {
                    let url = match source {
                        ScriptSource::File(reference) => {
                            let source = resolve_source(page, component, &reference)?;
                            assets
                                .add_script_file(kind, &source, page)
                                .with_context(|| {
                                    format!(
                                        "failed to build component script {}",
                                        source.get_with_slash()
                                    )
                                })?
                        }
                        ScriptSource::Inline(code) => {
                            let component = component_project_source(&component)?;
                            assets.add_script_raw(kind, component, code, page)?
                        }
                    };
                    let mut script = HtmlElement::new(typst_html::tag::script)
                        .with_attr(typst_html::attr::src, url);
                    script = match kind {
                        ScriptKind::Classic => script.with_attr(typst_html::attr::defer, ""),
                        ScriptKind::Module => script.with_attr(typst_html::attr::r#type, "module"),
                    };
                    append_to_head(document, script.spanned(span));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceDeclaration {
    component: FileId,
    span: Span,
    resource: Resource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Resource {
    Style(StyleSource),
    Script(ScriptKind, ScriptSource),
}

impl Resource {
    fn parse(kind: ResourceKind, value: &Value, declaration: Span) -> Result<Self> {
        match value {
            Value::Str(path) => {
                ensure!(!path.is_empty(), "resource path cannot be empty");
                let path = path.clone().into();
                Ok(match kind {
                    ResourceKind::Style => Self::Style(StyleSource::File(path)),
                    ResourceKind::Script => {
                        Self::Script(ScriptKind::Classic, ScriptSource::File(path))
                    }
                    ResourceKind::Module => {
                        Self::Script(ScriptKind::Module, ScriptSource::File(path))
                    }
                })
            }
            Value::Content(content) => {
                let raw = unique_raw(content)?;
                let text = raw
                    .pack_ref()
                    .field_by_name("text")
                    .map_err(|error| anyhow::anyhow!("failed to read raw text: {error}"))?;
                let Value::Str(text) = text else {
                    unreachable!("RawElem text must be a string");
                };
                ensure!(
                    raw.lang
                        .as_option()
                        .as_ref()
                        .and_then(|value| value.as_ref())
                        .is_some_and(|value| value.as_str() == kind.language()),
                    "{} raw resource must use the `{}` language tag",
                    kind.name(),
                    kind.language()
                );
                let code = text.into();
                let raw_span = raw.span();
                Ok(match kind {
                    ResourceKind::Style => Self::Style(StyleSource::Inline {
                        code,
                        origin: raw_span.id().map_or(declaration, |_| raw_span),
                    }),
                    ResourceKind::Script => {
                        Self::Script(ScriptKind::Classic, ScriptSource::Inline(code))
                    }
                    ResourceKind::Module => {
                        Self::Script(ScriptKind::Module, ScriptSource::Inline(code))
                    }
                })
            }
            _ => bail!(
                "{} resource must contain a path string or raw `{}` content",
                kind.name(),
                kind.language()
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StyleSource {
    File(EcoString),
    Inline { code: EcoString, origin: Span },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ScriptSource {
    File(EcoString),
    Inline(EcoString),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ResourceKind {
    Style,
    Script,
    Module,
}

impl ResourceKind {
    fn name(self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Script => "script",
            Self::Module => "module",
        }
    }

    fn language(self) -> &'static str {
        match self {
            Self::Style => "css",
            Self::Script | Self::Module => "js",
        }
    }
}

fn unique_raw(content: &Content) -> Result<&Packed<RawElem>> {
    let mut raw = None;
    let mut unexpected = None;
    content.sequence_recursive_for_each(&mut |child| {
        if child.is::<SpaceElem>() || child.is::<LinebreakElem>() || child.is::<ParbreakElem>() {
            return;
        }
        if let Some(element) = child.to_packed::<RawElem>()
            && raw.is_none()
        {
            raw = Some(element);
        } else {
            unexpected = Some(child);
        }
    });

    if unexpected.is_some() {
        bail!("resource content must contain exactly one raw element and whitespace");
    }
    raw.context("resource content does not contain a raw element")
}

fn resolve_source(
    page: &PagePublication<'_>,
    component: FileId,
    reference: &str,
) -> Result<VirtualPath> {
    if !reference.starts_with('/') {
        ensure!(
            matches!(component.root(), VirtualRoot::Project),
            "relative component resource {reference} originates outside the project"
        );
    }
    page.resolve_source_from(component.vpath(), reference)
}

fn component_project_source(component: &FileId) -> Result<&VirtualPath> {
    ensure!(
        matches!(component.root(), VirtualRoot::Project),
        "component resources from packages cannot be published yet"
    );
    Ok(component.vpath())
}
