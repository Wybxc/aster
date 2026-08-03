use std::collections::BTreeMap;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use typst::ecow::{EcoString, eco_format};
use typst::foundations::{
    Capturer, Closure, ClosureNode, Dict, Func, Module, Scope, Scopes, Str, Value, dict,
};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::{RootedPath, SyntaxNode, parse_code};
use typst::{Library, LibraryExt};
use typst_eval::CapturesVisitor;

pub const PROTOCOL_VERSION: i64 = 4;
pub const INPUT_NAME: &str = "_aster";

pub(crate) struct ContentEntry {
    pub collection: EcoString,
    pub id: EcoString,
    pub source: RootedPath,
}

/// Build the `_aster` lazy entry manifest, including the empty state.
pub(crate) fn protocol(entries: impl IntoIterator<Item = ContentEntry>) -> Value {
    let mut collections: BTreeMap<EcoString, Vec<(EcoString, RootedPath)>> = BTreeMap::new();
    for entry in entries {
        collections
            .entry(entry.collection)
            .or_default()
            .push((entry.id, entry.source));
    }
    protocol_value(collections)
}

pub fn with_protocol(config: Dict, protocol: Value) -> Result<Dict> {
    if config.contains(INPUT_NAME) {
        bail!("`{INPUT_NAME}` is reserved for Aster's content protocol");
    }
    let mut inputs = config;
    inputs.insert(Str::from(INPUT_NAME), protocol);
    Ok(inputs)
}

pub fn with_route_params(base: &Dict, params: &crate::engine::route::ParamSet) -> Result<Dict> {
    let mut inputs = base.clone();
    for (name, value) in params {
        if name.as_str() == INPUT_NAME {
            bail!("route parameter `{INPUT_NAME}` is reserved");
        }
        if inputs.contains(name.as_str()) {
            bail!("route parameter `{name}` conflicts with configuration input");
        }
        inputs.insert(
            Str::from(name.as_str()),
            Value::Str(Str::from(value.as_str())),
        );
    }
    Ok(inputs)
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
        "collections" => packed_collections,
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
    if fields.at("label", default: none) == <frontmatter> {
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
    }

    #[test]
    fn rejects_reserved_inputs() {
        let mut config = Dict::new();
        config.insert(Str::from(INPUT_NAME), Value::Int(0));
        assert!(with_protocol(config, empty()).is_err());

        let base = with_protocol(Dict::new(), empty()).unwrap();
        assert!(
            with_route_params(
                &base,
                &crate::engine::route::ParamSet::from([(INPUT_NAME.into(), "bad".into())]),
            )
            .is_err()
        );

        let mut configured = base;
        configured.insert(Str::from("site"), Value::Str(Str::from("Aster")));
        assert!(
            with_route_params(
                &configured,
                &crate::engine::route::ParamSet::from([("site".into(), "other".into())]),
            )
            .is_err()
        );
    }
}
