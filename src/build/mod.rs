//! Build orchestration and output publication.
//!
//! This layer combines the foundation and engine layers behind [`BuildSession`].

use std::fmt;

use typst::ecow::EcoString;

use crate::foundation::FilesystemDependency;

mod output;
mod pipeline;
mod transform;
mod world;

pub use pipeline::{BuildOutcome, BuildSession};

impl BuildSession {
    /// Iterate over the inputs observed by the latest build attempt.
    pub fn dependencies(&mut self) -> impl Iterator<Item = FilesystemDependency> + use<> {
        self.session.dependencies().into_iter()
    }
}

/// A non-fatal diagnostic produced while building a project.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BuildWarning(EcoString);

impl BuildWarning {
    pub(crate) fn new(message: impl Into<EcoString>) -> Self {
        Self(message.into())
    }

    /// Return the warning message without presentation-specific formatting.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for BuildWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for BuildWarning {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
