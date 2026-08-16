//! A package downloader that speaks rustls instead of system TLS.

use std::any::Any;
use std::io::{self, Read};

use typst_kit::downloader::Downloader;

/// A minimal HTTPS downloader for Typst Universe packages.
///
/// This replaces [`typst_kit::downloader::SystemDownloader`], whose native-tls
/// backend pulls in `openssl-sys` and cannot cross-compile without an OpenSSL
/// toolchain for the target. rustls needs no system library.
pub struct RustlsDownloader {
    agent: ureq::Agent,
}

impl RustlsDownloader {
    /// Create a downloader with the given user agent.
    pub fn new(user_agent: &str) -> Self {
        Self {
            agent: ureq::Agent::new_with_config(
                ureq::Agent::config_builder()
                    .user_agent(user_agent)
                    .https_only(true)
                    .build(),
            ),
        }
    }
}

impl Downloader for RustlsDownloader {
    fn stream(&self, _: &dyn Any, url: &str) -> io::Result<(Option<usize>, Box<dyn Read>)> {
        let response = self.agent.get(url).call().map_err(|error| match error {
            ureq::Error::StatusCode(404) => io::Error::new(io::ErrorKind::NotFound, error),
            ureq::Error::StatusCode(_) => io::Error::new(io::ErrorKind::UnexpectedEof, error),
            _ => io::Error::other(error),
        })?;
        let hint = response
            .headers()
            .get("Content-Length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());
        let reader = response.into_body().into_reader();
        Ok((hint, Box::new(reader)))
    }
}
