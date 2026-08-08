use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use typst_kit::downloader::{Downloader, SystemDownloader};
use url::Url;

#[derive(Deserialize)]
pub(super) struct WasmSource {
    pub(super) url: String,
    integrity: String,
}

/// On-demand parser modules stored below the operating system's cache directory.
pub(super) struct WasmCache {
    root: PathBuf,
    downloader: SystemDownloader,
}

impl WasmCache {
    pub(super) fn system() -> Result<Self> {
        let root = std::env::var_os("ASTER_LANGUAGE_CACHE_PATH")
            .map(PathBuf::from)
            .or_else(|| dirs::cache_dir().map(|path| path.join("aster/languages")))
            .context("could not determine Aster's language cache directory")?;
        Ok(Self::new(root))
    }

    fn new(root: PathBuf) -> Self {
        Self {
            root,
            downloader: SystemDownloader::new(concat!("aster/", env!("CARGO_PKG_VERSION"))),
        }
    }

    pub(super) fn obtain(&self, source: &WasmSource) -> Result<Vec<u8>> {
        let path = self.cache_path(source)?;
        if let Ok(bytes) = fs::read(&path)
            && verify_integrity(&bytes, &source.integrity).is_ok()
        {
            return Ok(bytes);
        }

        let bytes = self
            .downloader
            .download(&"syntax highlighting language file", &source.url)
            .with_context(|| format!("failed to download {}", source.url))?;
        verify_integrity(&bytes, &source.integrity)
            .with_context(|| format!("failed to verify {}", source.url))?;
        persist(&path, &bytes)?;
        Ok(bytes.to_vec())
    }

    fn cache_path(&self, source: &WasmSource) -> Result<PathBuf> {
        let url = Url::parse(&source.url)
            .with_context(|| format!("invalid language file URL {}", source.url))?;
        ensure!(url.scheme() == "https", "language file URL must use HTTPS");
        let host = url.host_str().context("language file URL has no host")?;
        let segments = url
            .path_segments()
            .context("language file URL cannot be a base URL")?;

        let mut path = self.root.join(host);
        for segment in segments.filter(|segment| !segment.is_empty()) {
            ensure!(
                segment != "." && segment != "..",
                "invalid URL path segment"
            );
            path.push(segment);
        }
        ensure!(
            path.file_name().is_some(),
            "language file URL has no filename"
        );
        Ok(path)
    }
}

fn persist(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("language cache path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create language cache {}", parent.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create a temporary file in {}", parent.display()))?;
    temp.write_all(bytes)
        .context("failed to write a language cache file")?;
    temp.persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to cache language file {}", path.display()))?;
    Ok(())
}

fn verify_integrity(bytes: &[u8], integrity: &str) -> Result<()> {
    let encoded = integrity
        .strip_prefix("sha256-")
        .context("language file does not provide a SHA-256 integrity value")?;
    let expected = STANDARD
        .decode(encoded)
        .context("invalid SHA-256 integrity value")?;
    ensure!(
        Sha256::digest(bytes).as_slice() == expected,
        "integrity mismatch"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(url: &str) -> WasmSource {
        WasmSource {
            url: url.into(),
            integrity: "sha256-uSRaW63+hKF0K04aF3zS3ygeijNFzFF98Rk2Hj0ftZY=".into(),
        }
    }

    #[test]
    fn derives_a_versioned_cache_path_from_the_url() {
        let cache = WasmCache::new("/cache".into());
        let path = cache
            .cache_path(&source(
                "https://cdn.jsdelivr.net/npm/@lumis-sh/wasm-rust@0.26.3/tree-sitter-rust.wasm",
            ))
            .unwrap();
        assert_eq!(
            path,
            Path::new(
                "/cache/cdn.jsdelivr.net/npm/@lumis-sh/wasm-rust@0.26.3/tree-sitter-rust.wasm"
            )
        );
    }

    #[test]
    fn verifies_file_integrity() {
        assert!(
            verify_integrity(
                b"aster",
                "sha256-uSRaW63+hKF0K04aF3zS3ygeijNFzFF98Rk2Hj0ftZY="
            )
            .is_ok()
        );
        assert!(verify_integrity(b"changed", "sha256-AA==").is_err());
    }
}
