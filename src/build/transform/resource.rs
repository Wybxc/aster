use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use comemo::Tracked;
use typst::ecow::EcoString;
use typst::foundations::{Bytes, Content, Value};
use typst::introspection::{Introspector, MetadataElem};
use typst::model::ParbreakElem;
use typst::syntax::{FileId, Span, VirtualPath, VirtualRoot};
use typst::text::{LinebreakElem, RawElem, SpaceElem};
use typst_html::{HtmlDocument, HtmlElement};

use crate::build::output::PagePublication;
use crate::foundation::files::ProjectFiles;

use super::css::CssProcessor;
use super::dom::{append_to_body, append_to_head};

/// Resources emitted by the component modules used to render one page.
pub(crate) struct ComponentResources {
    components: Vec<Component>,
}

struct Component {
    id: FileId,
    declarations: Vec<ResourceDeclaration>,
    slots: HashMap<(ResourceKind, Span), usize>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ResourceKind {
    Style,
    Script,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceDeclaration {
    kind: ResourceKind,
    span: Span,
    source: ResourceSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSource {
    File(EcoString),
    Inline { code: EcoString, origin: Span },
}

impl ComponentResources {
    /// Collect declarations before any postprocessor mutates the HTML DOM.
    pub fn collect(document: &HtmlDocument) -> Result<Self> {
        let mut components = Vec::<Component>::new();
        let mut indices = HashMap::<FileId, usize>::new();

        for content in document.introspector().query_labelled() {
            let Some(kind) = resource_kind(&content) else {
                continue;
            };
            let Some(metadata) = content.to_packed::<MetadataElem>() else {
                continue;
            };
            let span = content.span();
            let component = span.id().with_context(|| {
                format!(
                    "{} resource declaration has no source component",
                    kind.name()
                )
            })?;
            let source = parse_source(kind, &metadata.value, span).with_context(|| {
                format!(
                    "invalid {} resource in component {:?}",
                    kind.name(),
                    component.get()
                )
            })?;
            let declaration = ResourceDeclaration { kind, span, source };

            let index = *indices.entry(component).or_insert_with(|| {
                let index = components.len();
                components.push(Component {
                    id: component,
                    declarations: Vec::new(),
                    slots: HashMap::new(),
                });
                index
            });
            components[index].insert(declaration)?;
        }

        Ok(Self { components })
    }

    /// Build declared resources and attach their final HTML elements.
    pub fn apply(
        self,
        document: &mut HtmlDocument,
        page: &mut PagePublication<'_>,
        css: &mut CssProcessor<'_>,
        project_files: Tracked<ProjectFiles>,
    ) -> Result<()> {
        for component in self.components {
            for declaration in component.declarations {
                match (declaration.kind, declaration.source) {
                    (ResourceKind::Style, ResourceSource::File(reference)) => {
                        let source = resolve_source(page, component.id, &reference)?;
                        let url = css.add_file(&source, page)?;
                        let link = HtmlElement::new(typst_html::tag::link)
                            .with_attr(typst_html::attr::rel, "stylesheet")
                            .with_attr(typst_html::attr::href, url)
                            .spanned(declaration.span);
                        append_to_head(document, link);
                    }
                    (ResourceKind::Style, ResourceSource::Inline { code, origin }) => {
                        let origin_path = project_source(origin, "raw style")?;
                        let code = css.add_raw(&origin_path, code, page)?;
                        let component_path = component_project_source(component.id)?;
                        let url =
                            page.add_bundled_stylesheet(&component_path, code.as_bytes().to_vec())?;
                        let link = HtmlElement::new(typst_html::tag::link)
                            .with_attr(typst_html::attr::rel, "stylesheet")
                            .with_attr(typst_html::attr::href, url)
                            .spanned(declaration.span);
                        append_to_head(document, link);
                    }
                    (ResourceKind::Script, ResourceSource::File(reference)) => {
                        let source = resolve_source(page, component.id, &reference)?;
                        let content = project_files.read(&source).with_context(|| {
                            format!(
                                "failed to read component script {}",
                                source.get_with_slash()
                            )
                        })?;
                        let url = page.add_script(&source, content)?;
                        let script = HtmlElement::new(typst_html::tag::script)
                            .with_attr(typst_html::attr::src, url)
                            .spanned(declaration.span);
                        append_to_body(document, script);
                    }
                    (ResourceKind::Script, ResourceSource::Inline { code, origin: _ }) => {
                        let component_path = component_project_source(component.id)?;
                        let url =
                            page.add_script(&component_path, Bytes::new(code.as_bytes().to_vec()))?;
                        let script = HtmlElement::new(typst_html::tag::script)
                            .with_attr(typst_html::attr::src, url)
                            .spanned(declaration.span);
                        append_to_body(document, script);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Component {
    fn insert(&mut self, declaration: ResourceDeclaration) -> Result<()> {
        let slot = (declaration.kind, declaration.span);
        if let Some(&index) = self.slots.get(&slot) {
            ensure!(
                self.declarations[index] == declaration,
                "{} resource declaration in component {:?} produced different values",
                declaration.kind.name(),
                self.id.get()
            );
            return Ok(());
        }
        self.slots.insert(slot, self.declarations.len());
        self.declarations.push(declaration);
        Ok(())
    }
}

impl ResourceKind {
    fn name(self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Script => "script",
        }
    }

    fn language(self) -> &'static str {
        match self {
            Self::Style => "css",
            Self::Script => "js",
        }
    }
}

fn resource_kind(content: &Content) -> Option<ResourceKind> {
    match content.label()?.resolve().as_str() {
        "style" => Some(ResourceKind::Style),
        "script" => Some(ResourceKind::Script),
        _ => None,
    }
}

fn parse_source(kind: ResourceKind, value: &Value, declaration: Span) -> Result<ResourceSource> {
    match value {
        Value::Str(path) => {
            ensure!(!path.is_empty(), "resource path cannot be empty");
            Ok(ResourceSource::File(path.as_str().into()))
        }
        Value::Content(content) => {
            let raw = unique_raw(content)?;
            let text = raw
                .field_by_name("text")
                .map_err(|error| anyhow::anyhow!("failed to read raw text: {error}"))?;
            let Value::Str(text) = text else {
                unreachable!("RawElem text must be a string");
            };
            let language = raw.get_by_name("lang").ok().and_then(|value| match value {
                Value::Str(value) => Some(value),
                _ => None,
            });
            ensure!(
                language
                    .as_ref()
                    .is_some_and(|value| value.as_str() == kind.language()),
                "{} raw resource must use the `{}` language tag",
                kind.name(),
                kind.language()
            );
            Ok(ResourceSource::Inline {
                code: text.as_str().into(),
                origin: raw.span().id().map_or(declaration, |_| raw.span()),
            })
        }
        _ => bail!(
            "{} resource must contain a path string or raw `{}` content",
            kind.name(),
            kind.language()
        ),
    }
}

fn unique_raw(content: &Content) -> Result<&Content> {
    let mut raw = None;
    let mut unexpected = None;
    content.sequence_recursive_for_each(&mut |child| {
        if child.is::<SpaceElem>()
            || child.is::<LinebreakElem>()
            || child.is::<ParbreakElem>()
            || child.is_empty()
        {
            return;
        }
        if child.is::<RawElem>() && raw.is_none() {
            raw = Some(child);
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

fn project_source(span: Span, kind: &str) -> Result<VirtualPath> {
    let id = span
        .id()
        .with_context(|| format!("{kind} has no source file"))?;
    ensure!(
        matches!(id.root(), VirtualRoot::Project),
        "{kind} originates outside the project"
    );
    Ok(id.vpath().clone())
}

fn component_project_source(component: FileId) -> Result<VirtualPath> {
    ensure!(
        matches!(component.root(), VirtualRoot::Project),
        "component resources from packages cannot be published yet"
    );
    Ok(component.vpath().clone())
}
