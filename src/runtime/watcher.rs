//! Debounce core for Phase 3 file watching.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use notify::Event as NotifyEvent;

use super::pipeline::{BuildPipeline, PipelineError, PipelineOptions, PipelineReport};
use crate::process::ProcessRunner;

/// Kind of action a debounced file-change batch should trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchKind {
    /// Run the build pipeline.
    Pipeline,
}

/// A debounced batch of relevant changed paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchBatch {
    /// Stable, sorted list of changed paths.
    pub paths: Vec<PathBuf>,
    /// Action kind for this batch.
    pub kind: WatchKind,
}

/// Deterministic debounce state for filesystem events.
///
/// The real watcher can feed raw `notify` paths into this type; tests can use
/// explicit [`Instant`] values without sleeping.
#[derive(Debug, Clone)]
pub struct WatchDebouncer {
    debounce: Duration,
    pending: BTreeSet<PathBuf>,
    deadline: Option<Instant>,
}

impl WatchDebouncer {
    /// Create a new debouncer with the quiet period required before emitting.
    #[must_use]
    pub fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            pending: BTreeSet::new(),
            deadline: None,
        }
    }

    /// Observe a raw filesystem path. Relevant paths are batched and emitted
    /// only after the debounce deadline passes.
    pub fn observe(&mut self, path: impl AsRef<Path>, now: Instant) -> Option<WatchBatch> {
        let path = path.as_ref();
        if !is_pipeline_relevant(path) {
            return None;
        }
        self.pending.insert(normalize_path(path));
        self.deadline = Some(now + self.debounce);
        None
    }

    /// Observe one raw [`notify`] event.
    ///
    /// This adapter intentionally ignores the notify event kind and routes every
    /// reported path through the same path-based filter used by tests and CLI
    /// code, because editors vary widely in the precise event kind they emit.
    pub fn observe_notify_event(
        &mut self,
        event: &NotifyEvent,
        now: Instant,
    ) -> Option<WatchBatch> {
        for path in &event.paths {
            self.observe(path, now);
        }
        None
    }

    /// Emit the pending batch when the quiet period has elapsed.
    pub fn flush_due(&mut self, now: Instant) -> Option<WatchBatch> {
        let deadline = self.deadline?;
        if now < deadline || self.pending.is_empty() {
            return None;
        }
        self.deadline = None;
        let paths = std::mem::take(&mut self.pending).into_iter().collect();
        Some(WatchBatch {
            paths,
            kind: WatchKind::Pipeline,
        })
    }
}

/// Report produced when a debounced watcher batch runs the build pipeline.
#[derive(Debug, Clone)]
pub struct WatchBuildReport {
    /// File-change batch that triggered the pipeline.
    pub batch: WatchBatch,
    /// Build pipeline report for the triggered run.
    pub pipeline: PipelineReport,
}

/// Testable bridge from file-watch events to the build pipeline.
#[derive(Debug, Clone)]
pub struct WatchBuildLoop {
    workspace_root: PathBuf,
    debouncer: WatchDebouncer,
    options: PipelineOptions,
}

impl WatchBuildLoop {
    /// Create a watcher-backed build loop rooted at a sunscreen workspace.
    #[must_use]
    pub fn new(
        workspace_root: impl AsRef<Path>,
        debounce: Duration,
        options: PipelineOptions,
    ) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            debouncer: WatchDebouncer::new(debounce),
            options,
        }
    }

    /// Observe one path, relativizing absolute notify paths to the workspace.
    pub fn observe_path(&mut self, path: impl AsRef<Path>, now: Instant) -> Option<WatchBatch> {
        let path = self.workspace_relative(path.as_ref());
        self.debouncer.observe(path, now)
    }

    /// Observe one raw [`notify`] event.
    pub fn observe_notify_event(
        &mut self,
        event: &NotifyEvent,
        now: Instant,
    ) -> Option<WatchBatch> {
        for path in &event.paths {
            self.observe_path(path, now);
        }
        None
    }

    /// Run the build pipeline once a debounced batch is due.
    pub fn flush_due<R: ProcessRunner>(
        &mut self,
        now: Instant,
        runner: &R,
    ) -> Result<Option<WatchBuildReport>, PipelineError> {
        let Some(batch) = self.debouncer.flush_due(now) else {
            return Ok(None);
        };
        let pipeline =
            BuildPipeline::new(&self.workspace_root).run(runner, self.options.clone())?;
        Ok(Some(WatchBuildReport { batch, pipeline }))
    }

    fn workspace_relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.workspace_root)
            .unwrap_or(path)
            .to_path_buf()
    }
}

fn is_pipeline_relevant(path: &Path) -> bool {
    if has_ignored_component(path) {
        return false;
    }
    let components: Vec<_> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    if matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("Anchor.toml" | "sunscreen.yml" | "codama.json")
    ) {
        return true;
    }
    if is_cargo_manifest(path, &components) {
        return true;
    }
    if components.len() < 4 {
        return false;
    }
    components[0] == "programs"
        && components.contains(&"src")
        && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
}

fn is_cargo_manifest(path: &Path, components: &[&str]) -> bool {
    if !matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("Cargo.toml" | "Cargo.lock")
    ) {
        return false;
    }
    components.len() == 1 || components.first() == Some(&"programs")
}

fn has_ignored_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(part)
                if matches!(part.to_str(), Some(".git" | "target" | "node_modules"))
        )
    })
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir => out.push(".."),
            Component::RootDir | Component::Prefix(_) => out.push(component.as_os_str()),
        }
    }
    out
}
