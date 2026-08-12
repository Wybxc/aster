use std::collections::{BTreeMap, HashMap};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use aho_corasick::AhoCorasick;
use anyhow::Result;
use comemo::Tracked;
use lightningcss::bundler::{Bundler, FileProvider, ResolveResult, SourceProvider};
use lightningcss::dependencies::{Dependency, DependencyOptions, UrlDependency};
use lightningcss::stylesheet::{
    MinifyOptions, ParserFlags, ParserOptions, PrinterOptions, ToCssResult,
};
use lightningcss::targets::{Browsers, Targets};
use typst::ecow::{EcoString, EcoVec, eco_format};
use typst::foundations::Bytes;
use typst::syntax::VirtualPath;
use url::Url;

use crate::build::files::{FileAccessError, ProjectFiles};
use crate::foundation::config::CssConfig;

use super::super::url::{UrlReference, classify_url};

pub struct CssPipeline<'a> {
    project_files: Tracked<'a, ProjectFiles>,
    config: CssConfig,
    stylesheets: HashMap<StylesheetSource, BundledStylesheet>,
}

impl<'a> CssPipeline<'a> {
    pub fn new(project_files: Tracked<'a, ProjectFiles>, config: &CssConfig) -> Result<Self> {
        resolve_targets(&config.targets).map_err(|error| anyhow::anyhow!("{error:#}"))?;
        Ok(Self {
            project_files,
            config: config.clone(),
            stylesheets: HashMap::new(),
        })
    }

    /// Transform raw CSS declared by a component.
    pub fn bundle_raw(
        &mut self,
        origin: &VirtualPath,
        code: EcoString,
        project_root: &Path,
    ) -> Result<BundledStylesheet> {
        let source = StylesheetSource::Css(CssBundleSource::Memory {
            origin: origin.clone(),
            code,
        });
        self.bundle(source, project_root)
    }

    pub fn bundle_stylesheet(
        &mut self,
        kind: StylesheetKind,
        source: &VirtualPath,
        project_root: &Path,
    ) -> Result<BundledStylesheet> {
        let key = match kind {
            StylesheetKind::Css => StylesheetSource::Css(CssBundleSource::File(source.clone())),
            StylesheetKind::Tailwind => StylesheetSource::Tailwind(source.clone()),
        };
        self.bundle(key, project_root)
    }

    fn bundle(
        &mut self,
        source: StylesheetSource,
        project_root: &Path,
    ) -> Result<BundledStylesheet> {
        let stylesheet = if let Some(stylesheet) = self.stylesheets.get(&source) {
            stylesheet.clone()
        } else {
            let operation = match &source {
                StylesheetSource::Css(CssBundleSource::File(path)) => {
                    tracing::debug_span!(
                        "stylesheet",
                        source = %path.get_with_slash(),
                        message = %format_args!("bundled stylesheet {}", path.get_with_slash())
                    )
                }
                StylesheetSource::Css(CssBundleSource::Memory { origin, .. }) => {
                    tracing::debug_span!(
                        "stylesheet",
                        source = %origin.get_with_slash(),
                        message = %format_args!("bundled stylesheet {}", origin.get_with_slash())
                    )
                }
                StylesheetSource::Tailwind(path) => {
                    tracing::debug_span!(
                        "tailwind",
                        source = %path.get_with_slash(),
                        message = %format_args!(
                            "processed stylesheet {} with Tailwind",
                            path.get_with_slash()
                        )
                    )
                }
            };
            let _operation = operation.enter();
            let bundle = match &source {
                StylesheetSource::Css(source) => {
                    bundle_css(self.project_files, source, project_root, &self.config)
                }
                StylesheetSource::Tailwind(path) => {
                    bundle_tailwind_file(self.project_files, path, project_root, &self.config)
                }
            }
            .map_err(|error| anyhow::anyhow!("{error:#}"))?;
            self.stylesheets.insert(source, bundle.clone());
            bundle
        };
        Ok(stylesheet)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StylesheetKind {
    Css,
    Tailwind,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum StylesheetSource {
    Css(CssBundleSource),
    Tailwind(VirtualPath),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum CssBundleSource {
    File(VirtualPath),
    Memory {
        origin: VirtualPath,
        code: EcoString,
    },
}

fn bundle_tailwind_file(
    project_files: Tracked<ProjectFiles>,
    entry: &VirtualPath,
    project_root: &Path,
    config: &CssConfig,
) -> std::result::Result<BundledStylesheet, BundleError> {
    project_files.watch(entry)?;
    let filesystem_entry =
        entry
            .realize(project_root)
            .map_err(|error| BundleError::InvalidPath {
                path: PathBuf::from(entry.get_with_slash()).into(),
                message: eco_format!("{error}"),
            })?;
    let code = run_tailwind_cli(OsStr::new("tailwindcss"), &filesystem_entry, project_root)?;
    bundle_css(
        project_files,
        &CssBundleSource::Memory {
            origin: entry.clone(),
            code: code.into(),
        },
        project_root,
        config,
    )
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

#[comemo::memoize]
fn bundle_css(
    project_files: Tracked<ProjectFiles>,
    source: &CssBundleSource,
    project_root: &Path,
    config: &CssConfig,
) -> std::result::Result<BundledStylesheet, BundleError> {
    let virtual_entry = match source {
        CssBundleSource::File(entry) => entry,
        CssBundleSource::Memory { origin, .. } => origin,
    };
    let entry = virtual_entry
        .realize(project_root)
        .map_err(|error| BundleError::InvalidPath {
            path: PathBuf::from(virtual_entry.get_with_slash()).into(),
            message: eco_format!("{error}"),
        })?;

    match source {
        CssBundleSource::File(_) => {
            let provider = ConfinedFileProvider::new(project_root.to_owned(), project_files);
            bundle_with_provider(&entry, &provider, &provider, config)
        }
        CssBundleSource::Memory { code, .. } => {
            let provider = MemoryFileProvider::new(
                entry.clone(),
                code.clone(),
                project_root.to_owned(),
                project_files,
            );
            bundle_with_provider(&entry, &provider, &provider.files, config)
        }
    }
}

fn bundle_with_provider(
    entry: &Path,
    provider: &impl SourceProvider<Error = BundleError>,
    files: &ConfinedFileProvider<'_>,
    config: &CssConfig,
) -> std::result::Result<BundledStylesheet, BundleError> {
    let targets = resolve_targets(&config.targets)?;
    let mut flags = ParserFlags::empty();
    flags.set(ParserFlags::CUSTOM_MEDIA, config.custom_media);
    let mut bundler = Bundler::new(
        provider,
        None,
        ParserOptions {
            flags,
            ..ParserOptions::default()
        },
    );
    let mut stylesheet = bundler.bundle(entry).map_err(|error| {
        let (kind, location) = decompose(&error);
        BundleError::Bundle {
            path: entry.into(),
            kind,
            location,
        }
    })?;

    stylesheet
        .minify(MinifyOptions {
            targets,
            ..MinifyOptions::default()
        })
        .map_err(|error| {
            let (kind, location) = decompose(&error);
            BundleError::Transform { kind, location }
        })?;
    let ToCssResult {
        code, dependencies, ..
    } = stylesheet
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

    let mut references = EcoVec::new();
    let dependencies = dependencies.unwrap_or_default();
    let mut seen = BTreeMap::new();
    for dependency in &dependencies {
        let (placeholder, url, source) = match dependency {
            Dependency::Import(dependency) => (
                dependency.placeholder.as_str(),
                dependency.url.as_str(),
                dependency.loc.file_path.as_str(),
            ),
            Dependency::Url(dependency) => (
                dependency.placeholder.as_str(),
                dependency.url.as_str(),
                dependency.loc.file_path.as_str(),
            ),
        };
        let signature = (url, source);
        if let Some(existing) = seen.get(placeholder) {
            if existing != &signature {
                return Err(BundleError::PlaceholderCollision {
                    placeholder: placeholder.into(),
                });
            }
            continue;
        }
        seen.insert(placeholder, signature);

        match dependency {
            Dependency::Import(dependency) => {
                references.push(CssReference::Url {
                    placeholder: dependency.placeholder.as_str().into(),
                    url: dependency.url.as_str().into(),
                });
            }
            Dependency::Url(dependency) => {
                references.push(files.resolve_url_reference(dependency)?);
            }
        }
    }

    Ok(BundledStylesheet {
        code: code.into(),
        references,
    })
}

struct MemoryFileProvider<'a> {
    entry: PathBuf,
    code: EcoString,
    files: ConfinedFileProvider<'a>,
}

impl<'a> MemoryFileProvider<'a> {
    fn new(
        entry: PathBuf,
        code: EcoString,
        project_root: PathBuf,
        project_files: Tracked<'a, ProjectFiles>,
    ) -> Self {
        Self {
            entry,
            code,
            files: ConfinedFileProvider::new(project_root, project_files),
        }
    }
}

impl SourceProvider for MemoryFileProvider<'_> {
    type Error = BundleError;

    fn read<'a>(&'a self, file: &Path) -> std::result::Result<&'a str, Self::Error> {
        if file == self.entry {
            Ok(&self.code)
        } else {
            self.files.read(file)
        }
    }

    fn resolve(
        &self,
        specifier: &str,
        originating_file: &Path,
    ) -> std::result::Result<ResolveResult, Self::Error> {
        self.files.resolve(specifier, originating_file)
    }
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

    fn resolve_url_reference(
        &self,
        dependency: &UrlDependency,
    ) -> std::result::Result<CssReference, BundleError> {
        let origin = Path::new(&dependency.loc.file_path);
        let Some((path, suffix)) =
            resolve_file_reference(&self.project_root, origin, &dependency.url)?
        else {
            return Ok(CssReference::Url {
                placeholder: dependency.placeholder.as_str().into(),
                url: dependency.url.as_str().into(),
            });
        };
        let (source, _) = self.confined(&path)?;
        let content = self.project_files.read(&source)?;
        Ok(CssReference::Asset {
            placeholder: dependency.placeholder.as_str().into(),
            source,
            content,
            suffix,
        })
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
        let Some((candidate, _)) =
            resolve_file_reference(&self.project_root, originating_file, specifier)?
        else {
            return Ok(ResolveResult::External(specifier.to_owned()));
        };
        let (_, path) = self.confined(&candidate)?;
        Ok(ResolveResult::File(path))
    }
}

/// Serialized CSS with every dependency placeholder still unresolved.
///
/// Each distinct placeholder has exactly one entry in `references`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct BundledStylesheet {
    code: EcoString,
    references: EcoVec<CssReference>,
}

impl BundledStylesheet {
    pub fn resolve_references(
        self,
        mut publish_asset: impl FnMut(&VirtualPath, Bytes) -> Result<EcoString>,
    ) -> Result<Bytes> {
        if self.references.is_empty() {
            return Ok(Bytes::from_string(self.code));
        }

        let mut replacements = Vec::with_capacity(self.references.len());
        for reference in self.references {
            let (placeholder, url) = match reference {
                CssReference::Url { placeholder, url } => (placeholder, url),
                CssReference::Asset {
                    placeholder,
                    source,
                    content,
                    suffix,
                } => {
                    let mut url = publish_asset(&source, content)?;
                    url.push_str(&suffix);
                    (placeholder, url)
                }
            };
            replacements.push((placeholder, url));
        }
        replace_css_placeholders(&self.code, &replacements)
            .map(Bytes::from_string)
            .map_err(|error| anyhow::anyhow!("{error:#}"))
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum CssReference {
    Url {
        placeholder: EcoString,
        url: EcoString,
    },
    Asset {
        placeholder: EcoString,
        source: VirtualPath,
        content: Bytes,
        suffix: EcoString,
    },
}

fn replace_css_placeholders(
    code: &str,
    replacements: &[(EcoString, EcoString)],
) -> std::result::Result<String, BundleError> {
    let serialize = |value: &str| {
        let mut result = String::new();
        cssparser::serialize_string(value, &mut result).expect("writing to a string cannot fail");
        result
    };
    let patterns = replacements
        .iter()
        .map(|(placeholder, _)| serialize(placeholder))
        .collect::<Vec<_>>();
    let serialized_replacements = replacements
        .iter()
        .map(|(_, replacement)| serialize(replacement))
        .collect::<Vec<_>>();
    let matcher = AhoCorasick::new(&patterns).map_err(|error| BundleError::PlaceholderMatcher {
        message: eco_format!("{error}"),
    })?;
    let mut found = vec![false; patterns.len()];
    let mut result = String::with_capacity(code.len());
    matcher.replace_all_with(code, &mut result, |matched, _, output| {
        let index = matched.pattern().as_usize();
        found[index] = true;
        output.push_str(&serialized_replacements[index]);
        true
    });

    if let Some(index) = found.iter().position(|found| !found) {
        return Err(BundleError::MissingPlaceholder {
            placeholder: replacements[index].0.clone(),
        });
    }
    Ok(result)
}

fn resolve_file_reference(
    project_root: &Path,
    origin: &Path,
    reference: &str,
) -> std::result::Result<Option<(PathBuf, EcoString)>, BundleError> {
    let (base, path, suffix) = match classify_url(reference) {
        UrlReference::Rooted { path, suffix } => (
            Url::from_directory_path(project_root),
            path.trim_start_matches('/'),
            suffix,
        ),
        UrlReference::Relative { path, suffix } => (Url::from_file_path(origin), path, suffix),
        UrlReference::Data { .. } | UrlReference::Browser => return Ok(None),
    };
    let base = base.map_err(|()| BundleError::InvalidUrl {
        url: reference.into(),
        source_path: origin.into(),
        message: "source path cannot be represented as a file URL".into(),
    })?;
    let resolved = base.join(path).map_err(|error| BundleError::InvalidUrl {
        url: reference.into(),
        source_path: origin.into(),
        message: eco_format!("{error}"),
    })?;
    if resolved.scheme() != "file" {
        return Ok(None);
    }

    let path = resolved
        .to_file_path()
        .map_err(|()| BundleError::InvalidUrl {
            url: reference.into(),
            source_path: origin.into(),
            message: "resolved URL cannot be represented as a file path".into(),
        })?;

    Ok(Some((path, suffix.into())))
}

#[comemo::memoize]
fn resolve_targets(queries: &EcoVec<EcoString>) -> std::result::Result<Targets, BundleError> {
    if queries.is_empty() {
        return Ok(Targets::default());
    }

    Browsers::from_browserslist(queries)
        .map(Into::into)
        .map_err(|error| BundleError::InvalidTargets {
            message: eco_format!("{error}"),
        })
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
    #[error("failed to build CSS dependency placeholder matcher: {message}")]
    PlaceholderMatcher { message: EcoString },
    #[error("CSS dependency placeholder collision for {placeholder}")]
    PlaceholderCollision { placeholder: EcoString },
    #[error(transparent)]
    File(#[from] FileAccessError),
}
