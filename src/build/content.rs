//! Tracked content discovery.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use comemo::Tracked;
use typst::ecow::EcoString;
use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};

use crate::build::files::{self, ProjectFiles};
use crate::engine::content::{ContentEntry, Runtime};

/// Discover content entries through the tracked project filesystem.
pub fn load(project_files: Tracked<ProjectFiles>, root: &VirtualPath) -> Result<Runtime> {
    let entries = files::list_typst_files(project_files, root, false)?
        .into_iter()
        .map(|path| content_entry(root, path))
        .collect::<Result<Vec<_>>>()?;
    tracing::debug!(
        entries = entries.len(),
        "loaded {} content entr{}",
        entries.len(),
        if entries.len() == 1 { "y" } else { "ies" }
    );
    Ok(Runtime::new(entries))
}

fn content_entry(root: &VirtualPath, path: VirtualPath) -> Result<ContentEntry> {
    let relative = Path::new(path.get_without_slash())
        .strip_prefix(Path::new(root.get_without_slash()))
        .context("content path is outside configured content directory")?;
    if relative.components().count() < 2 {
        bail!(
            "entry {} is not inside a collection; expected content/<collection>/.../<id>.typ",
            path.get_with_slash()
        );
    }

    let mut components = relative.components();
    let collection = components
        .next()
        .map(|component| EcoString::from(component.as_os_str().to_string_lossy().as_ref()))
        .context("entry not inside a collection directory")?;
    let mut id = components.collect::<PathBuf>();
    id.set_extension("");

    Ok(ContentEntry {
        collection,
        id: EcoString::from(id.to_string_lossy().replace('\\', "/")),
        source: RootedPath::new(VirtualRoot::Project, path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_content_path_becomes_collection_and_id() {
        let root = VirtualPath::new("/content").unwrap();
        let entry = content_entry(
            &root,
            VirtualPath::new("/content/blog/guides/start.typ").unwrap(),
        )
        .unwrap();

        assert_eq!(entry.collection, "blog");
        assert_eq!(entry.id, "guides/start");
        assert_eq!(
            entry.source.vpath().get_with_slash(),
            "/content/blog/guides/start.typ"
        );
    }

    #[test]
    fn content_entry_requires_a_collection() {
        let root = VirtualPath::new("/content").unwrap();
        let error = content_entry(&root, VirtualPath::new("/content/post.typ").unwrap())
            .err()
            .expect("top-level content entry must fail");

        assert!(format!("{error:#}").contains("not inside a collection"));
    }
}
