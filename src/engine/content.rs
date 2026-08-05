//! Construction of Aster's Typst runtime protocol.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use typst::ecow::{EcoString, eco_format};
use typst::foundations::{
    Array, Capturer, Closure, ClosureNode, Dict, Func, Module, Scope, Scopes, Str, Value, dict,
};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::{RootedPath, SyntaxNode, parse_code};
use typst::{Library, LibraryExt};
use typst_eval::CapturesVisitor;

/// Runtime protocol version understood by this Aster release.
pub const PROTOCOL_VERSION: i64 = 6;
/// Version of Aster providing the runtime protocol.
pub const ASTER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Reserved `sys.inputs` key containing Aster's runtime protocol.
pub const INPUT_NAME: &str = "_aster";

/// One source entry exposed through Aster's lazy content protocol.
pub struct ContentEntry {
    /// Collection containing the entry.
    pub collection: EcoString,
    /// Entry identifier within its collection.
    pub id: EcoString,
    /// Typst source loaded by the entry's lazy accessors.
    pub source: RootedPath,
}

/// Build the base `_aster` protocol used for route probing.
pub fn protocol(entries: impl IntoIterator<Item = ContentEntry>) -> Value {
    let mut collections: BTreeMap<EcoString, Vec<(EcoString, RootedPath)>> = BTreeMap::new();
    for entry in entries {
        collections
            .entry(entry.collection)
            .or_default()
            .push((entry.id, entry.source));
    }
    protocol_value(collections)
}

/// Construct the Typst inputs owned by Aster's runtime protocol.
pub fn inputs(protocol: Value) -> Dict {
    let mut inputs = Dict::new();
    inputs.insert(Str::from(INPUT_NAME), protocol);
    inputs
}

/// Add one generated route's URL and parameters to a project input dictionary.
pub fn with_route(base: &Dict, path: EcoString, params: &crate::engine::route::ParamSet) -> Dict {
    let mut route_params = Dict::new();
    for (name, value) in params {
        route_params.insert(
            Str::from(name.as_str()),
            Value::Str(Str::from(value.as_str())),
        );
    }

    with_protocol_field(
        base,
        "route",
        Value::Dict(dict! {
            "path" => path,
            "params" => route_params,
        }),
    )
}

/// Add the complete planned page and endpoint URL sets to project inputs.
pub fn with_routes(base: &Dict, pages: &[EcoString], endpoints: &[EcoString]) -> Dict {
    let paths = |values: &[EcoString]| {
        Value::Array(
            values
                .iter()
                .map(|path| Value::Str(Str::from(path.as_str())))
                .collect::<Array>(),
        )
    };
    with_protocol_field(
        base,
        "routes",
        Value::Dict(dict! {
            "pages" => paths(pages),
            "endpoints" => paths(endpoints),
        }),
    )
}

fn with_protocol_field(base: &Dict, name: &str, value: Value) -> Dict {
    let mut inputs = base.clone();
    let Value::Dict(mut protocol) = inputs
        .get(INPUT_NAME)
        .expect("Aster inputs must contain the runtime protocol")
        .clone()
    else {
        unreachable!("Aster runtime protocol must be a dictionary");
    };
    protocol.insert(name.into(), value);
    inputs.insert(INPUT_NAME.into(), Value::Dict(protocol));
    inputs
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
            "endpoints" => Array::new(),
        },
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

    fn empty() -> Value {
        protocol(std::iter::empty())
    }

    #[test]
    fn protocol_contains_lazy_entry_modules() {
        let protocol = protocol([ContentEntry {
            collection: "blog".into(),
            id: "nested/post".into(),
            source: RootedPath::new(
                VirtualRoot::Project,
                VirtualPath::new("content/blog/nested/post.typ").unwrap(),
            ),
        }]);

        let Value::Dict(protocol) = protocol else {
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
        let Value::Dict(protocol) = empty() else {
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
        let base = with_routes(
            &inputs(empty()),
            &["/".into(), "/blog/post/".into()],
            &["/feed.xml".into()],
        );
        let params = crate::engine::route::ParamSet::from([
            ("site".into(), "route".into()),
            (INPUT_NAME.into(), "nested".into()),
        ]);
        let inputs = with_route(&base, "/blog/post/".into(), &params);

        assert_eq!(inputs.len(), 1);
        let Value::Dict(protocol) = inputs.get(INPUT_NAME).unwrap() else {
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
        let Value::Array(endpoints) = routes.get("endpoints").unwrap() else {
            panic!("planned endpoints must be an array");
        };
        assert_eq!(
            pages.iter().collect::<Vec<_>>(),
            [
                &Value::Str(Str::from("/")),
                &Value::Str(Str::from("/blog/post/"))
            ]
        );
        assert_eq!(
            endpoints.iter().collect::<Vec<_>>(),
            [&Value::Str(Str::from("/feed.xml"))]
        );
    }
}
