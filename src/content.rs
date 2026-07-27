use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::foundations::{Content, Dict, Str, Value};
use typst::introspection::MetadataElem;
use typst::utils::LazyHash;

use crate::compile;

/// Discover every `.typ` file under `content/`, compile each one and return
/// the `_aster` protocol value.
///
/// Each entry's body is stored as `Value::Content(content)`.  Frontmatter
/// metadata set via `#metadata(...) <frontmatter>` is extracted into a
/// separate `metadata` dict on the entry.
pub fn load_collections(
    content_dir: &Path,
    project_root: &Path,
    builder: &compile::CompileContext,
    library: &LazyHash<Library>,
) -> Result<Value> {
    let typ_files =
        crate::project::find_typ_files(content_dir).context("failed to scan content directory")?;

    let mut cols: BTreeMap<String, Vec<(String, PathBuf, Content)>> = BTreeMap::new();

    for path in &typ_files {
        let relative = path.strip_prefix(content_dir).context("path error")?;

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

        let body = builder.content(path, project_root, library)?;
        cols.entry(collection.clone())
            .or_default()
            .push((id, path.clone(), body));
    }

    // Build the nested Dict.
    let mut collections_dict = BTreeMap::<String, Dict>::new();
    for (col_name, entries) in &cols {
        let mut entry_map = BTreeMap::<Str, Value>::new();
        for (id, file_path, body) in entries {
            let metadata = extract_frontmatter(body);
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
                (Str::from("body"), Value::Content(body.clone())),
                (Str::from("metadata"), Value::Dict(metadata)),
            ]);
            entry_map.insert(Str::from(id.as_str()), Value::Dict(entry_dict));
        }
        collections_dict.insert(col_name.clone(), Dict::from_iter(entry_map));
    }

    let mut outer = BTreeMap::<Str, Value>::new();
    for (k, v) in collections_dict {
        outer.insert(Str::from(k), Value::Dict(v));
    }

    let aster_payload = Dict::from_iter([
        (Str::from("protocol"), Value::Int(1)),
        (
            Str::from("collections"),
            Value::Dict(Dict::from_iter(outer)),
        ),
    ]);
    Ok(Value::Dict(aster_payload))
}

/// Walk the content tree looking for `#metadata(...) <frontmatter>` elements
/// and merge their values into a single dict.
fn extract_frontmatter(content: &Content) -> Dict {
    let mut merged = BTreeMap::<Str, Value>::new();
    let _ = content.traverse(&mut |element| -> ControlFlow<()> {
        if element
            .label()
            .is_some_and(|l| *l.resolve() == *"frontmatter")
            && element.is::<MetadataElem>()
            && let Some(meta) = element.to_packed::<MetadataElem>()
            && let Value::Dict(dict) = &meta.value
        {
            for (k, v) in dict.iter() {
                merged.insert(k.clone(), v.clone());
            }
        }
        ControlFlow::Continue(())
    });
    Dict::from_iter(merged)
}
