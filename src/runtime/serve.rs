//! Headless serve-loop primitives for Phase 3.

use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};

use super::pipeline::{PipelineError, PipelineOptions};
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

impl NotifyWatchSource {
    /// Watch a sunscreen workspace recursively.
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, NotifyWatchError> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(workspace_root.as_ref(), RecursiveMode::Recursive)?;
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
            .map(|path| path.display().to_string())
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
