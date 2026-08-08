use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};
use lumis_core::events::HighlightEvent;
use lumis_wasm_runtime::{LanguageSpec, Runtime};
use serde::Deserialize;

use super::cache::{WasmCache, WasmSource};

static CATALOG: LazyLock<LanguageCatalog> = LazyLock::new(|| {
    let manifest: LanguageManifest = toml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/lumis/languages.toml"
    )))
    .expect("syntax language catalog must be valid");
    LanguageCatalog::from_manifest(manifest)
});

static QUERIES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/vendor/lumis/queries");

pub(super) struct LanguageRegistry {
    runtime: Runtime,
    cache: WasmCache,
}

impl LanguageRegistry {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            runtime: Runtime::new()
                .context("failed to initialize the syntax highlighting runtime")?,
            cache: WasmCache::system()?,
        })
    }

    pub(super) fn highlight(
        &self,
        source: &str,
        language: &str,
    ) -> Result<Option<Vec<HighlightEvent>>> {
        let Some(language) = CATALOG.resolve(language) else {
            return Ok(None);
        };
        self.load(language)?;
        self.runtime
            .highlight(source, &language.id, false)
            .map(Some)
            .with_context(|| format!("failed to highlight {} code", language.id))
    }

    fn load(&self, language: &Language) -> Result<()> {
        if self.runtime.has_language(&language.id) {
            return Ok(());
        }

        let wasm = self
            .cache
            .obtain(&language.wasm)
            .with_context(|| format!("failed to obtain the {} parser", language.id))?;
        let highlights = query(&language.queries, "highlights")
            .with_context(|| format!("missing {} highlight queries", language.id))?
            .context("Lumis language has no highlight query")?;
        let injections = query(&language.queries, "injections")?.unwrap_or_default();
        let locals = query(&language.queries, "locals")?.unwrap_or_default();
        let brackets = query(&language.queries, "brackets")?
            .or(query("default", "brackets")?)
            .unwrap_or_default();

        self.runtime
            .load_language(LanguageSpec {
                id: language.id.clone(),
                aliases: language.aliases.clone(),
                grammar_name: language.grammar.clone(),
                wasm,
                highlights: highlights.into(),
                injections: injections.into(),
                locals: locals.into(),
                brackets: brackets.into(),
            })
            .with_context(|| format!("failed to load the {} parser", language.id))
    }
}

fn query(language: &str, name: &str) -> Result<Option<&'static str>> {
    let path = format!("{language}/{name}.scm");
    QUERIES
        .get_file(&path)
        .map(|file| {
            file.contents_utf8()
                .with_context(|| format!("vendored Lumis query {path} is not UTF-8"))
        })
        .transpose()
}

struct LanguageCatalog {
    languages: BTreeMap<String, Language>,
    aliases: HashMap<String, String>,
}

impl LanguageCatalog {
    fn from_manifest(manifest: LanguageManifest) -> Self {
        let mut languages = BTreeMap::new();
        let mut aliases = HashMap::new();
        for (id, entry) in manifest.languages {
            aliases.insert(id.to_ascii_lowercase(), id.clone());
            for alias in &entry.aliases {
                aliases.insert(alias.to_ascii_lowercase(), id.clone());
            }
            languages.insert(
                id.clone(),
                Language {
                    id,
                    grammar: entry.grammar,
                    aliases: entry.aliases,
                    wasm: entry.wasm,
                    queries: entry.queries,
                },
            );
        }
        Self { languages, aliases }
    }

    fn resolve(&self, name: &str) -> Option<&Language> {
        let id = self.aliases.get(&name.to_ascii_lowercase())?;
        self.languages.get(id)
    }
}

struct Language {
    id: String,
    grammar: String,
    aliases: Vec<String>,
    wasm: WasmSource,
    queries: String,
}

#[derive(Deserialize)]
struct LanguageManifest {
    languages: BTreeMap<String, LanguageEntry>,
}

#[derive(Deserialize)]
struct LanguageEntry {
    grammar: String,
    #[serde(default)]
    aliases: Vec<String>,
    wasm: WasmSource,
    queries: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_language_aliases() {
        assert_eq!(CATALOG.resolve("SH").unwrap().id, "bash");
        assert_eq!(CATALOG.resolve("c#").unwrap().id, "csharp");
        assert_eq!(CATALOG.resolve("c_sharp").unwrap().grammar, "c_sharp");
        assert_eq!(CATALOG.resolve("proto").unwrap().id, "protobuf");
        assert_eq!(CATALOG.resolve("erb").unwrap().grammar, "embedded_template");
    }

    #[test]
    fn loads_the_language_catalog() {
        let manifest: LanguageManifest = toml::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/lumis/languages.toml"
        )))
        .unwrap();
        assert_eq!(CATALOG.languages.len(), 112);
        assert_eq!(manifest.languages.len(), CATALOG.languages.len());
    }

    #[test]
    fn uses_lumis_npm_parsers_and_vendored_queries() {
        for language in CATALOG.languages.values() {
            assert!(language.wasm.url.contains("/@lumis-sh/wasm-"));
            assert!(
                query(&language.queries, "highlights").unwrap().is_some(),
                "{} uses missing {} queries",
                language.id,
                language.queries
            );
        }
        assert!(CATALOG.resolve("typst").is_none());
        assert!(CATALOG.resolve("gdshader").is_none());
    }

    #[test]
    #[ignore = "downloads every syntax language"]
    fn every_catalog_language_loads() {
        for id in CATALOG.languages.keys() {
            let registry = LanguageRegistry::new().unwrap();
            registry
                .highlight("", id)
                .unwrap_or_else(|error| panic!("failed to load {id}: {error:#}"));
        }
    }
}
