//! Build orchestration and output publication.
//!
//! This layer combines the foundation and engine layers behind [`BuildSession`].

use std::fmt;

use comemo::{Track, Tracked};
use typst::ecow::EcoString;
use typst_kit::datetime::Time;
use typst_kit::fonts::FontStore;

use crate::foundation::{FilesystemDependency, FontConfig, Project};

mod files;
mod output;
mod pipeline;
mod transform;
mod world;

use self::files::ProjectFiles;

pub use pipeline::BuildOutcome;

/// A reusable build session bound to one Aster project.
///
/// A session retains compiler resources and observed filesystem state across
/// builds while reloading the project manifest for every build attempt.
pub struct BuildSession {
    project: Project,
    font_config: Option<FontConfig>,
    fonts: FontStore,
    files: ProjectFiles,
    now: Time,
}

impl BuildSession {
    /// Create a reusable session bound to a validated project.
    pub fn new(project: Project) -> Self {
        let files = ProjectFiles::new(&project);
        Self {
            project,
            font_config: None,
            fonts: FontStore::new(),
            files,
            now: Time::system(),
        }
    }

    /// Return the project bound to this session.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// Return the inputs observed by the latest build attempt.
    pub fn dependencies(&mut self) -> Vec<FilesystemDependency> {
        self.files.dependencies()
    }

    fn reset(&mut self) {
        self.files.reset();
        self.now.reset();
    }

    fn project_files(&self) -> Tracked<'_, ProjectFiles> {
        self.files.track()
    }
}

/// A non-fatal diagnostic produced while building a project.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BuildWarning(EcoString);

impl BuildWarning {
    fn new(message: impl Into<EcoString>) -> Self {
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
