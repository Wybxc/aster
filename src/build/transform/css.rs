use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::Result;
use comemo::Tracked;
use cssparser::serialize_string;
use lightningcss::bundler::{Bundler, FileProvider, ResolveResult, SourceProvider};
use lightningcss::dependencies::{Dependency, DependencyOptions, UrlDependency};
use lightningcss::stylesheet::{
    MinifyOptions, ParserFlags, ParserOptions, PrinterOptions, StyleSheet,
};
use lightningcss::targets::{Browsers, Targets};
use typst::ecow::{EcoString, EcoVec, eco_format};
use typst::foundations::Bytes;
use typst::syntax::VirtualPath;
use typst_html::HtmlElement;
use url::Url;

use crate::build::output::PagePublication;
use crate::build::transform::{Processor, WalkControl, dom::HtmlElementExt};
use crate::foundation::config::CssConfig;
use crate::foundation::files::{FileAccessError, ProjectFiles};

/// A cheaply cloneable CSS transformation error at the memoization seam.
///
/// Upstream lightningcss errors carry non-static lifetimes or are not Clone,
/// so their stable classifications and locations are decomposed into fields;
/// the display strings are derived from those fields. All fields are
/// reference-counted or cheap to clone.
#[derive(Debug, Clone, thiserror::Error)]
enum BundleError {
    #[error(
        "a Tailwind stylesheet requires the `tailwindcss` executable, but it was not found\n\
         hint: install the standalone Tailwind CSS CLI and make `tailwindcss` available on PATH"
    )]
    TailwindNotFound,
    #[error("failed to start Tailwind CSS CLI: {message}")]
    TailwindStart { message: EcoString },
    #[error("Tailwind CSS CLI failed for {path}: {message}")]
    TailwindFailed { path: Arc<Path>, message: EcoString },
    #[error("Tailwind CSS CLI returned non-UTF-8 output for {path}")]
    TailwindUtf8 { path: Arc<Path> },
    #[error("failed to bundle {path}: {kind}{location}")]
    Bundle {
        path: Arc<Path>,
        kind: EcoString,
        location: EcoString,
    },
    #[error("failed to transform CSS: {kind}{location}")]
    Transform {
        kind: EcoString,
        location: EcoString,
    },
    #[error("invalid CSS browser targets: {message}")]
    InvalidTargets { message: EcoString },
    #[error("failed to serialize CSS: {kind}{location}")]
    Serialize {
        kind: EcoString,
        location: EcoString,
    },
    #[error("CSS reference {path} escapes project root {project_root}")]
    Escapes {
        path: Arc<Path>,
        project_root: Arc<Path>,
    },
    #[error("invalid CSS path {path}: {message}")]
    InvalidPath { path: Arc<Path>, message: EcoString },
    #[error("invalid CSS URL {url} in {source_path}: {message}")]
    InvalidUrl {
        url: EcoString,
        source_path: Arc<Path>,
        message: EcoString,
    },
    #[error("CSS dependency placeholder {placeholder} was not found in serialized output")]
    MissingPlaceholder { placeholder: EcoString },
    #[error("CSS dependency placeholder collision for {placeholder}")]
    PlaceholderCollision { placeholder: EcoString },
    #[error(transparent)]
    File(#[from] FileAccessError),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct BundledStylesheet {
    code: EcoString,
    assets: EcoVec<CssAsset>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CssAsset {
    placeholder: EcoString,
    source: VirtualPath,
    content: Bytes,
    suffix: EcoString,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum StylesheetKind {
    Css,
    Tailwind,
}

/// Decompose a lightningcss error into its stable classification and a
/// formatted source location, both of which are cheaply cloneable.
fn decompose(error: &lightningcss::error::Error<impl std::fmt::Display>) -> (EcoString, EcoString) {
    let location = error
        .loc
        .as_ref()
        .map(|loc| eco_format!(" at {}:{}:{}", loc.filename, loc.line, loc.column))
        .unwrap_or_default();
    (eco_format!("{}", error.kind), location)
}

pub(crate) struct CssProcessor<'a> {
    project_files: Tracked<'a, ProjectFiles>,
    config: CssConfig,
    bundles: HashMap<(StylesheetKind, VirtualPath), BundledStylesheet>,
}

impl<'a> CssProcessor<'a> {
    pub fn new(project_files: Tracked<'a, ProjectFiles>, config: &CssConfig) -> Result<Self> {
        resolve_targets(config).map_err(|error| anyhow::anyhow!("{error:#}"))?;
        Ok(Self {
            project_files,
            config: config.clone(),
            bundles: HashMap::new(),
        })
    }
}

impl Processor for CssProcessor<'_> {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        if !element.is_tag(typst_html::tag::link) {
            return Ok(WalkControl::Continue);
        }
        let Some((kind, relation)) = element.get_attr("rel").and_then(|relation| {
            let kind = match relation.as_str() {
                "css" => StylesheetKind::Css,
                "tailwind" => StylesheetKind::Tailwind,
                _ => return None,
            };
            Some((kind, relation))
        }) else {
            return Ok(WalkControl::Continue);
        };

        let href = element.get_attr("href").ok_or_else(|| {
            anyhow::anyhow!("link element with rel=\"{relation}\" is missing href attribute")
        })?;
        let source = page.resolve_source(&href)?;
        let key = (kind, source.clone());
        let bundle = if let Some(bundle) = self.bundles.get(&key) {
            bundle.clone()
        } else {
            let bundle = match kind {
                StylesheetKind::Css => bundle_file(
                    self.project_files,
                    &source,
                    page.project_root(),
                    &self.config,
                ),
                StylesheetKind::Tailwind => bundle_tailwind_file(
                    self.project_files,
                    &source,
                    page.project_root(),
                    &self.config,
                ),
            }
            .map_err(|error| anyhow::anyhow!("{error:#}"))?;
            self.bundles.insert(key, bundle.clone());
            bundle
        };
        let mut css = bundle.code.to_string();
        for asset in bundle.assets {
            let mut reference = page
                .add_css_asset(&asset.source, asset.content)?
                .to_string();
            reference.push_str(&asset.suffix);
            replace_css_url(&mut css, &asset.placeholder, &reference)
                .map_err(|error| anyhow::anyhow!("{error:#}"))?;
        }
        let url = page.add_bundled_stylesheet(&source, css.into_bytes())?;

        element.update_attr("href", move |value| *value = url);
        element.update_attr("rel", |value| *value = "stylesheet".into());
        Ok(WalkControl::Continue)
    }
}

/// Bundle a CSS entry point while confining and tracking every transitive import.
#[comemo::memoize]
fn bundle_file(
    project_files: Tracked<ProjectFiles>,
    entry: &VirtualPath,
    project_root: &Path,
    config: &CssConfig,
) -> std::result::Result<BundledStylesheet, BundleError> {
    let entry = entry
        .realize(project_root)
        .map_err(|error| BundleError::InvalidPath {
            path: PathBuf::from(entry.get_with_slash()).into(),
            message: eco_format!("{error}"),
        })?;
    let provider = ConfinedFileProvider::new(project_root.to_owned(), project_files);
    let targets = resolve_targets(config)?;
    let mut flags = ParserFlags::empty();
    flags.set(ParserFlags::CUSTOM_MEDIA, config.custom_media);
    let mut bundler = Bundler::new(
        &provider,
        None,
        ParserOptions {
            flags,
            ..ParserOptions::default()
        },
    );
    let stylesheet = bundler.bundle(&entry).map_err(|error| {
        let (kind, location) = decompose(&error);
        BundleError::Bundle {
            path: entry.into(),
            kind,
            location,
        }
    })?;
    finish_stylesheet(stylesheet, &provider, targets, config)
}

fn bundle_tailwind_file(
    project_files: Tracked<ProjectFiles>,
    entry: &VirtualPath,
    project_root: &Path,
    config: &CssConfig,
) -> std::result::Result<BundledStylesheet, BundleError> {
    observe_tailwind_inputs(project_files, entry)?;
    let entry = entry
        .realize(project_root)
        .map_err(|error| BundleError::InvalidPath {
            path: PathBuf::from(entry.get_with_slash()).into(),
            message: eco_format!("{error}"),
        })?;
    let css = run_tailwind_cli(OsStr::new("tailwindcss"), &entry, project_root)?;
    let provider = ConfinedFileProvider::new(project_root.to_owned(), project_files);
    let targets = resolve_targets(config)?;
    let mut flags = ParserFlags::empty();
    flags.set(ParserFlags::CUSTOM_MEDIA, config.custom_media);
    let stylesheet = StyleSheet::parse(
        &css,
        ParserOptions {
            filename: entry.to_string_lossy().into_owned(),
            flags,
            ..ParserOptions::default()
        },
    )
    .map_err(|error| {
        let (kind, location) = decompose(&error);
        BundleError::Bundle {
            path: entry.into(),
            kind,
            location,
        }
    })?;
    finish_stylesheet(stylesheet, &provider, targets, config)
}

fn observe_tailwind_inputs(
    project_files: Tracked<ProjectFiles>,
    entry: &VirtualPath,
) -> std::result::Result<(), BundleError> {
    project_files.read(entry)?;
    if let Some(directory) = entry.parent()
        && !directory.is_root()
    {
        project_files.list(&directory, true)?;
    }

    for name in [
        "tailwind.config.js",
        "tailwind.config.cjs",
        "tailwind.config.mjs",
        "tailwind.config.ts",
    ] {
        let path = VirtualPath::new(name).expect("Tailwind config path is valid");
        let _ = project_files.read(&path);
    }
    Ok(())
}

fn run_tailwind_cli(
    executable: &OsStr,
    entry: &Path,
    project_root: &Path,
) -> std::result::Result<String, BundleError> {
    let output = Command::new(executable)
        .arg("--input")
        .arg(entry)
        .current_dir(project_root)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                BundleError::TailwindNotFound
            } else {
                BundleError::TailwindStart {
                    message: eco_format!("{error}"),
                }
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        return Err(BundleError::TailwindFailed {
            path: entry.into(),
            message: if message.is_empty() {
                eco_format!("process exited with {}", output.status)
            } else {
                message.into()
            },
        });
    }
    String::from_utf8(output.stdout).map_err(|_| BundleError::TailwindUtf8 { path: entry.into() })
}

fn finish_stylesheet(
    mut stylesheet: StyleSheet<'_>,
    provider: &ConfinedFileProvider<'_>,
    targets: Targets,
    config: &CssConfig,
) -> std::result::Result<BundledStylesheet, BundleError> {
    if config.minify || targets.browsers.is_some() {
        stylesheet
            .minify(MinifyOptions {
                targets,
                ..MinifyOptions::default()
            })
            .map_err(|error| {
                let (kind, location) = decompose(&error);
                BundleError::Transform { kind, location }
            })?;
    }
    let mut result = stylesheet
        .to_css(PrinterOptions {
            minify: config.minify,
            targets,
            analyze_dependencies: Some(DependencyOptions::default()),
            ..PrinterOptions::default()
        })
        .map_err(|error| {
            let (kind, location) = decompose(&error);
            BundleError::Serialize { kind, location }
        })?;

    let mut assets = EcoVec::new();
    let mut seen = BTreeMap::new();
    for dependency in result.dependencies.take().unwrap_or_default() {
        match dependency {
            Dependency::Import(dependency) => {
                if remember_dependency(
                    &mut seen,
                    &dependency.placeholder,
                    &dependency.url,
                    &dependency.loc.file_path,
                )? {
                    replace_css_url(&mut result.code, &dependency.placeholder, &dependency.url)?;
                }
            }
            Dependency::Url(dependency) => {
                if !remember_dependency(
                    &mut seen,
                    &dependency.placeholder,
                    &dependency.url,
                    &dependency.loc.file_path,
                )? {
                    continue;
                }
                if let Some(asset) = provider.load_asset(&dependency)? {
                    assets.push(asset);
                } else {
                    replace_css_url(&mut result.code, &dependency.placeholder, &dependency.url)?;
                }
            }
        }
    }

    Ok(BundledStylesheet {
        code: result.code.into(),
        assets,
    })
}

fn remember_dependency(
    seen: &mut BTreeMap<EcoString, (EcoString, Arc<Path>)>,
    placeholder: &str,
    url: &str,
    source: &str,
) -> std::result::Result<bool, BundleError> {
    let signature = (EcoString::from(url), Arc::<Path>::from(Path::new(source)));
    if let Some(existing) = seen.get(placeholder) {
        if existing != &signature {
            return Err(BundleError::PlaceholderCollision {
                placeholder: placeholder.into(),
            });
        }
        return Ok(false);
    }
    seen.insert(placeholder.into(), signature);
    Ok(true)
}

fn replace_css_url(
    code: &mut String,
    placeholder: &str,
    replacement: &str,
) -> std::result::Result<(), BundleError> {
    let placeholder_value = serialize_css_string(placeholder);
    if !code.contains(&placeholder_value) {
        return Err(BundleError::MissingPlaceholder {
            placeholder: placeholder.into(),
        });
    }
    *code = code.replace(&placeholder_value, &serialize_css_string(replacement));
    Ok(())
}

fn serialize_css_string(value: &str) -> String {
    let mut result = String::new();
    serialize_string(value, &mut result).expect("writing to a string cannot fail");
    result
}

fn resolve_targets(config: &CssConfig) -> std::result::Result<Targets, BundleError> {
    if config.targets.is_empty() {
        return Ok(Targets::default());
    }

    Browsers::from_browserslist(&config.targets)
        .map(Into::into)
        .map_err(|error| BundleError::InvalidTargets {
            message: eco_format!("{error}"),
        })
}

struct ConfinedFileProvider<'a> {
    project_root: PathBuf,
    project_files: Tracked<'a, ProjectFiles>,
    files: FileProvider,
}

impl<'a> ConfinedFileProvider<'a> {
    fn new(project_root: PathBuf, project_files: Tracked<'a, ProjectFiles>) -> Self {
        Self {
            project_root,
            project_files,
            files: FileProvider::new(),
        }
    }

    fn confined(&self, path: &Path) -> std::result::Result<(VirtualPath, PathBuf), BundleError> {
        let virtual_path = VirtualPath::virtualize(&self.project_root, path).map_err(|_| {
            BundleError::Escapes {
                path: path.into(),
                project_root: self.project_root.clone().into(),
            }
        })?;
        let path =
            virtual_path
                .realize(&self.project_root)
                .map_err(|error| BundleError::InvalidPath {
                    path: path.into(),
                    message: eco_format!("{error}"),
                })?;
        Ok((virtual_path, path))
    }

    fn load_asset(
        &self,
        dependency: &UrlDependency,
    ) -> std::result::Result<Option<CssAsset>, BundleError> {
        let origin = Path::new(&dependency.loc.file_path);
        let Some((path, suffix)) = resolve_file_reference(origin, &dependency.url)? else {
            return Ok(None);
        };
        let (source, _) = self.confined(&path)?;
        let content = self.project_files.read(&source)?;
        Ok(Some(CssAsset {
            placeholder: dependency.placeholder.as_str().into(),
            source,
            content,
            suffix,
        }))
    }
}

impl SourceProvider for ConfinedFileProvider<'_> {
    type Error = BundleError;

    fn read<'a>(&'a self, file: &Path) -> std::result::Result<&'a str, Self::Error> {
        let (virtual_file, file) = self.confined(file)?;
        self.project_files.read(&virtual_file)?;
        self.files
            .read(&file)
            .map_err(|error| FileAccessError::io(file.into(), error).into())
    }

    fn resolve(
        &self,
        specifier: &str,
        originating_file: &Path,
    ) -> std::result::Result<ResolveResult, Self::Error> {
        let Some((candidate, _)) = resolve_file_reference(originating_file, specifier)? else {
            return Ok(ResolveResult::External(specifier.to_owned()));
        };
        let (_, path) = self.confined(&candidate)?;
        Ok(ResolveResult::File(path))
    }
}

fn resolve_file_reference(
    origin: &Path,
    reference: &str,
) -> std::result::Result<Option<(PathBuf, EcoString)>, BundleError> {
    if is_browser_managed_reference(reference) {
        return Ok(None);
    }

    let base = Url::from_file_path(origin).map_err(|()| BundleError::InvalidUrl {
        url: reference.into(),
        source_path: origin.into(),
        message: "source path cannot be represented as a file URL".into(),
    })?;
    let mut resolved = base
        .join(reference)
        .map_err(|error| BundleError::InvalidUrl {
            url: reference.into(),
            source_path: origin.into(),
            message: eco_format!("{error}"),
        })?;
    if resolved.scheme() != "file" {
        return Ok(None);
    }

    let query = resolved.query().map(str::to_owned);
    let fragment = resolved.fragment().map(str::to_owned);
    resolved.set_query(None);
    resolved.set_fragment(None);
    let path = resolved
        .to_file_path()
        .map_err(|()| BundleError::InvalidUrl {
            url: reference.into(),
            source_path: origin.into(),
            message: "resolved URL cannot be represented as a file path".into(),
        })?;

    let mut suffix = String::new();
    if let Some(query) = query {
        suffix.push('?');
        suffix.push_str(&query);
    }
    if let Some(fragment) = fragment {
        suffix.push('#');
        suffix.push_str(&fragment);
    }
    Ok(Some((path, suffix.into())))
}

fn is_browser_managed_reference(reference: &str) -> bool {
    reference.is_empty()
        || matches!(reference.chars().next(), Some('/' | '#' | '?'))
        || Url::parse(reference).is_ok()
}

#[cfg(test)]
mod tests {
    use comemo::Track;

    use super::*;
    use crate::foundation::{FilesystemDependency, Project};

    #[test]
    fn tailwind_inputs_track_entry_tree_and_conventional_configs() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join("styles")).unwrap();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        std::fs::write(root.join("styles/site.css"), "@import \"tailwindcss\";").unwrap();
        let project = Project::open(root).unwrap();
        let mut files = ProjectFiles::new(&project);
        let entry = VirtualPath::new("styles/site.css").unwrap();

        observe_tailwind_inputs(files.track(), &entry).unwrap();

        let dependencies = files.dependencies();
        assert!(dependencies.contains(&FilesystemDependency::File(root.join("styles/site.css"))));
        assert!(dependencies.contains(&FilesystemDependency::Tree(root.join("styles"))));
        for name in [
            "tailwind.config.js",
            "tailwind.config.cjs",
            "tailwind.config.mjs",
            "tailwind.config.ts",
        ] {
            assert!(dependencies.contains(&FilesystemDependency::File(root.join(name))));
        }
    }
}
