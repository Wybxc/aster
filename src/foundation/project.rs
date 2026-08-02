use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use typst::syntax::VirtualPath;
use walkdir::WalkDir;

use crate::foundation::config::AsterConfig;

/// A discovered Aster project rooted at an absolute lexical path.
#[derive(Clone)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    /// Find the nearest project at or above `dir`.
    pub fn find(dir: &Path) -> Option<Self> {
        let dir = std::path::absolute(dir).ok()?;
        let mut current = Some(dir.as_path());
        while let Some(path) = current {
            if path.join("aster.toml").is_file() {
                return Some(Self {
                    root: path.to_owned(),
                });
            }
            current = path.parent();
        }
        None
    }

    /// Open a directory containing an `aster.toml` project manifest.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root =
            std::path::absolute(root.into()).context("failed to make project root absolute")?;
        if !root.join("aster.toml").is_file() {
            bail!("no aster.toml found in {}", root.display());
        }
        Ok(Self { root })
    }

    /// Return the absolute project root without resolving symbolic links.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the project manifest path.
    pub fn config_file(&self) -> PathBuf {
        self.root.join("aster.toml")
    }

    pub(crate) fn realize(&self, path: &VirtualPath) -> PathBuf {
        path.realize(&self.root)
            .expect("validated project path must realize within the project root")
    }
}

/// Validated project-relative paths used by one build configuration.
#[derive(Clone)]
pub(crate) struct ProjectLayout {
    source: VirtualPath,
    content: VirtualPath,
    output: VirtualPath,
    assets: VirtualPath,
    fonts: Vec<VirtualPath>,
}

impl ProjectLayout {
    pub(crate) fn new(config: &AsterConfig) -> Result<Self> {
        let source = project_directory(&config.paths.source, "source")?;
        let content = project_directory(&config.paths.content, "content")?;
        let output = project_directory(&config.paths.output, "output")?;
        ensure_disjoint(&source, "source", &content, "content")?;
        ensure_disjoint(&source, "source", &output, "output")?;
        ensure_disjoint(&content, "content", &output, "output")?;

        let assets = project_directory(&config.output.assets, "assets")?;
        let fonts = config
            .typst
            .fonts
            .paths
            .iter()
            .map(|path| project_path(path, "font"))
            .collect::<Result<_>>()?;
        for font in &fonts {
            ensure_disjoint(font, "font", &output, "output")?;
        }

        Ok(Self {
            source,
            content,
            output,
            assets,
            fonts,
        })
    }

    pub(crate) fn source(&self) -> &VirtualPath {
        &self.source
    }

    pub(crate) fn content(&self) -> &VirtualPath {
        &self.content
    }

    pub(crate) fn assets(&self) -> &VirtualPath {
        &self.assets
    }

    pub(crate) fn output_dir(&self, project: &Project) -> PathBuf {
        project.realize(&self.output)
    }

    pub(crate) fn font_dirs<'a>(
        &'a self,
        project: &'a Project,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        self.fonts.iter().map(move |path| project.realize(path))
    }

    /// Return every structural and tracked build input that watch mode should
    /// observe, excluding the generated output tree.
    pub(crate) fn watch_paths<I>(&self, project: &Project, dependencies: I) -> Vec<PathBuf>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let output = self.output_dir(project);
        let mut paths = self.structural_watch_paths(project);
        paths.extend(
            dependencies
                .into_iter()
                .filter(|path| VirtualPath::virtualize(&output, path).is_err()),
        );
        paths.sort();
        paths.dedup();
        paths
    }

    fn structural_watch_paths(&self, project: &Project) -> Vec<PathBuf> {
        let mut directories = vec![
            project.realize(&self.source),
            project.realize(&self.content),
        ];
        directories.extend(self.font_dirs(project));
        let mut paths = vec![project.config_file()];
        paths.extend(directories.iter().cloned());
        for directory in directories {
            if !directory.is_dir() {
                continue;
            }
            // Builds report traversal errors; watching every reachable directory
            // lets a later filesystem change recover without exiting watch mode.
            paths.extend(
                WalkDir::new(directory)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().is_dir())
                    .map(|entry| entry.into_path()),
            );
        }
        paths
    }
}

fn project_directory(value: &str, name: &str) -> Result<VirtualPath> {
    let path = project_path(value, name)?;
    ensure!(
        !path.is_root(),
        "{name} directory cannot be the project root"
    );
    Ok(path)
}

fn project_path(value: &str, name: &str) -> Result<VirtualPath> {
    ensure!(
        !Path::new(value).is_absolute(),
        "{name} path must be relative"
    );
    VirtualPath::new(value).with_context(|| format!("invalid {name} path `{value}`"))
}

fn ensure_disjoint(
    left: &VirtualPath,
    left_name: &str,
    right: &VirtualPath,
    right_name: &str,
) -> Result<()> {
    let left_path = Path::new(left.get_without_slash());
    let right_path = Path::new(right.get_without_slash());
    ensure!(
        !left_path.starts_with(right_path) && !right_path.starts_with(left_path),
        "{left_name} and {right_name} directories must not overlap"
    );
    Ok(())
}
