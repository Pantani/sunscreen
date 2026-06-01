//! Headless serve-loop primitives for Phase 3.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};

use super::pipeline::{PipelineError, PipelineOptions};
use super::render_event_path;
use super::subprocess::ProcessRunner;
use super::watcher::{WatchBuildLoop, WatchBuildReport};

/// One input consumed by the headless serve loop.
#[derive(Debug, Clone)]
pub enum ServeLoopInput {
    /// Raw filesystem event received from `notify`.
    NotifyEvent(NotifyEvent, Instant),
    /// Timer tick used to flush debounced batches.
    Tick(Instant),
}

/// Error returned by the production filesystem watcher source.
#[derive(Debug, thiserror::Error)]
pub enum NotifyWatchError {
    /// Underlying notify backend error.
    #[error("filesystem watcher error: {0}")]
    Notify(#[from] notify::Error),
    /// Watcher callback channel closed unexpectedly.
    #[error("filesystem watcher channel disconnected")]
    Disconnected,
}

/// Production filesystem event source backed by [`notify`].
pub struct NotifyWatchSource {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<NotifyEvent>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WatchRoot {
    path: PathBuf,
    mode: RecursiveMode,
}

impl NotifyWatchSource {
    /// Watch the source and config files that can trigger a rebuild.
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, NotifyWatchError> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        for root in watch_roots(workspace_root.as_ref()) {
            if root.path.exists() {
                watcher.watch(&root.path, root.mode)?;
            }
        }
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Receive the next notify event, or `Ok(None)` when the timeout elapses.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<NotifyEvent>, NotifyWatchError> {
        match self.rx.recv_timeout(timeout) {
            Ok(Ok(event)) => Ok(Some(event)),
            Ok(Err(err)) => Err(NotifyWatchError::Notify(err)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(NotifyWatchError::Disconnected),
        }
    }
}

fn watch_roots(workspace_root: &Path) -> Vec<WatchRoot> {
    let mut roots = vec![WatchRoot {
        path: workspace_root.join("programs"),
        mode: RecursiveMode::Recursive,
    }];
    roots.extend(
        ["Anchor.toml", "sunscreen.yml", "codama.json"].map(|file| WatchRoot {
            path: workspace_root.join(file),
            mode: RecursiveMode::NonRecursive,
        }),
    );
    roots
}

/// Testable headless bridge for `chain serve`.
#[derive(Debug, Clone)]
pub struct HeadlessServeLoop {
    watch_loop: WatchBuildLoop,
}

impl HeadlessServeLoop {
    /// Create a headless serve loop rooted at a sunscreen workspace.
    #[must_use]
    pub fn new(
        workspace_root: impl AsRef<Path>,
        debounce: Duration,
        options: PipelineOptions,
    ) -> Self {
        Self {
            watch_loop: WatchBuildLoop::new(workspace_root, debounce, options),
        }
    }

    /// Handle one event/tick and return JSON events that should be emitted.
    pub fn handle_input<R: ProcessRunner>(
        &mut self,
        input: ServeLoopInput,
        runner: &R,
    ) -> Result<Vec<serde_json::Value>, PipelineError> {
        match input {
            ServeLoopInput::NotifyEvent(event, now) => {
                self.watch_loop.observe_notify_event(&event, now);
                let Some(report) = self.watch_loop.flush_due(now, runner)? else {
                    return Ok(Vec::new());
                };
                Ok(render_watch_build_report(&report))
            }
            ServeLoopInput::Tick(now) => {
                let Some(report) = self.watch_loop.flush_due(now, runner)? else {
                    return Ok(Vec::new());
                };
                Ok(render_watch_build_report(&report))
            }
        }
    }
}

fn render_watch_build_report(report: &WatchBuildReport) -> Vec<serde_json::Value> {
    let status = if report.pipeline.success() {
        "ok"
    } else {
        "failed"
    };
    let mut events = Vec::with_capacity(report.pipeline.events.len() + 2);
    events.push(serde_json::json!({
        "event": "chain_serve_build_started",
        "paths": report
            .batch
            .paths
            .iter()
            .map(|path| render_event_path(path))
            .collect::<Vec<_>>(),
    }));
    events.extend(report.pipeline.events.iter().map(|event| event.to_json()));
    events.push(serde_json::json!({
        "event": "chain_serve_build_finished",
        "status": status,
        "exit_code": report.pipeline.exit_code,
    }));
    events
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use notify::RecursiveMode;

    use super::render_event_path;

    #[test]
    fn render_event_path_uses_forward_slashes() {
        assert_eq!(
            render_event_path(Path::new(r"programs\demo\src\instructions\deposit.rs")),
            "programs/demo/src/instructions/deposit.rs"
        );
    }

    #[test]
    fn notify_watch_roots_avoid_recursive_workspace_watch() {
        let root = Path::new("/tmp/sunscreen-workspace");
        let roots = super::watch_roots(root);

        assert_eq!(
            roots
                .iter()
                .find(|entry| entry.path == root.join("programs"))
                .map(|entry| entry.mode),
            Some(RecursiveMode::Recursive)
        );
        assert!(roots
            .iter()
            .any(|entry| entry.path == root.join("Anchor.toml")
                && entry.mode == RecursiveMode::NonRecursive));
        assert!(!roots
            .iter()
            .any(|entry| entry.path == root && entry.mode == RecursiveMode::Recursive));
    }
}
