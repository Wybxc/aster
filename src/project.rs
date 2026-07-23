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
