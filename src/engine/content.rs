//! Construction of Aster's Typst runtime protocol.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use typst::ecow::{EcoString, eco_format};
use typst::foundations::{
    Array, Capturer, Closure, ClosureNode, Dict, Func, Module, Scope, Scopes, Str, Value, dict,
};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::{RootedPath, SyntaxNode, parse_code};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt};
use typst_eval::CapturesVisitor;

use crate::engine::route::ParamSet;

/// Runtime protocol version understood by this Aster release.
pub const PROTOCOL_VERSION: i64 = 7;
/// Version of Aster providing the runtime protocol.
pub const ASTER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Reserved `sys.inputs` key containing Aster's runtime protocol.
pub const INPUT_NAME: &str = "_aster";

/// The complete Aster-owned Typst runtime state for one build phase.
pub struct Runtime {
    inputs: Dict,
    library: LazyHash<Library>,
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
        Self::from_inputs(inputs)
    }

    /// Add the complete planned page URL set.
    pub fn with_page_routes(&self, pages: &[EcoString]) -> Self {
        self.with_protocol_field(
            "routes",
            Value::Dict(dict! {
                "pages" => pages
                    .iter()
                    .map(|path| Value::Str(Str::from(path.as_str())))
                    .collect::<Array>(),
            }),
        )
    }

    /// Add the final rendered page snapshot available to generators.
    pub fn with_site(&self, pages: &[SitePage]) -> Self {
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
        self.with_protocol_field("site", Value::Dict(dict! { "pages" => pages }))
    }

    /// Select one route and expose its parameters to a compilation.
    pub fn for_route(&self, path: EcoString, params: &ParamSet) -> Self {
        let mut route_params = Dict::new();
        for (name, value) in params {
            route_params.insert(
                Str::from(name.as_str()),
                Value::Str(Str::from(value.as_str())),
            );
        }

        self.with_protocol_field(
            "route",
            Value::Dict(dict! {
                "path" => path,
                "params" => route_params,
            }),
        )
    }

    pub fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn with_protocol_field(&self, name: &str, value: Value) -> Self {
        let mut inputs = self.inputs.clone();
        let Value::Dict(mut protocol) = inputs
            .get(INPUT_NAME)
            .expect("Aster inputs must contain the runtime protocol")
            .clone()
        else {
            unreachable!("Aster runtime protocol must be a dictionary");
        };
        protocol.insert(name.into(), value);
        inputs.insert(INPUT_NAME.into(), Value::Dict(protocol));
        Self::from_inputs(inputs)
    }

    fn from_inputs(inputs: Dict) -> Self {
        let library = LazyHash::new(
            Library::builder()
                .with_inputs(inputs.clone())
                .with_features([Feature::Html].into_iter().collect())
                .build(),
        );
        Self { inputs, library }
    }
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
        "route" => Value::None,
        "routes" => dict! {
            "pages" => Array::new(),
        },
        "site" => dict! { "pages" => Array::new() },
    })
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
        assert!(matches!(protocol.get("route"), Ok(Value::None)));
    }

    #[test]
    fn route_context_keeps_parameters_inside_aster_protocol() {
        let base = empty().with_page_routes(&["/".into(), "/blog/post/".into()]);
        let params = crate::engine::route::ParamSet::from([
            ("site".into(), "route".into()),
            (INPUT_NAME.into(), "nested".into()),
        ]);
        let runtime = base.for_route("/blog/post/".into(), &params);

        assert_eq!(runtime.inputs.len(), 1);
        let Value::Dict(protocol) = runtime.inputs.get(INPUT_NAME).unwrap() else {
            panic!("runtime protocol must be a dictionary");
        };
        let Value::Dict(route) = protocol.get("route").unwrap() else {
            panic!("route context must be a dictionary");
        };
        assert_eq!(
            route.get("path").unwrap(),
            &Value::Str(Str::from("/blog/post/"))
        );
        let Value::Dict(params) = route.get("params").unwrap() else {
            panic!("route parameters must be a dictionary");
        };
        assert_eq!(params.get("site").unwrap(), &Value::Str(Str::from("route")));
        assert_eq!(
            params.get(INPUT_NAME).unwrap(),
            &Value::Str(Str::from("nested"))
        );
        let Value::Dict(routes) = protocol.get("routes").unwrap() else {
            panic!("planned routes must be a dictionary");
        };
        let Value::Array(pages) = routes.get("pages").unwrap() else {
            panic!("planned pages must be an array");
        };
        assert_eq!(
            pages.iter().collect::<Vec<_>>(),
            [
                &Value::Str(Str::from("/")),
                &Value::Str(Str::from("/blog/post/"))
            ]
        );
        assert!(routes.get("endpoints").is_err());
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
