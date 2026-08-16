//! Construction of Aster's Typst runtime protocol.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use typst::diag::{At, FileError, FileResult, SourceResult};
use typst::ecow::{EcoString, eco_format};
use typst::engine::Engine;
use typst::foundations::{
    Array, Bytes, Capturer, Closure, ClosureNode, Dict, Func, Module, Scope, Scopes, Str, Value,
    dict, func,
};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Span, SyntaxNode, VirtualPath, VirtualRoot, parse_code};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_eval::CapturesVisitor;

use crate::engine::route::ParamSet;

/// Runtime protocol version understood by this Aster release.
pub const PROTOCOL_VERSION: i64 = 9;
/// Version of Aster providing the runtime protocol.
pub const ASTER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Reserved `sys.inputs` key containing Aster's runtime protocol.
pub const INPUT_NAME: &str = "_aster";

/// The complete Aster-owned Typst runtime state for one build phase.
pub struct Runtime {
    inputs: Dict,
    library: LazyHash<Library>,
    page_routes: PageRoutes,
}

impl Runtime {
    /// Construct the base runtime used for route planning.
    pub fn new(entries: impl IntoIterator<Item = ContentEntry>) -> Self {
        let mut collections: BTreeMap<EcoString, Vec<(EcoString, RootedPath)>> = BTreeMap::new();
        for entry in entries {
            collections
                .entry(entry.collection)
                .or_default()
                .push((entry.id, entry.source));
        }

        let mut inputs = Dict::new();
        inputs.insert(Str::from(INPUT_NAME), protocol_value(collections));
        let library = library(&inputs);
        Self {
            inputs,
            library,
            page_routes: PageRoutes::default(),
        }
    }

    /// Supply the complete planned page URL set without changing the library.
    pub fn with_page_routes(mut self, pages: Vec<EcoString>) -> Self {
        self.page_routes = PageRoutes::new(pages);
        self
    }

    /// Add the final rendered page snapshot available to generators.
    pub fn with_site(mut self, pages: &[SitePage]) -> Self {
        let pages = pages
            .iter()
            .map(|page| {
                let content = page.content.as_ref().map_or(Value::None, |content| {
                    Value::Dict(dict! {
                        "html" => content.html.clone(),
                        "text" => content.text.clone(),
                    })
                });
                Value::Dict(dict! {
                    "path" => page.path.clone(),
                    "html" => page.html.clone(),
                    "content" => content,
                })
            })
            .collect::<Array>();
        let Value::Dict(mut protocol) = self
            .inputs
            .get(INPUT_NAME)
            .expect("Aster inputs must contain the runtime protocol")
            .clone()
        else {
            unreachable!("Aster runtime protocol must be a dictionary");
        };
        protocol.insert("site".into(), Value::Dict(dict! { "pages" => pages }));
        self.inputs.insert(INPUT_NAME.into(), Value::Dict(protocol));
        self.library = library(&self.inputs);
        self
    }

    /// Construct the context used to probe a dynamic template.
    pub fn probe(&self) -> CompilationRuntime<'_> {
        CompilationRuntime {
            runtime: self,
            route: RouteContext::default(),
        }
    }

    /// Construct one route context that borrows this runtime's shared library.
    pub fn for_route(&self, path: EcoString, params: &ParamSet) -> CompilationRuntime<'_> {
        CompilationRuntime {
            runtime: self,
            route: RouteContext::new(path, params),
        }
    }
}

/// Dynamic Aster values supplied to one compilation over a shared [`Library`].
pub struct CompilationRuntime<'a> {
    runtime: &'a Runtime,
    route: RouteContext,
}

impl CompilationRuntime<'_> {
    pub fn library(&self) -> &LazyHash<Library> {
        &self.runtime.library
    }

    /// Resolve an Aster-owned virtual file, or return `None` for other files.
    pub fn resolve_file(&self, id: FileId) -> Option<FileResult<Bytes>> {
        if !owns_runtime_file(id) {
            return None;
        }

        let value = self
            .route
            .value(id)
            .or_else(|| self.runtime.page_routes.value(id));
        Some(
            value
                .cloned()
                .map(Str::from)
                .map(Bytes::from_string)
                .ok_or_else(|| FileError::NotFound(id.vpath().get_with_slash().into())),
        )
    }
}

#[derive(Default)]
struct RouteContext {
    path: Option<EcoString>,
    params: ParamSet,
}

impl RouteContext {
    fn new(path: EcoString, params: &ParamSet) -> Self {
        Self {
            path: Some(path),
            params: params.clone(),
        }
    }

    fn path_id() -> FileId {
        runtime_file_id("route/path")
    }

    fn param_id(name: &str) -> FileId {
        runtime_file_id(&format!("route/params/{}", hex::encode(name)))
    }

    fn value(&self, id: FileId) -> Option<&EcoString> {
        if id == Self::path_id() {
            return self.path.as_ref();
        }

        let encoded = id
            .vpath()
            .get_without_slash()
            .strip_prefix("route/params/")?;
        let name = hex::decode(encoded).ok()?;
        self.params.get(std::str::from_utf8(&name).ok()?)
    }
}

#[derive(Default)]
struct PageRoutes(Vec<EcoString>);

impl PageRoutes {
    fn new(pages: Vec<EcoString>) -> Self {
        Self(pages)
    }

    fn id(index: usize) -> FileId {
        runtime_file_id(&format!("routes/pages/{index}"))
    }

    fn value(&self, id: FileId) -> Option<&EcoString> {
        let index = id
            .vpath()
            .get_without_slash()
            .strip_prefix("routes/pages/")?
            .parse::<usize>()
            .ok()?;
        self.0.get(index)
    }
}

fn owns_runtime_file(id: FileId) -> bool {
    matches!(id.root(), VirtualRoot::Package(package) if package == runtime_package())
}

fn runtime_file_id(path: &str) -> FileId {
    RootedPath::new(
        VirtualRoot::Package(runtime_package().clone()),
        VirtualPath::new(path).expect("Aster runtime path must be valid"),
    )
    .intern()
}

fn runtime_package() -> &'static PackageSpec {
    static PACKAGE: LazyLock<PackageSpec> = LazyLock::new(|| {
        "@aster/runtime:0.1.0"
            .parse()
            .expect("Aster runtime package specification must be valid")
    });
    &PACKAGE
}

fn library(inputs: &Dict) -> LazyHash<Library> {
    LazyHash::new(
        Library::builder()
            .with_inputs(inputs.clone())
            .with_features([Feature::Html].into_iter().collect())
            .build(),
    )
}

/// One source entry exposed through Aster's lazy content protocol.
pub struct ContentEntry {
    /// Collection containing the entry.
    pub collection: EcoString,
    /// Entry identifier within its collection.
    pub id: EcoString,
    /// Typst source loaded by the entry's lazy accessors.
    pub source: RootedPath,
}

/// One rendered page exposed to Typst generators.
pub struct SitePage {
    /// Browser-facing route of the page.
    pub path: EcoString,
    /// Complete final HTML document.
    pub html: EcoString,
    /// Optional labelled main-content fragment.
    pub content: Option<SiteContent>,
}

/// The final HTML and plain text of a page's `<aster-content>` element.
pub struct SiteContent {
    pub html: EcoString,
    pub text: EcoString,
}

fn protocol_value(collections: BTreeMap<EcoString, Vec<(EcoString, RootedPath)>>) -> Value {
    let mut packed_collections = Dict::new();
    for (collection_name, entries) in collections {
        let mut packed_entries = Dict::new();
        for (id, source) in entries {
            packed_entries.insert(
                Str::from(id.as_str()),
                Value::Module(entry_module(&collection_name, &id, source)),
            );
        }
        packed_collections.insert(
            Str::from(collection_name.as_str()),
            Value::Dict(packed_entries),
        );
    }

    Value::Dict(dict! {
        "protocol" => PROTOCOL_VERSION,
        "version" => ASTER_VERSION,
        "collections" => packed_collections,
        "route" => route_module(),
        "routes" => routes_module(),
        "site" => dict! { "pages" => Array::new() },
    })
}

fn route_module() -> Module {
    let mut scope = Scope::new();
    scope.define_func::<path>();
    scope.define_func::<param>();
    Module::new("route", scope)
}

fn routes_module() -> Module {
    let mut scope = Scope::new();
    scope.define_func::<pages>();
    Module::new("routes", scope)
}

fn read(engine: &mut Engine, id: FileId, span: Span) -> SourceResult<Option<Str>> {
    match engine.world.file(id) {
        Ok(bytes) => bytes.to_str().map(Some).map_err(FileError::from).at(span),
        Err(FileError::NotFound(_)) => Ok(None),
        Err(error) => Err(error).at(span),
    }
}

fn route_value(
    engine: &mut Engine,
    id: FileId,
    default: Option<Value>,
    span: Span,
) -> SourceResult<Value> {
    Ok(read(engine, id, span)?
        .map(Value::Str)
        .unwrap_or_else(|| default.unwrap_or(Value::None)))
}

/// Return the current browser-facing route path.
#[func]
fn path(
    engine: &mut Engine,
    span: Span,
    /// A value returned while no concrete route is being compiled.
    #[named]
    default: Option<Value>,
) -> SourceResult<Value> {
    route_value(engine, RouteContext::path_id(), default, span)
}

/// Return one parameter of the current route.
#[func]
fn param(
    engine: &mut Engine,
    span: Span,
    /// The route parameter name.
    name: Str,
    /// A value returned when the parameter is absent.
    #[named]
    default: Option<Value>,
) -> SourceResult<Value> {
    route_value(engine, RouteContext::param_id(name.as_str()), default, span)
}

/// Return all planned page browser paths in deterministic order.
#[func]
fn pages(engine: &mut Engine, span: Span) -> SourceResult<Array> {
    let mut pages = Array::new();
    for index in 0.. {
        let Some(path) = read(engine, PageRoutes::id(index), span)? else {
            break;
        };
        pages.push(Value::Str(path));
    }
    Ok(pages)
}

fn entry_module(collection: &EcoString, id: &EcoString, source: RootedPath) -> Module {
    let mut scope = Scope::new();
    scope.define("id", Str::from(id.as_str()));
    scope.define("collection", Str::from(collection.as_str()));
    scope.define("metadata", metadata_closure(source.clone()));
    scope.define("render", render_closure(source));
    Module::new(eco_format!("{collection}/{id}"), scope)
}

fn metadata_closure(source: RootedPath) -> Func {
    const METADATA_CLOSURE_SOURCE: &str = r#"() => {
  let find-frontmatter(node) = {
    let fields = node.fields()
    if fields.at("label", default: none) == <aster-frontmatter> {
      let value = fields.at("value", default: (:))
      return if type(value) == dictionary { value } else { (:) }
    }

    for value in fields.values() {
      if type(value) == content {
        let found = find-frontmatter(value)
        if found != none { return found }
      } else if type(value) == array {
        for child in value {
          if type(child) == content {
            let found = find-frontmatter(child)
            if found != none { return found }
          }
        }
      }
    }

    none
  }

  import source as entry-module
  let entry-content = include entry-module
  let metadata = find-frontmatter(entry-content)
  if metadata == none { (:) } else { metadata }
}"#;

    static METADATA_CLOSURE_NODE: LazyLock<SyntaxNode> =
        LazyLock::new(|| parse_closure(METADATA_CLOSURE_SOURCE));

    captured_closure(METADATA_CLOSURE_NODE.clone(), source)
}

fn render_closure(source: RootedPath) -> Func {
    const RENDER_CLOSURE_SOURCE: &str = r#"() => {
  import source as entry-module
  include entry-module
}"#;

    static RENDER_CLOSURE_NODE: LazyLock<SyntaxNode> =
        LazyLock::new(|| parse_closure(RENDER_CLOSURE_SOURCE));

    captured_closure(RENDER_CLOSURE_NODE.clone(), source)
}

fn parse_closure(source: &str) -> SyntaxNode {
    let root = parse_code(source);
    let (errors, _) = root.errors_and_warnings();
    assert!(errors.is_empty(), "Aster entry closure must parse");
    let code = root
        .cast::<ast::Code>()
        .expect("entry closure must be code");
    let mut expressions = code.exprs();
    let ast::Expr::Closure(closure) = expressions.next().expect("entry closure must exist") else {
        panic!("entry expression must be a closure");
    };
    assert!(
        expressions.next().is_none(),
        "entry closure must be the only expression"
    );
    closure.to_untyped().clone()
}

fn captured_closure(node: SyntaxNode, source: RootedPath) -> Func {
    static CAPTURE_LIBRARY: LazyLock<Library> = LazyLock::new(Library::default);

    let mut scopes = Scopes::new(Some(&CAPTURE_LIBRARY));
    scopes.top.define("source", source);

    let mut captures = CapturesVisitor::new(Some(&scopes), Capturer::Function);
    captures.visit(&node);
    Func::from(Closure {
        node: ClosureNode::Closure(node),
        defaults: Vec::new(),
        captured: captures.finish(),
        num_pos_params: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use typst::syntax::{VirtualPath, VirtualRoot};

    fn empty() -> Runtime {
        Runtime::new(std::iter::empty())
    }

    #[test]
    fn protocol_contains_lazy_entry_modules() {
        let runtime = Runtime::new([ContentEntry {
            collection: "blog".into(),
            id: "nested/post".into(),
            source: RootedPath::new(
                VirtualRoot::Project,
                VirtualPath::new("content/blog/nested/post.typ").unwrap(),
            ),
        }]);

        let Value::Dict(protocol) = runtime.inputs.get(INPUT_NAME).unwrap() else {
            panic!("protocol must be a dictionary");
        };
        let Value::Dict(collections) = protocol.get("collections").unwrap() else {
            panic!("collections must be a dictionary");
        };
        let Value::Dict(blog) = collections.get("blog").unwrap() else {
            panic!("collection must be a dictionary");
        };
        let Value::Module(entry) = blog.get("nested/post").unwrap() else {
            panic!("entry must be a module");
        };
        assert_eq!(
            entry.field("id", ()).unwrap(),
            &Value::Str(Str::from("nested/post"))
        );
        assert_eq!(
            entry.field("collection", ()).unwrap(),
            &Value::Str(Str::from("blog"))
        );
        assert!(matches!(
            entry.field("metadata", ()).unwrap(),
            Value::Func(_)
        ));
        assert!(matches!(entry.field("render", ()).unwrap(), Value::Func(_)));
        assert!(entry.field("file-path", ()).is_err());
        assert!(entry.field("content", ()).is_err());
    }

    #[test]
    fn empty_protocol_has_one_owner() {
        let runtime = empty();
        let Value::Dict(protocol) = runtime.inputs.get(INPUT_NAME).unwrap() else {
            panic!("protocol must be a dictionary");
        };
        assert_eq!(
            protocol.get("protocol").unwrap(),
            &Value::Int(PROTOCOL_VERSION)
        );
        assert_eq!(
            protocol.get("collections").unwrap(),
            &Value::Dict(Dict::new())
        );
        assert_eq!(
            protocol.get("version").unwrap(),
            &Value::Str(Str::from(ASTER_VERSION))
        );
        let Value::Module(route) = protocol.get("route").unwrap() else {
            panic!("route protocol must be a module");
        };
        assert!(matches!(route.field("path", ()).unwrap(), Value::Func(_)));
        assert!(matches!(route.field("param", ()).unwrap(), Value::Func(_)));
    }

    #[test]
    fn compilation_runtime_resolves_virtual_files_without_changing_the_protocol() {
        let base = empty();
        let library = base.library.clone();
        let base = base.with_page_routes(vec!["/".into(), "/blog/post/".into()]);
        assert!(base.library == library);
        let params = crate::engine::route::ParamSet::from([
            ("site".into(), "route".into()),
            (INPUT_NAME.into(), "nested".into()),
            ("..".into(), "parent".into()),
        ]);
        let runtime = base.for_route("/blog/post/".into(), &params);

        assert_eq!(base.inputs.len(), 1);
        let Value::Dict(protocol) = base.inputs.get(INPUT_NAME).unwrap() else {
            panic!("runtime protocol must be a dictionary");
        };
        let Value::Module(route_module) = protocol.get("route").unwrap() else {
            panic!("route protocol must be a module");
        };
        assert_eq!(
            runtime
                .resolve_file(RouteContext::path_id())
                .unwrap()
                .unwrap()
                .to_str()
                .unwrap(),
            Str::from("/blog/post/")
        );
        assert_eq!(
            runtime
                .resolve_file(RouteContext::param_id("site"))
                .unwrap()
                .unwrap()
                .to_str()
                .unwrap(),
            Str::from("route")
        );
        assert_eq!(
            runtime
                .resolve_file(RouteContext::param_id(INPUT_NAME))
                .unwrap()
                .unwrap()
                .to_str()
                .unwrap(),
            Str::from("nested")
        );
        assert_eq!(
            runtime
                .resolve_file(RouteContext::param_id(".."))
                .unwrap()
                .unwrap()
                .to_str()
                .unwrap(),
            Str::from("parent")
        );
        assert!(
            runtime
                .resolve_file(RouteContext::param_id("missing"))
                .unwrap()
                .is_err()
        );
        assert!(matches!(
            route_module.field("path", ()).unwrap(),
            Value::Func(_)
        ));
        let Value::Module(routes) = protocol.get("routes").unwrap() else {
            panic!("planned routes must be a module");
        };
        assert!(matches!(routes.field("pages", ()).unwrap(), Value::Func(_)));
        assert_eq!(
            runtime
                .resolve_file(PageRoutes::id(0))
                .unwrap()
                .unwrap()
                .to_str()
                .unwrap(),
            Str::from("/")
        );
        assert_eq!(
            runtime
                .resolve_file(PageRoutes::id(1))
                .unwrap()
                .unwrap()
                .to_str()
                .unwrap(),
            Str::from("/blog/post/")
        );
        assert!(runtime.resolve_file(PageRoutes::id(2)).unwrap().is_err());
        assert!(routes.field("endpoints", ()).is_err());
    }

    #[test]
    fn site_snapshot_exposes_final_page_content() {
        let runtime = empty().with_site(&[SitePage {
            path: "/post/".into(),
            html: "<html>...</html>".into(),
            content: Some(SiteContent {
                html: "<article>Post</article>".into(),
                text: "Post".into(),
            }),
        }]);
        let Value::Dict(protocol) = runtime.inputs.get(INPUT_NAME).unwrap() else {
            panic!("runtime protocol must be a dictionary");
        };
        let Value::Dict(site) = protocol.get("site").unwrap() else {
            panic!("site snapshot must be a dictionary");
        };
        let Value::Array(pages) = site.get("pages").unwrap() else {
            panic!("site pages must be an array");
        };
        assert_eq!(pages.len(), 1);
    }
}
