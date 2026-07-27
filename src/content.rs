use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::foundations::{Content, Dict, Str, Value, dict};
use typst::introspection::MetadataElem;
use typst::utils::LazyHash;

use crate::compile;
use crate::project::ProjectRoot;

/// Discover every `.typ` file under `content/`, compile each one and return
/// the `_aster` protocol value.
///
/// Each entry's body is stored as `Value::Content(content)`.  Frontmatter
/// metadata set via `#metadata(...) <frontmatter>` is extracted into a
/// separate `metadata` dict on the entry.
pub fn load_collections(
    project: &ProjectRoot,
    builder: &compile::CompileContext,
    library: &LazyHash<Library>,
) -> Result<Value> {
    let content_dir = project.content_dir();
    let typ_files: Vec<_> = project
        .walk_content()
        .filter(|p| p.extension().is_some_and(|ext| ext == "typ"))
        .collect();

    let mut cols: BTreeMap<String, Vec<(String, PathBuf, Content)>> = BTreeMap::new();

    for path in &typ_files {
        let relative = path.strip_prefix(&content_dir).context("path error")?;

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

        let body = builder.content(path, project, library)?;
        cols.entry(collection.clone())
            .or_default()
            .push((id, path.clone(), body));
    }

    // Build the nested Dict.
    let mut collections = Dict::new();
    for (col_name, entries) in &cols {
        let mut entry = Dict::new();
        for (id, file_path, body) in entries {
            let metadata = frontmatter(body).unwrap_or_default();
            entry.insert(
                Str::from(id.as_str()),
                Value::Dict(dict! {
                    "id" => id.as_str(),
                    "collection" => col_name.as_str(),
                    "file-path" => file_path.to_string_lossy().as_ref(),
                    "body" => body.clone(),
                    "metadata" => metadata,
                }),
            );
        }
        collections.insert(Str::from(col_name.as_str()), Value::Dict(entry));
    }

    Ok(Value::Dict(dict! {
        "protocol" => 1,
        "collections" => collections,
    }))
}

fn frontmatter(content: &Content) -> Option<Dict> {
    content
        .traverse(&mut |element| {
            if element
                .label()
                .is_some_and(|l| *l.resolve() == *"frontmatter")
                && let Some(meta) = element.to_packed::<MetadataElem>()
                && let Value::Dict(dict) = &meta.value
            {
                ControlFlow::Break(dict.clone())
            } else {
                ControlFlow::Continue(())
            }
        })
        .break_value()
}
