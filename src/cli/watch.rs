use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use aster::{BuildSession, FilesystemDependency};
use notify_debouncer_full::notify::{
    Config, EventKind, RecommendedWatcher, RecursiveMode, event::ModifyKind,
};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer_opt};

use crate::cli::{resolve_project, telemetry};

pub fn run(project_dir: Option<PathBuf>) -> Result<()> {
    let project = resolve_project(project_dir)?;
    let mut session = BuildSession::new(project.clone());
    let mut watcher = Watcher::new().context("failed to initialize file watcher")?;
    tracing::info!(
        project = %project.root().display(),
        "watching project"
    );

    loop {
        let result = session.build();
        match result {
            Ok(outcome) => telemetry::report_build(&outcome),
            Err(error) => tracing::error!(
                error = %format_args!("{error:#}"),
                "build failed: {error:#}"
            ),
        }

        watcher
            .replace(session.dependencies())
            .context("failed to update watched inputs")?;
        watcher
            .wait()
            .context("failed while watching project inputs")?;
        tracing::info!(reason = "change detected", "rebuilding after a change");
    }
}

pub struct Watcher {
    debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    events: Receiver<DebounceEventResult>,
    watched: BTreeMap<PathBuf, RecursiveMode>,
    missing: BTreeSet<PathBuf>,
}

const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(100);
const POLL_INTERVAL: Duration = Duration::from_millis(300);

impl Watcher {
    pub fn new() -> Result<Self> {
        let (sender, events) = std::sync::mpsc::channel();
        let config = Config::default().with_poll_interval(POLL_INTERVAL);
        let debouncer = new_debouncer_opt::<_, RecommendedWatcher, RecommendedCache>(
            DEBOUNCE_TIMEOUT,
            None,
            sender,
            RecommendedCache::new(),
            config,
        )?;
        Ok(Self {
            debouncer,
            events,
            watched: BTreeMap::new(),
            missing: BTreeSet::new(),
        })
    }

    pub fn replace(
        &mut self,
        dependencies: impl IntoIterator<Item = FilesystemDependency>,
    ) -> Result<()> {
        let mut desired = BTreeMap::new();
        let mut missing = BTreeSet::new();
        for dependency in dependencies {
            let (path, mode) = match dependency {
                FilesystemDependency::File(path) => (path, RecursiveMode::NonRecursive),
                FilesystemDependency::Tree(path) => (path, RecursiveMode::Recursive),
            };
            if !path.exists() {
                missing.insert(path);
                continue;
            }
            desired
                .entry(path)
                .and_modify(|current| {
                    if mode == RecursiveMode::Recursive {
                        *current = mode;
                    }
                })
                .or_insert(mode);
        }
        self.missing = missing;

        for path in std::mem::take(&mut self.watched).into_keys() {
            // Backends may implicitly remove a watch when its path is deleted.
            self.debouncer.unwatch(&path).ok();
        }

        for (path, mode) in desired {
            self.debouncer
                .watch(&path, mode)
                .with_context(|| format!("failed to watch {}", path.display()))?;
            self.watched.insert(path, mode);
        }

        Ok(())
    }

    pub fn wait(&mut self) -> Result<()> {
        self.wait_until(None)
    }

    fn wait_until(&mut self, deadline: Option<Instant>) -> Result<()> {
        if self.missing.iter().any(|path| path.exists()) {
            return Ok(());
        }
        loop {
            let timeout = match deadline {
                Some(deadline) => {
                    let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                        bail!("timed out waiting for a file change");
                    };
                    Some(if self.missing.is_empty() {
                        remaining
                    } else {
                        POLL_INTERVAL.min(remaining)
                    })
                }
                None if self.missing.is_empty() => None,
                None => Some(POLL_INTERVAL),
            };
            let received = match timeout {
                Some(timeout) => self.events.recv_timeout(timeout),
                None => self
                    .events
                    .recv()
                    .map_err(|_| RecvTimeoutError::Disconnected),
            };

            match received {
                Ok(Ok(events)) => {
                    if events.iter().any(|event| relevant(event.kind)) {
                        return Ok(());
                    }
                }
                Ok(Err(errors)) => {
                    let errors = errors
                        .into_iter()
                        .map(|error| error.to_string())
                        .collect::<Vec<_>>()
                        .join("; ");
                    bail!("file watcher failed: {errors}");
                }
                Err(RecvTimeoutError::Timeout) if self.missing.iter().any(|path| path.exists()) => {
                    return Ok(());
                }
                Err(RecvTimeoutError::Timeout)
                    if deadline.is_some_and(|deadline| Instant::now() >= deadline) =>
                {
                    bail!("timed out waiting for a file change");
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    bail!("file watcher event channel disconnected");
                }
            }
        }
    }
}

fn relevant(kind: EventKind) -> bool {
    match kind {
        EventKind::Any | EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Access(_) | EventKind::Other => false,
        EventKind::Modify(kind) => matches!(
            kind,
            ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Name(_)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watcher(dependencies: impl IntoIterator<Item = FilesystemDependency>) -> Watcher {
        let mut watcher = Watcher::new().unwrap();
        watcher.replace(dependencies).unwrap();
        watcher
    }

    fn change_repeatedly(path: PathBuf) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            for revision in 0..10 {
                std::thread::sleep(Duration::from_millis(100));
                std::fs::write(&path, format!("changed {revision}")).unwrap();
            }
        })
    }

    #[test]
    fn recursively_watches_nested_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let mut watcher = watcher([FilesystemDependency::Tree(root)]);
        let writer = change_repeatedly(nested.join("page.typ"));

        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn watches_files_non_recursively() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("page.typ");
        std::fs::write(&file, "initial").unwrap();
        let mut watcher = watcher([FilesystemDependency::File(file.clone())]);
        let writer = change_repeatedly(file);

        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn replaces_the_complete_watch_set() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("old.typ");
        let new = temp.path().join("new.typ");
        std::fs::write(&old, "old").unwrap();
        std::fs::write(&new, "new").unwrap();
        let mut watcher = watcher([FilesystemDependency::File(old.clone())]);

        watcher
            .replace([FilesystemDependency::File(new.clone())])
            .unwrap();
        assert!(!watcher.watched.contains_key(&old));
        assert!(watcher.watched.contains_key(&new));

        let writer = change_repeatedly(new);
        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn polls_missing_paths_until_they_exist() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("content");
        let mut watcher = watcher([FilesystemDependency::Tree(missing.clone())]);
        let created = missing.clone();
        let creator = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            std::fs::create_dir(created).unwrap();
        });

        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
        creator.join().unwrap();
    }

    #[test]
    fn notices_missing_path_created_before_waiting() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("content");
        let mut watcher = watcher([FilesystemDependency::Tree(missing.clone())]);

        std::fs::create_dir(&missing).unwrap();

        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
    }
}
