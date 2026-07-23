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

/// Recursively collect all `.typ` files under `dir`.
pub fn find_typ_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_typ_files(&path));
            } else if path.extension().map_or(false, |ext| ext == "typ") {
                files.push(path);
            }
        }
    }
    files
}
