use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use typst::syntax::VirtualPath;

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

    /// Return the manifest path in the project virtual filesystem.
    pub fn config_path(&self) -> VirtualPath {
        VirtualPath::new("aster.toml").expect("the manifest path is a valid virtual path")
    }

    /// Resolve a validated virtual project path lexically against the project root.
    pub fn realize(&self, path: &VirtualPath) -> PathBuf {
        path.realize(&self.root)
            .expect("validated project path must realize within the project root")
    }
}

/// Validated project-relative paths used by one build configuration.
#[derive(Clone)]
pub struct ProjectLayout {
    pages: VirtualPath,
    content: VirtualPath,
    public: VirtualPath,
    output: VirtualPath,
    generated_assets: VirtualPath,
    fonts: Vec<VirtualPath>,
    watch_paths: Vec<VirtualPath>,
}

impl ProjectLayout {
    /// Validate the configured project and output paths.
    pub fn new(config: &AsterConfig) -> Result<Self> {
        let pages = project_directory(&config.paths.pages, "pages")?;
        let content = project_directory(&config.paths.content, "content")?;
        let public = project_directory(&config.paths.public, "public")?;
        let output = project_directory(&config.paths.output, "output")?;
        ensure_disjoint(&pages, "pages", &content, "content")?;
        ensure_disjoint(&pages, "pages", &public, "public")?;
        ensure_disjoint(&pages, "pages", &output, "output")?;
        ensure_disjoint(&content, "content", &public, "public")?;
        ensure_disjoint(&content, "content", &output, "output")?;
        ensure_disjoint(&public, "public", &output, "output")?;

        let generated_assets = project_directory(&config.output.assets, "generated assets")?;
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
        let watch_paths = config
            .watch
            .paths
            .iter()
            .map(|value| {
                let path = project_path(value, "watch")?;
                ensure!(!path.is_root(), "watch path cannot be the project root");
                ensure!(
                    !overlaps(&path, &output),
                    "watch path `{}` must not overlap the output directory",
                    path.get_without_slash()
                );
                Ok(path)
            })
            .collect::<Result<_>>()?;

        Ok(Self {
            pages,
            content,
            public,
            output,
            generated_assets,
            fonts,
            watch_paths,
        })
    }

    /// Return the page template directory.
    pub fn pages(&self) -> &VirtualPath {
        &self.pages
    }

    /// Return the content collection directory.
    pub fn content(&self) -> &VirtualPath {
        &self.content
    }

    /// Return the public file directory.
    pub fn public(&self) -> &VirtualPath {
        &self.public
    }

    /// Return the generated asset directory within the output tree.
    pub fn generated_assets(&self) -> &VirtualPath {
        &self.generated_assets
    }

    /// Return the generated output directory.
    pub fn output(&self) -> &VirtualPath {
        &self.output
    }

    /// Iterate over configured project font directories.
    pub fn font_dirs(&self) -> impl Iterator<Item = &VirtualPath> {
        self.fonts.iter()
    }

    /// Iterate over additional configured watch paths.
    pub fn watch_paths(&self) -> impl Iterator<Item = &VirtualPath> {
        self.watch_paths.iter()
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
    ensure!(
        !overlaps(left, right),
        "{left_name} and {right_name} directories must not overlap"
    );
    Ok(())
}

fn overlaps(left: &VirtualPath, right: &VirtualPath) -> bool {
    let left = Path::new(left.get_without_slash());
    let right = Path::new(right.get_without_slash());
    left.starts_with(right) || right.starts_with(left)
}
