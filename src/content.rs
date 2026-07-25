use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst::comemo::Track;
use typst::ecow::EcoVec;
use typst::foundations::{Array, Dict, Output, Str, Value};
use typst::model::LateLinkResolver;
use typst_html::{HtmlDocument, HtmlFrame, HtmlNode};
use typst_svg::svg_in_html;

use crate::compile;

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
) -> Result<Value> {
    let typ_files =
        crate::project::find_typ_files(content_dir).context("failed to scan content directory")?;
    // Collect rendered body values per collection.
    let mut cols: BTreeMap<String, Vec<(String, PathBuf, EcoVec<Value>)>> = BTreeMap::new();

    for path in &typ_files {
        let relative = path.strip_prefix(content_dir).context("path error")?;

        // A valid entry is at least content/<collection>/<file>.typ.
        if relative.components().count() < 2 {
            bail!(
                "entry {:?} is not inside a collection subdirectory; \
                 expected content/<collection>/.../<id>.typ",
                path.display()
            );
        }

        let mut components = relative.components();
        let collection = components
            .next()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .context("entry not inside a collection directory")?;

        let id = {
            let mut p = PathBuf::new();
            for c in components {
                p.push(c);
            }
            p.set_extension("");
            p.to_string_lossy().to_string()
        };

        let rendered = compile_content_entry(path, project_root, config_inputs.clone())?;
        cols.entry(collection.clone())
            .or_default()
            .push((id, path.clone(), rendered));
    }

    // Build the nested Dict: { "blog": { "post-1": {...}, ... }, ... }
    let mut collections_dict = BTreeMap::<String, Dict>::new();
    for (col_name, entries) in &cols {
        let mut entry_map = BTreeMap::<Str, Value>::new();
        for (id, file_path, rendered_values) in entries {
            let entry_dict = Dict::from_iter([
                (Str::from("id"), Value::Str(Str::from(id.as_str()))),
                (
                    Str::from("collection"),
                    Value::Str(Str::from(col_name.as_str())),
                ),
                (
                    Str::from("file-path"),
                    Value::Str(Str::from(file_path.to_string_lossy().as_ref())),
                ),
                (
                    Str::from("rendered"),
                    Value::Array(Array::from_iter(rendered_values.iter().cloned())),
                ),
            ]);

            entry_map.insert(Str::from(id.as_str()), Value::Dict(entry_dict));
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

/// Compile a single content entry to body DOM values.
fn compile_content_entry(entry: &Path, project_root: &Path, inputs: Dict) -> Result<EcoVec<Value>> {
    let doc = compile::compile_document(entry, project_root, inputs)?;
    body_to_values(&doc)
}

/// Extract the children of `<body>` and convert each to a Typst `Value`.
fn body_to_values(doc: &HtmlDocument) -> Result<EcoVec<Value>> {
    let root = doc.root();
    for child in &root.children {
        if let HtmlNode::Element(e) = child
            && e.tag == typst_html::tag::body
        {
            return nodes_to_values(&e.children, doc);
        }
    }
    Ok(EcoVec::new())
}

/// Render a laid-out frame to an inline data-URI SVG `<img>` element value.
///
/// Uses the document's introspector to resolve any late links inside the
/// frame (typically there are none for content entries).
fn frame_to_img(frame: &HtmlFrame, doc: &HtmlDocument) -> Value {
    let introspector = Output::introspector(doc);
    let resolver = LateLinkResolver::new(None, introspector);
    let svg = svg_in_html(
        &frame.inner,
        frame.text_size,
        false,
        frame.id.as_deref(),
        "",
        &frame.anchors,
        resolver.track(),
    );

    // Build a data URI for the SVG.
    let mut src = String::from("data:image/svg+xml,");
    for b in svg.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'.' | b'_' | b'~'
            | b'/' | b':' | b';' | b'='
            | b'?' | b'&' | b'!' | b'@'
            | b'$' | b'+' | b',' => src.push(b as char),
            b' ' => src.push_str("%20"),
            b'#' => src.push_str("%23"),
            b'%' => src.push_str("%25"),
            b'<' => src.push_str("%3C"),
            b'>' => src.push_str("%3E"),
            b'"' => src.push_str("%22"),
            b'\'' => src.push_str("%27"),
            b'(' => src.push_str("%28"),
            b')' => src.push_str("%29"),
            _ => {
                let _ = write!(src, "%{b:02X}");
            }
        }
    }

    Value::Dict(Dict::from_iter([
        (Str::from("kind"), Value::Str(Str::from("element"))),
        (Str::from("tag"), Value::Str(Str::from("img"))),
        (
            Str::from("attrs"),
            Value::Dict(Dict::from_iter([
                (Str::from("src"), Value::Str(Str::from(src))),
                (Str::from("alt"), Value::Str(Str::from(""))),
            ])),
        ),
        (Str::from("children"), Value::Array(Array::new())),
        (Str::from("void"), Value::Bool(true)),
    ]))
}

/// Convert a slice of `HtmlNode`s to `Vec<Value>` (recursive).
///
/// - `Element` → `{kind: "element", tag, attrs, children, void}`
/// - `Text` → `{kind: "text", value}`
/// - `Frame` → inline `<img>` with SVG data URI
/// - `Tag` → silently skipped
fn nodes_to_values(nodes: &[HtmlNode], doc: &HtmlDocument) -> Result<EcoVec<Value>> {
    let mut out = EcoVec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            HtmlNode::Element(elem) => {
                let tag = elem.tag.resolve();
                let attrs = Dict::from_iter(elem.attrs.0.iter().map(|(k, v)| {
                    let k = k.resolve();
                    let v = v.clone();
                    (Str::from(k.as_str()), Value::Str(v.into()))
                }));
                let children = nodes_to_values(&elem.children, doc)?;
                let void = typst_html::tag::is_void(elem.tag);

                out.push(Value::Dict(Dict::from_iter([
                    (Str::from("kind"), Value::Str(Str::from("element"))),
                    (Str::from("tag"), Value::Str(Str::from(tag.as_str()))),
                    (Str::from("attrs"), Value::Dict(attrs)),
                    (
                        Str::from("children"),
                        Value::Array(Array::from(children)),
                    ),
                    (Str::from("void"), Value::Bool(void)),
                ])));
            }
            HtmlNode::Text(text, _) => {
                out.push(Value::Dict(Dict::from_iter([
                    (Str::from("kind"), Value::Str(Str::from("text"))),
                    (Str::from("value"), Value::Str(Str::from(text.as_str()))),
                ])));
            }
            HtmlNode::Frame(frame) => {
                out.push(frame_to_img(frame, doc));
            }
            // Introspection tags carry no DOM — skip silently.
            HtmlNode::Tag(_) => {}
        }
    }
    Ok(out)
}
