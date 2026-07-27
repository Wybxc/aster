use std::path::{Path, PathBuf};

/// Search upward from `dir` for an `aster.toml` file.
pub fn find_root(dir: &Path) -> Option<PathBuf> {
    let mut current = Some(dir);
    while let Some(path) = current {
        if path.join("aster.toml").exists() {
            return Some(path.to_owned());
        }
        current = path.parent();
    }
    None
}

/// Collect all `.typ` files under `dir` (iterative, depth-first).
///
/// Propagates I/O errors from individual entries.
pub fn find_typ_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_owned()];

    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "typ") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

// ---------------------------------------------------------------------------
// Project layout
// ---------------------------------------------------------------------------

/// The page templates directory (`<root>/src`).
pub fn src_dir(root: &Path) -> PathBuf {
    root.join("src")
}

/// The content collections directory (`<root>/content`).
pub fn content_dir(root: &Path) -> PathBuf {
    root.join("content")
}

/// The build output directory (`<root>/dist`).
pub fn output_dir(root: &Path) -> PathBuf {
    root.join("dist")
}

/// Compute the output HTML path for a page template.
///
/// Returns `Some(<root>/dist/<relative>.html)` when `page` is inside
/// `src_dir(root)`, or `None` if it isn't.
pub fn page_output_path(page: &Path, root: &Path) -> Option<PathBuf> {
    let src = src_dir(root);
    let relative = page.strip_prefix(&src).ok()?;
    Some(output_dir(root).join(relative).with_extension("html"))
}
