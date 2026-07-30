use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use typst::Library;
use typst::ecow::EcoString;
use typst::foundations::{Content, Dict, Str, Value, dict};
use typst::introspection::MetadataElem;
use typst::utils::LazyHash;

use crate::compile::TypstSession;

pub const PROTOCOL_VERSION: i64 = 1;
pub const INPUT_NAME: &str = "_aster";

pub struct LoadedContent {
    pub protocol: Value,
    pub warnings: Vec<String>,
}

/// Build the complete `_aster` protocol value, including the empty state.
pub fn load(session: &TypstSession, library: &LazyHash<Library>) -> Result<LoadedContent> {
    let project = session.project();
    let content_dir = project.content_dir();
    let mut collections: BTreeMap<EcoString, Vec<(EcoString, PathBuf, Content)>> = BTreeMap::new();
    let mut warnings = Vec::new();

    for path in project
        .content_files()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "typ"))
    {
        let content_relative = path
            .strip_prefix(&content_dir)
            .context("content path error")?;
        let project_relative = path
            .strip_prefix(project.root())
            .context("content path is outside project")?;
        if content_relative.components().count() < 2 {
            bail!(
                "entry {} is not inside a collection; expected content/<collection>/.../<id>.typ",
                path.display()
            );
        }

        let mut components = content_relative.components();
        let collection = components
            .next()
            .map(|component| EcoString::from(component.as_os_str().to_string_lossy().as_ref()))
            .context("entry not inside a collection directory")?;
        let id = {
            let mut path = PathBuf::new();
            for component in components {
                path.push(component);
            }
            path.set_extension("");
            EcoString::from(path.to_string_lossy().replace('\\', "/"))
        };

        let evaluated = session.evaluate(&path, library)?;
        warnings.extend(evaluated.warnings);
        collections.entry(collection).or_default().push((
            id,
            project_relative.to_owned(),
            evaluated.content,
        ));
    }

    Ok(LoadedContent {
        protocol: protocol_value(collections),
        warnings,
    })
}

#[cfg(test)]
pub fn empty() -> Value {
    protocol_value(BTreeMap::new())
}

pub fn install(config: Dict, protocol: Value) -> Result<Dict> {
    if config.contains(INPUT_NAME) {
        bail!("`{INPUT_NAME}` is reserved for Aster's content protocol");
    }
    let mut inputs = config;
    inputs.insert(Str::from(INPUT_NAME), protocol);
    Ok(inputs)
}

pub fn with_route_params(base: &Dict, params: &crate::route::ParamSet) -> Result<Dict> {
    let mut inputs = base.clone();
    for (name, value) in params {
        if name.as_str() == INPUT_NAME {
            bail!("route parameter `{INPUT_NAME}` is reserved");
        }
        if inputs.contains(name.as_str()) {
            bail!("route parameter `{name}` conflicts with configuration input");
        }
        inputs.insert(
            Str::from(name.as_str()),
            Value::Str(Str::from(value.as_str())),
        );
    }
    Ok(inputs)
}

fn protocol_value(collections: BTreeMap<EcoString, Vec<(EcoString, PathBuf, Content)>>) -> Value {
    let mut packed_collections = Dict::new();
    for (collection_name, entries) in collections {
        let mut packed_entries = Dict::new();
        for (id, relative_path, body) in entries {
            packed_entries.insert(
                Str::from(id.as_str()),
                Value::Dict(dict! {
                    "id" => id.as_str(),
                    "collection" => collection_name.as_str(),
                    "file-path" => relative_path.to_string_lossy().replace('\\', "/"),
                    "body" => body.clone(),
                    "metadata" => frontmatter(&body).unwrap_or_default(),
                }),
            );
        }
        packed_collections.insert(
            Str::from(collection_name.as_str()),
            Value::Dict(packed_entries),
        );
    }

    Value::Dict(dict! {
        "protocol" => PROTOCOL_VERSION,
        "collections" => packed_collections,
    })
}

fn frontmatter(content: &Content) -> Option<Dict> {
    content
        .traverse(&mut |element| {
            if element
                .label()
                .is_some_and(|label| *label.resolve() == *"frontmatter")
                && let Some(metadata) = element.to_packed::<MetadataElem>()
                && let Value::Dict(dict) = &metadata.value
            {
                ControlFlow::Break(dict.clone())
            } else {
                ControlFlow::Continue(())
            }
        })
        .break_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_protocol_has_one_owner() {
        let Value::Dict(protocol) = empty() else {
            panic!("protocol must be a dictionary");
        };
        assert_eq!(
            protocol.get("protocol").unwrap(),
            &Value::Int(PROTOCOL_VERSION)
        );
        assert_eq!(
            protocol.get("collections").unwrap(),
            &Value::Dict(Dict::new())
        );
    }

    #[test]
    fn rejects_reserved_config_input() {
        let mut config = Dict::new();
        config.insert(Str::from(INPUT_NAME), Value::Int(0));
        assert!(install(config, empty()).is_err());
    }

    #[test]
    fn route_params_cannot_replace_internal_or_config_inputs() {
        let base = install(Dict::new(), empty()).unwrap();
        assert!(
            with_route_params(
                &base,
                &crate::route::ParamSet::from([(INPUT_NAME.into(), "bad".into())]),
            )
            .is_err()
        );

        let mut configured = base;
        configured.insert(Str::from("site"), Value::Str(Str::from("Aster")));
        assert!(
            with_route_params(
                &configured,
                &crate::route::ParamSet::from([("site".into(), "other".into())]),
            )
            .is_err()
        );
    }
}
