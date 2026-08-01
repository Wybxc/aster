use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use walkdir::WalkDir;

/// A discovered Aster project with one normalized layout policy.
#[derive(Clone)]
pub struct ProjectRoot {
    root: PathBuf,
}

impl ProjectRoot {
    pub fn find(dir: &Path) -> Option<Self> {
        let mut current = Some(dir);
        while let Some(path) = current {
            if path.join("aster.toml").is_file() {
                return Some(Self {
                    root: normalize(path),
                });
            }
            current = path.parent();
        }
        None
    }

    pub fn new(root: PathBuf) -> Result<Self> {
        let root = normalize(&root);
        if !root.join("aster.toml").is_file() {
            bail!("no aster.toml found in {}", root.display());
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn src_dir(&self) -> PathBuf {
        self.root.join("src")
    }

    pub fn content_dir(&self) -> PathBuf {
        self.root.join("content")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join("dist")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("aster.toml")
    }

    /// Return every structural and tracked build input that watch mode should
    /// observe, excluding the generated output tree.
    pub fn watch_paths(&self, dependencies: &[PathBuf]) -> Vec<PathBuf> {
        let output = self.output_dir();
        let canonical_output = std::fs::canonicalize(self.root())
            .ok()
            .map(|root| root.join("dist"));
        let mut paths = self.structural_watch_paths();
        paths.extend(
            dependencies
                .iter()
                .filter(|path| !inside_output(path, &output, canonical_output.as_deref()))
                .cloned(),
        );
        paths.sort();
        paths.dedup();
        paths
    }

    fn structural_watch_paths(&self) -> Vec<PathBuf> {
        let directories = [self.src_dir(), self.content_dir()];
        let mut paths = vec![self.config_file()];
        paths.extend(directories.iter().cloned());
        for directory in directories {
            if !directory.is_dir() {
                continue;
            }
            // Builds report traversal errors; watching every reachable directory
            // lets a later filesystem change recover without exiting watch mode.
            paths.extend(
                WalkDir::new(directory)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().is_dir())
                    .map(|entry| entry.into_path()),
            );
        }
        paths.sort();
        paths.dedup();
        paths
    }
}

fn inside_output(path: &Path, output: &Path, canonical_output: Option<&Path>) -> bool {
    path.starts_with(output)
        || canonical_output.is_some_and(|canonical| path.starts_with(canonical))
}

fn normalize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_watch_paths_include_nested_and_missing_layout_directories() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src/blog/nested")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = ProjectRoot::new(root.to_owned()).unwrap();

        let paths = project.watch_paths(&[]);

        assert!(paths.contains(&project.config_file()));
        assert!(paths.contains(&project.src_dir()));
        assert!(paths.contains(&project.src_dir().join("blog")));
        assert!(paths.contains(&project.src_dir().join("blog/nested")));
        assert!(paths.contains(&project.content_dir()));
    }

    #[test]
    fn watch_paths_merge_dependencies_and_exclude_output() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("src/blog")).unwrap();
        std::fs::create_dir_all(root.join("dist")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        let project = ProjectRoot::new(root.to_owned()).unwrap();
        let theme = root.join("theme.tmTheme");
        let generated = project.output_dir().join("index.html");

        let paths = project.watch_paths(&[theme.clone(), generated.clone()]);

        assert!(paths.contains(&theme));
        assert!(paths.contains(&project.src_dir().join("blog")));
        assert!(!paths.contains(&generated));
        assert!(
            !paths
                .iter()
                .any(|path| path.starts_with(project.output_dir()))
        );
    }
}
