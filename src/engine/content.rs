use std::collections::BTreeMap;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use typst::ecow::EcoString;
use typst::foundations::{
    Capturer, Closure, ClosureNode, Dict, Func, Module, Scope, Scopes, Str, Value, dict,
};
use typst::syntax::ast::{self, AstNode};
use typst::syntax::{RootedPath, SyntaxNode, parse_code};
use typst::{Library, LibraryExt};
use typst_eval::CapturesVisitor;

pub const PROTOCOL_VERSION: i64 = 3;
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

#[cfg(test)]
pub fn empty() -> Value {
    protocol(std::iter::empty())
}

pub fn install(config: Dict, protocol: Value) -> Result<Dict> {
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
    scope.define("render", render_closure(source));
    Module::new(format!("{collection}/{id}"), scope)
}

fn render_closure(source: RootedPath) -> Func {
    const RENDER_CLOSURE_SOURCE: &str = r#"() => {
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
  (
    metadata: if metadata == none { (:) } else { metadata },
    content: entry-content,
  )
}"#;

    static RENDER_CLOSURE_NODE: LazyLock<SyntaxNode> = LazyLock::new(|| {
        let root = parse_code(RENDER_CLOSURE_SOURCE);
        let (errors, _) = root.errors_and_warnings();
        assert!(errors.is_empty(), "Aster render closure must parse");
        let code = root
            .cast::<ast::Code>()
            .expect("render closure must be code");
        let mut expressions = code.exprs();
        let ast::Expr::Closure(closure) = expressions.next().expect("render closure must exist")
        else {
            panic!("render expression must be a closure");
        };
        assert!(
            expressions.next().is_none(),
            "render closure must be the only expression"
        );
        closure.to_untyped().clone()
    });

    static CAPTURE_LIBRARY: LazyLock<Library> = LazyLock::new(Library::default);

    let mut scopes = Scopes::new(Some(&CAPTURE_LIBRARY));
    scopes.top.define("source", source);

    let mut captures = CapturesVisitor::new(Some(&scopes), Capturer::Function);
    captures.visit(&RENDER_CLOSURE_NODE);
    Func::from(Closure {
        node: ClosureNode::Closure(RENDER_CLOSURE_NODE.clone()),
        defaults: Vec::new(),
        captured: captures.finish(),
        num_pos_params: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use typst::syntax::{VirtualPath, VirtualRoot};

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
    fn rejects_reserved_config_input() {
        let mut config = Dict::new();
        config.insert(Str::from(INPUT_NAME), Value::Int(0));
        assert!(install(config, empty()).is_err());
    }

    #[test]
    fn route_params_cannot_replace_internal_or_config_inputs() {
        let base = install(Dict::new(), empty()).unwrap();
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
