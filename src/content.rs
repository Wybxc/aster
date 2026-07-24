use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use typst::foundations::{Array, Dict, Str, Value};
use typst_html::{HtmlDocument, HtmlNode};

use crate::compile;

// ---------------------------------------------------------------------------
// Intermediate representation of HTML Content nodes
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum ContentNode {
    Text {
        value: String,
    },
    Element {
        tag: String,
        attrs: BTreeMap<String, String>,
        children: Vec<ContentNode>,
        void: bool,
    },
}

impl ContentNode {
    fn from_body(doc: &HtmlDocument) -> Result<Vec<ContentNode>, String> {
        let root = doc.root();
        for child in &root.children {
            if let HtmlNode::Element(e) = child
                && e.tag == typst_html::tag::body
            {
                return Self::collect(&e.children);
            }
        }
        Ok(Vec::new())
    }

    fn from_node(node: &HtmlNode) -> Result<Option<ContentNode>, String> {
        match node {
            HtmlNode::Element(elem) => Self::from_element(elem).map(Some),
            HtmlNode::Text(text, _) => Ok(Some(ContentNode::Text {
                value: text.to_string(),
            })),
            HtmlNode::Frame(_) => Err(
                "frame-based content is not supported in content collections; \
                 avoid html.frame() in collection entries"
                    .to_owned(),
            ),
            // Introspection tags carry no DOM — skip silently.
            HtmlNode::Tag(_) => Ok(None),
        }
    }

    fn collect(nodes: &[HtmlNode]) -> Result<Vec<ContentNode>, String> {
        let mut out = Vec::new();
        for node in nodes {
            if let Some(n) = Self::from_node(node)? {
                out.push(n);
            }
        }
        Ok(out)
    }

    fn from_element(elem: &typst_html::HtmlElement) -> Result<ContentNode, String> {
        let mut attrs = BTreeMap::new();
        for (attr, value) in &elem.attrs.0 {
            attrs.insert(attr.resolve().to_string(), value.to_string());
        }

        let void = typst_html::tag::is_void(elem.tag);
        let children = Self::collect(&elem.children)?;

        Ok(ContentNode::Element {
            tag: elem.tag.resolve().to_string(),
            attrs,
            children,
            void,
        })
    }

    pub fn into_value(self) -> Value {
        match self {
            ContentNode::Text { value } => Value::Dict(Dict::from_iter([
                (Str::from("kind"), Value::Str(Str::from("text"))),
                (Str::from("value"), Value::Str(Str::from(value))),
            ])),
            ContentNode::Element {
                tag,
                attrs,
                children,
                void,
            } => {
                let attrs_dict = Value::Dict(Dict::from_iter(
                    attrs
                        .into_iter()
                        .map(|(k, v)| (Str::from(k), Value::Str(Str::from(v)))),
                ));

                let children_arr = Value::Array(Array::from_iter(
                    children.into_iter().map(|c| c.into_value()),
                ));

                Value::Dict(Dict::from_iter([
                    (Str::from("kind"), Value::Str(Str::from("element"))),
                    (Str::from("tag"), Value::Str(Str::from(tag))),
                    (Str::from("attrs"), attrs_dict),
                    (Str::from("children"), children_arr),
                    (Str::from("void"), Value::Bool(void)),
                ]))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-entry result
// ---------------------------------------------------------------------------

pub struct ContentEntry {
    collection: String,
    id: String,
    file_path: PathBuf,
    rendered: Vec<ContentNode>,
}

// ---------------------------------------------------------------------------
// Top-level: discover & compile all entries → collections Dict
// ---------------------------------------------------------------------------

/// Discover every `.typ` file under `content/`, compile each one as an HTML
/// document, extract the body DOM, and return the complete `_aster` protocol
/// value (including protocol version and collections).
///
/// The returned `Value` is a `Dict` with keys `protocol` and `collections`,
/// suitable for `sys.inputs._aster`.
pub fn load_collections(
    content_dir: &Path,
    project_root: &Path,
    config_inputs: Dict,
) -> Result<Value, String> {
    let typ_files = crate::project::find_typ_files(content_dir)
        .map_err(|e| format!("failed to scan content directory: {e}"))?;
    // Collect entries per collection.
    let mut cols: BTreeMap<String, Vec<ContentEntry>> = BTreeMap::new();

    for path in &typ_files {
        let relative = path.strip_prefix(content_dir).map_err(|_| "path error")?;

        // A valid entry is at least content/<collection>/<file>.typ.
        if relative.components().count() < 2 {
            return Err(format!(
                "entry {:?} is not inside a collection subdirectory; \
                 expected content/<collection>/.../<id>.typ",
                path.display()
            ));
        }

        let mut components = relative.components();
        let collection = components
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .ok_or_else(|| "entry not inside a collection directory".to_owned())?;

        let id = {
            let mut p = PathBuf::new();
            for c in components {
                p.push(c);
            }
            p.set_extension("");
            p.to_string_lossy().to_string()
        };

        let content_nodes = compile_content_entry(path, project_root, config_inputs.clone())?;
        cols.entry(collection.clone())
            .or_default()
            .push(ContentEntry {
                collection,
                id: id.clone(),
                file_path: path.clone(),
                rendered: content_nodes,
            });
    }

    // Build the nested Dict: { "blog": { "post-1": {...}, ... }, ... }
    let mut collections_dict = BTreeMap::<String, Dict>::new();
    for (col_name, entries) in &cols {
        let mut entry_map = BTreeMap::<Str, Value>::new();
        for entry in entries {
            let rendered =
                Array::from_iter(entry.rendered.clone().into_iter().map(|n| n.into_value()));

            let entry_dict = Dict::from_iter([
                (Str::from("id"), Value::Str(Str::from(entry.id.as_str()))),
                (
                    Str::from("collection"),
                    Value::Str(Str::from(entry.collection.as_str())),
                ),
                (
                    Str::from("file-path"),
                    Value::Str(Str::from(entry.file_path.to_string_lossy().as_ref())),
                ),
                (Str::from("rendered"), Value::Array(rendered)),
            ]);

            entry_map.insert(Str::from(entry.id.as_str()), Value::Dict(entry_dict));
        }
        collections_dict.insert(col_name.clone(), Dict::from_iter(entry_map));
    }

    let mut outer = BTreeMap::<Str, Value>::new();
    for (k, v) in collections_dict {
        outer.insert(Str::from(k), Value::Dict(v));
    }

    // Build the _aster protocol envelope.
    let aster_payload = Dict::from_iter([
        (Str::from("protocol"), Value::Int(1)),
        (
            Str::from("collections"),
            Value::Dict(Dict::from_iter(outer)),
        ),
    ]);
    Ok(Value::Dict(aster_payload))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compile a single content entry to body content nodes.
fn compile_content_entry(
    entry: &Path,
    project_root: &Path,
    inputs: Dict,
) -> Result<Vec<ContentNode>, String> {
    let doc: HtmlDocument =
        compile::compile_document(entry, project_root, inputs).map_err(|e| format!("{e:#}"))?;
    ContentNode::from_body(&doc)
}
