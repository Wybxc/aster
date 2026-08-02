use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use aster::{BuildSession, FilesystemDependency};
use notify_debouncer_full::notify::{
    Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
    event::{ModifyKind, RenameMode},
};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, FileIdCache, RecommendedCache,
    new_debouncer_opt,
};

use crate::cli::{diag, resolve_project};

pub fn run(project_dir: Option<PathBuf>) -> Result<()> {
    let project = resolve_project(project_dir)?;
    let mut session = BuildSession::new(project.clone());
    let mut watcher = Watcher::new().context("failed to initialize file watcher")?;
    diag::emit_watching(project.root());

    loop {
        let result = session.build();
        match result {
            Ok(outcome) => diag::report_build(&outcome),
            Err(error) => diag::emit_error(&format!("{error:#}")),
        }

        watcher
            .replace(session.dependencies())
            .context("failed to update watched inputs")?;
        watcher
            .wait()
            .context("failed while watching project inputs")?;
        diag::emit_rebuilding();
    }
}

struct Watcher<T: NotifyWatcher = RecommendedWatcher, C: FileIdCache = RecommendedCache> {
    debouncer: Debouncer<T, C>,
    events: Receiver<DebounceEventResult>,
    watched: BTreeMap<PathBuf, RecursiveMode>,
    missing: BTreeSet<PathBuf>,
}

const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(100);
const POLL_INTERVAL: Duration = Duration::from_millis(300);

impl Watcher<RecommendedWatcher, RecommendedCache> {
    fn new() -> Result<Self> {
        let (sender, events) = std::sync::mpsc::channel();
        let config = Config::default().with_poll_interval(POLL_INTERVAL);
        let debouncer = new_debouncer_opt::<_, RecommendedWatcher, RecommendedCache>(
            DEBOUNCE_TIMEOUT,
            None,
            sender,
            RecommendedCache::new(),
            config,
        )?;
        Ok(Self::from_debouncer(debouncer, events))
    }
}

impl<T: NotifyWatcher, C: FileIdCache> Watcher<T, C> {
    fn from_debouncer(debouncer: Debouncer<T, C>, events: Receiver<DebounceEventResult>) -> Self {
        Self {
            debouncer,
            events,
            watched: BTreeMap::new(),
            missing: BTreeSet::new(),
        }
    }

    fn replace(
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

        let obsolete = self
            .watched
            .iter()
            .filter(|&(path, mode)| desired.get(path) != Some(mode))
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        for path in obsolete {
            // Backends may implicitly remove a watch when its path is deleted.
            self.debouncer.unwatch(&path).ok();
            self.watched.remove(&path);
        }

        for (path, mode) in desired {
            if self.watched.get(&path) == Some(&mode) {
                continue;
            }
            self.debouncer
                .watch(&path, mode)
                .with_context(|| format!("failed to watch {}", path.display()))?;
            self.watched.insert(path, mode);
        }

        Ok(())
    }

    fn wait(&mut self) -> Result<()> {
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
                    self.forget_removed_watches(&events);
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

    fn forget_removed_watches(&mut self, events: &[DebouncedEvent]) {
        for event in events {
            let removed = match event.kind {
                EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    event.paths.as_slice()
                }
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                    event.paths.first().map(std::slice::from_ref).unwrap_or(&[])
                }
                _ => continue,
            };
            for path in removed {
                if self.watched.remove(path).is_some() {
                    self.debouncer.unwatch(path).ok();
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
    use notify_debouncer_full::NoCache;
    use notify_debouncer_full::notify::PollWatcher;

    fn polling_watcher(
        dependencies: impl IntoIterator<Item = FilesystemDependency>,
    ) -> Watcher<PollWatcher, NoCache> {
        let (sender, events) = std::sync::mpsc::channel();
        let config = Config::default()
            .with_poll_interval(Duration::from_millis(50))
            .with_compare_contents(true);
        let debouncer = new_debouncer_opt(
            Duration::from_millis(50),
            None,
            sender,
            NoCache::new(),
            config,
        )
        .unwrap();
        let mut watcher = Watcher::from_debouncer(debouncer, events);
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
    fn dependency_kinds_select_watch_modes() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("aster.toml");
        let tree = temp.path().join("src");
        std::fs::write(&file, "").unwrap();
        std::fs::create_dir(&tree).unwrap();
        let mut watcher = polling_watcher([]);

        watcher
            .replace([
                FilesystemDependency::File(file.clone()),
                FilesystemDependency::Tree(tree.clone()),
            ])
            .unwrap();

        assert_eq!(
            watcher.watched.get(&file),
            Some(&RecursiveMode::NonRecursive)
        );
        assert_eq!(watcher.watched.get(&tree), Some(&RecursiveMode::Recursive));
    }

    #[test]
    fn recursively_watches_nested_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let mut watcher = polling_watcher([FilesystemDependency::Tree(root)]);
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
        let mut watcher = polling_watcher([FilesystemDependency::File(file.clone())]);
        let writer = change_repeatedly(file);

        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
        writer.join().unwrap();
    }

    #[test]
    fn polls_missing_paths_until_they_exist() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("content");
        let mut watcher = polling_watcher([FilesystemDependency::Tree(missing.clone())]);
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
        let mut watcher = polling_watcher([FilesystemDependency::Tree(missing.clone())]);

        std::fs::create_dir(&missing).unwrap();

        watcher
            .wait_until(Some(Instant::now() + Duration::from_secs(5)))
            .unwrap();
    }
}
