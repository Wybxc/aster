use anyhow::{Context, Result};
use lumis_core::events::HighlightEvent;
use lumis_wasm_runtime::{HttpFetcher, LanguageStore, Runtime, StoreConfig, set_compile_cache_dir};

pub struct LanguageRegistry {
    runtime: Runtime,
}

impl LanguageRegistry {
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .context("could not determine Aster's cache directory")?
            .join("aster/lumis");
        set_compile_cache_dir(cache_dir.clone());
        let store = LanguageStore::new(StoreConfig { cache_dir }, Box::new(HttpFetcher));
        let runtime = Runtime::with_worker_limit(1)
            .context("failed to initialize the syntax highlighting runtime")?
            .with_store(store);
        Ok(Self { runtime })
    }

    pub fn highlight(&self, source: &str, language: &str) -> Result<Option<Vec<HighlightEvent>>> {
        let Some(language) = lumis_wasm_runtime::catalog::find(language) else {
            return Ok(None);
        };
        self.runtime
            .highlight(source, language.id, false)
            .map(Some)
            .with_context(|| format!("failed to highlight {} code", language.id))
    }
}
