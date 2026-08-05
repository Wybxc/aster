use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{Context, Result};
use comemo::Tracked;
use serde::Deserialize;
use typst::ecow::{EcoString, eco_format};
use typst::foundations::Bytes;
use typst::syntax::VirtualPath;
use typst_html::HtmlElement;
use url::Url;

use crate::build::output::PagePublication;
use crate::foundation::files::{FileAccessError, ProjectFiles};

use super::{Processor, WalkControl, dom::HtmlElementExt};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ScriptKind {
    Classic,
    Module,
}

/// Builds and publishes scripts declared by Typst components and HTML elements.
pub(crate) struct ScriptProcessor<'a> {
    project_files: Tracked<'a, ProjectFiles>,
    project_root: PathBuf,
    modules: HashMap<ModuleSource, Bytes>,
}

impl<'a> ScriptProcessor<'a> {
    pub fn new(project_files: Tracked<'a, ProjectFiles>, project_root: &Path) -> Self {
        Self {
            project_files,
            project_root: project_root.to_owned(),
            modules: HashMap::new(),
        }
    }

    pub fn add_file(
        &mut self,
        kind: ScriptKind,
        source: &VirtualPath,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        let content = match kind {
            ScriptKind::Classic => self.project_files.read(source)?,
            ScriptKind::Module => self.resolve_module(ModuleSource::File(source.clone()))?,
        };
        page.add_script(source, content)
    }

    pub fn add_raw(
        &mut self,
        kind: ScriptKind,
        origin: &VirtualPath,
        code: EcoString,
        page: &mut PagePublication<'_>,
    ) -> Result<EcoString> {
        let content = match kind {
            ScriptKind::Classic => Bytes::from_string(code),
            ScriptKind::Module => self.resolve_module(ModuleSource::Memory {
                origin: origin.clone(),
                code,
            })?,
        };
        page.add_script(origin, content)
    }

    fn resolve_module(&mut self, source: ModuleSource) -> Result<Bytes> {
        if let Some(module) = self.modules.get(&source) {
            return Ok(module.clone());
        }
        let module = bundle_module(
            self.project_files,
            &source,
            &self.project_root,
            Path::new("esbuild"),
        )
        .map_err(|error| anyhow::anyhow!("{error:#}"))?;
        self.modules.insert(source, module.clone());
        Ok(module)
    }
}

impl Processor for ScriptProcessor<'_> {
    fn process_element(
        &mut self,
        element: &mut HtmlElement,
        page: &mut PagePublication<'_>,
    ) -> Result<WalkControl> {
        if !element.is_tag(typst_html::tag::script)
            || !element
                .get_attr("type")
                .is_some_and(|kind| kind.eq_ignore_ascii_case("module"))
        {
            return Ok(WalkControl::Continue);
        }

        let url = if let Some(reference) = element.get_attr("src") {
            let Some(reference) = local_module_reference(&reference) else {
                return Ok(WalkControl::Continue);
            };
            let source = page
                .resolve_source(reference)
                .with_context(|| format!("invalid module script source {reference}"))?;
            self.add_file(ScriptKind::Module, &source, page)?
        } else {
            let origin = page.template().clone();
            let code = element.inner_text().into();
            self.add_raw(ScriptKind::Module, &origin, code, page)?
        };

        if element.get_attr("src").is_some() {
            element.update_attr("src", move |value| *value = url);
            Ok(WalkControl::Continue)
        } else {
            element.attrs.push(typst_html::attr::src, url);
            element.children.clear();
            Ok(WalkControl::SkipChildren)
        }
    }
}

fn local_module_reference(reference: &str) -> Option<&str> {
    if reference.is_empty() || matches!(reference.chars().next(), Some('/' | '#' | '?')) {
        return None;
    }
    if Url::parse(reference).is_ok() {
        return None;
    }
    let end = reference.find(['?', '#']).unwrap_or(reference.len());
    Some(&reference[..end])
}

#[cfg(test)]
mod reference_tests {
    use super::local_module_reference;

    #[test]
    fn bundles_only_local_relative_module_references() {
        assert_eq!(
            local_module_reference("./entry.js?version=1#main"),
            Some("./entry.js")
        );
        assert_eq!(
            local_module_reference("scripts/entry.js"),
            Some("scripts/entry.js")
        );
        assert_eq!(local_module_reference("/scripts/entry.js"), None);
        assert_eq!(local_module_reference("//cdn.example.com/entry.js"), None);
        assert_eq!(local_module_reference("https://example.com/entry.js"), None);
        assert_eq!(
            local_module_reference("data:text/javascript,export{}"),
            None
        );
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ModuleSource {
    File(VirtualPath),
    Memory {
        origin: VirtualPath,
        code: EcoString,
    },
}

impl ModuleSource {
    fn origin(&self) -> &VirtualPath {
        match self {
            Self::File(path) | Self::Memory { origin: path, .. } => path,
        }
    }
}

#[derive(Deserialize)]
struct EsbuildMetafile {
    inputs: BTreeMap<String, EsbuildFile>,
    outputs: BTreeMap<String, EsbuildFile>,
}

#[derive(Deserialize)]
struct EsbuildFile {}

fn bundle_module(
    project_files: Tracked<ProjectFiles>,
    source: &ModuleSource,
    project_root: &Path,
    executable: &Path,
) -> std::result::Result<Bytes, BundleError> {
    let origin = source.origin();
    let origin_path = origin
        .realize(project_root)
        .map_err(|error| BundleError::InvalidPath {
            path: PathBuf::from(origin.get_with_slash()).into(),
            message: eco_format!("{error}"),
        })?;
    let working_dir = origin_path
        .parent()
        .ok_or_else(|| BundleError::InvalidPath {
            path: origin_path.clone().into(),
            message: "module source has no parent directory".into(),
        })?;

    if let ModuleSource::File(path) = source {
        project_files.read(path)?;
    }

    let temporary = tempfile::tempdir().map_err(|error| BundleError::Temporary {
        message: eco_format!("{error}"),
    })?;
    let output_path = temporary.path().join("module.js");
    let metafile_path = temporary.path().join("meta.json");
    let mut command = Command::new(executable);
    command
        .arg("--bundle")
        .arg("--format=esm")
        .arg("--platform=browser")
        .arg("--preserve-symlinks")
        .arg("--charset=utf8")
        .arg("--legal-comments=inline")
        .arg("--log-level=warning")
        .arg("--log-limit=0")
        .arg("--color=false")
        .arg("--external:http://*")
        .arg("--external:https://*")
        .arg(path_option("--outfile=", &output_path))
        .arg(path_option("--metafile=", &metafile_path))
        .current_dir(working_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let inline_sourcefile = match source {
        ModuleSource::File(_) => {
            command.arg(&origin_path);
            None
        }
        ModuleSource::Memory { .. } => {
            let sourcefile = origin_path.with_extension("aster-module.js");
            command
                .arg("--loader=js")
                .arg(path_option("--sourcefile=", &sourcefile))
                .stdin(Stdio::piped());
            Some(sourcefile)
        }
    };

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            BundleError::EsbuildNotFound
        } else {
            BundleError::EsbuildStart {
                message: eco_format!("{error}"),
            }
        }
    })?;
    if let ModuleSource::Memory { code, .. } = source {
        child
            .stdin
            .take()
            .expect("piped esbuild stdin")
            .write_all(code.as_bytes())
            .map_err(|error| BundleError::EsbuildInput {
                path: origin_path.clone().into(),
                message: eco_format!("{error}"),
            })?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| BundleError::EsbuildWait {
            message: eco_format!("{error}"),
        })?;
    if !output.status.success() {
        if let Some(parent) = origin.parent()
            && !parent.is_root()
        {
            project_files.watch(&parent)?;
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        return Err(BundleError::EsbuildFailed {
            path: origin_path.into(),
            message: if message.is_empty() {
                eco_format!("process exited with {}", output.status)
            } else {
                message.into()
            },
        });
    }

    let metadata = std::fs::read(&metafile_path).map_err(|error| BundleError::MetadataRead {
        path: metafile_path.clone().into(),
        message: eco_format!("{error}"),
    })?;
    let metadata: EsbuildMetafile =
        serde_json::from_slice(&metadata).map_err(|error| BundleError::MetadataParse {
            path: metafile_path.into(),
            message: eco_format!("{error}"),
        })?;
    for input in metadata.inputs.keys() {
        if input == "<stdin>" {
            continue;
        }
        let path = Path::new(input);
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            working_dir.join(path)
        };
        if inline_sourcefile
            .as_ref()
            .is_some_and(|source| source == &path)
        {
            continue;
        }
        let virtual_path =
            VirtualPath::virtualize(project_root, &path).map_err(|error| BundleError::Escapes {
                path: path.clone().into(),
                project_root: project_root.into(),
                message: eco_format!("{error}"),
            })?;
        project_files.read(&virtual_path)?;
    }

    if metadata.outputs.len() != 1 {
        return Err(BundleError::AdditionalOutputs {
            path: origin_path.into(),
            outputs: metadata
                .outputs
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
                .into(),
        });
    }

    let code = std::fs::read(&output_path).map_err(|error| BundleError::OutputRead {
        path: output_path.into(),
        message: eco_format!("{error}"),
    })?;
    Ok(Bytes::new(code))
}

fn path_option(name: &str, path: &Path) -> OsString {
    let mut value = OsString::from(name);
    value.push(path.as_os_str());
    value
}

#[derive(Debug, Clone, thiserror::Error)]
enum BundleError {
    #[error(transparent)]
    File(#[from] FileAccessError),
    #[error(
        "an ES module requires the `esbuild` executable, but it was not found\n\
         hint: install esbuild and make `esbuild` available on PATH"
    )]
    EsbuildNotFound,
    #[error("failed to start esbuild: {message}")]
    EsbuildStart { message: EcoString },
    #[error("failed to send module {path} to esbuild: {message}")]
    EsbuildInput { path: Arc<Path>, message: EcoString },
    #[error("failed to wait for esbuild: {message}")]
    EsbuildWait { message: EcoString },
    #[error("esbuild failed for {path}: {message}")]
    EsbuildFailed { path: Arc<Path>, message: EcoString },
    #[error("failed to create temporary esbuild output: {message}")]
    Temporary { message: EcoString },
    #[error("invalid module path {path}: {message}")]
    InvalidPath { path: Arc<Path>, message: EcoString },
    #[error("failed to read esbuild metafile {path}: {message}")]
    MetadataRead { path: Arc<Path>, message: EcoString },
    #[error("failed to parse esbuild metafile {path}: {message}")]
    MetadataParse { path: Arc<Path>, message: EcoString },
    #[error("module {path} produced unsupported additional outputs: {outputs}")]
    AdditionalOutputs { path: Arc<Path>, outputs: EcoString },
    #[error("module dependency {path} escapes project root {project_root}: {message}")]
    Escapes {
        path: Arc<Path>,
        project_root: Arc<Path>,
        message: EcoString,
    },
    #[error("failed to read esbuild output {path}: {message}")]
    OutputRead { path: Arc<Path>, message: EcoString },
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use comemo::Track;

    use super::*;
    use crate::foundation::Project;

    #[test]
    fn tracks_metafile_inputs_and_rebuilds_after_change() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("aster.toml"), "").unwrap();
        std::fs::create_dir(root.join("modules")).unwrap();
        let entry = root.join("modules/entry.js");
        let dependency = root.join("modules/dependency.js");
        std::fs::write(&entry, "import './dependency.js';").unwrap();
        std::fs::write(&dependency, "first dependency").unwrap();

        let executable = root.join("esbuild");
        std::fs::write(
            &executable,
            r#"#!/bin/sh
outfile=
metafile=
entry=
for argument in "$@"; do
  case "$argument" in
    --outfile=*) outfile=${argument#--outfile=} ;;
    --metafile=*) metafile=${argument#--metafile=} ;;
    -*) ;;
    *) entry=$argument ;;
  esac
done
content=
while IFS= read -r line || [ -n "$line" ]; do
  content="${content}${line}"
done < "$PWD/dependency.js"
printf '%s\n' "$content" > "$outfile"
printf '{"inputs":{"entry.js":{},"dependency.js":{}},"outputs":{"%s":{}}}\n' "$outfile" > "$metafile"
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).unwrap();

        let project = Project::open(root).unwrap();
        let mut files = ProjectFiles::new(&project);
        let source = ModuleSource::File(VirtualPath::new("modules/entry.js").unwrap());
        let first = bundle_module(files.track(), &source, root, &executable).unwrap();
        assert_eq!(first.as_slice(), b"first dependency\n");
        let dependencies = files.dependencies();
        assert!(dependencies.iter().any(|item| item.path() == entry));
        assert!(dependencies.iter().any(|item| item.path() == dependency));

        std::fs::write(&dependency, "second dependency").unwrap();
        files.reset();
        let second = bundle_module(files.track(), &source, root, &executable).unwrap();
        assert_eq!(second.as_slice(), b"second dependency\n");
    }
}
