//! Build orchestration and output publication.
//!
//! This layer combines the foundation and engine layers behind [`BuildSession`].

use std::fmt;

use typst::ecow::EcoString;

use crate::foundation::{FilesystemDependency, Project};

mod output;
mod pipeline;
mod transform;
mod world;

pub use pipeline::BuildOutcome;

/// A reusable build session bound to one Aster project.
pub struct BuildSession {
    session: world::TypstSession,
}

impl BuildSession {
    /// Create a reusable session bound to a validated project.
    pub fn new(project: Project) -> Self {
        Self {
            session: world::TypstSession::new(project),
        }
    }

    /// Return the inputs observed by the latest build attempt.
    pub fn dependencies(&mut self) -> Vec<FilesystemDependency> {
        self.session.dependencies()
    }

    /// Return the project bound to this session.
    pub fn project(&self) -> &Project {
        self.session.project()
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
