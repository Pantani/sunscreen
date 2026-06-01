//! Build pipeline orchestration for Phase 3.

use std::path::{Path, PathBuf};

use super::render_event_path;
use super::subprocess::{CommandOutput, CommandSpec, ProcessError, ProcessRunner};

/// One subprocess-backed step in the build pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStep {
    /// `anchor build`
    AnchorBuild,
    /// `pnpm exec codama run`
    CodamaRun,
}

impl PipelineStep {
    /// Stable label for logs and JSON events.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnchorBuild => "anchor_build",
            Self::CodamaRun => "codama_run",
        }
    }

    fn command(self, cwd: &Path) -> CommandSpec {
        match self {
            Self::AnchorBuild => CommandSpec::new("anchor").arg("build").cwd(cwd),
            Self::CodamaRun => CommandSpec::new("pnpm")
                .arg("exec")
                .arg("codama")
                .arg("run")
                .cwd(cwd),
        }
    }
}

/// Build pipeline options.
#[derive(Debug, Clone, Copy)]
pub struct PipelineOptions {
    /// Run `pnpm exec codama run` after a successful `anchor build`.
    pub run_codama: bool,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self { run_codama: true }
    }
}

/// One event emitted by the build pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEvent {
    /// Event name (`command_started` or `command_finished`).
    pub event: &'static str,
    /// Pipeline step that produced the event.
    pub step: PipelineStep,
    /// Full command argv.
    pub command: Vec<String>,
    /// Working directory for the command.
    pub cwd: String,
    /// Optional status for finished events.
    pub status: Option<String>,
    /// Optional exit code for finished events.
    pub exit_code: Option<i32>,
    /// Optional duration for finished events.
    pub duration_ms: Option<u128>,
    /// Optional captured stdout for finished events.
    pub stdout: Option<String>,
    /// Optional captured stderr for finished events.
    pub stderr: Option<String>,
}

impl PipelineEvent {
    fn started(step: PipelineStep, command: &CommandSpec, cwd: &Path) -> Self {
        Self {
            event: "command_started",
            step,
            command: command.display_argv(),
            cwd: render_event_path(cwd),
            status: None,
            exit_code: None,
            duration_ms: None,
            stdout: None,
            stderr: None,
        }
    }

    fn finished(
        step: PipelineStep,
        command: &CommandSpec,
        cwd: &Path,
        output: &CommandOutput,
    ) -> Self {
        Self {
            event: "command_finished",
            step,
            command: command.display_argv(),
            cwd: render_event_path(cwd),
            status: Some(if output.success() { "ok" } else { "failed" }.into()),
            exit_code: Some(output.exit_code),
            duration_ms: Some(output.duration_ms),
            stdout: Some(output.stdout.clone()),
            stderr: Some(output.stderr.clone()),
        }
    }

    /// Render this event as a JSON object for headless output.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let mut payload = serde_json::json!({
            "event": self.event,
            "step": self.step.as_str(),
            "command": self.command,
            "cwd": self.cwd,
        });
        if let serde_json::Value::Object(obj) = &mut payload {
            if let Some(status) = &self.status {
                obj.insert("status".into(), serde_json::json!(status));
            }
            if let Some(exit_code) = self.exit_code {
                obj.insert("exit_code".into(), serde_json::json!(exit_code));
            }
            if let Some(duration_ms) = self.duration_ms {
                obj.insert("duration_ms".into(), serde_json::json!(duration_ms));
            }
            if let Some(stdout) = &self.stdout {
                obj.insert("stdout".into(), serde_json::json!(stdout));
            }
            if let Some(stderr) = &self.stderr {
                obj.insert("stderr".into(), serde_json::json!(stderr));
            }
        }
        payload
    }
}

/// Completed pipeline result.
#[derive(Debug, Clone)]
pub struct PipelineReport {
    /// Events emitted while running the subprocess pipeline.
    pub events: Vec<PipelineEvent>,
    /// Pipeline exit code.
    pub exit_code: i32,
}

impl PipelineReport {
    /// Whether every requested step completed successfully.
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Error while starting or waiting for a pipeline step.
#[derive(Debug)]
pub struct PipelineError {
    /// Step that failed to start.
    pub step: PipelineStep,
    /// Underlying subprocess error.
    pub source: ProcessError,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.step.as_str(), self.source)
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Build pipeline rooted at one sunscreen workspace.
#[derive(Debug, Clone)]
pub struct BuildPipeline {
    workspace_root: PathBuf,
}

impl BuildPipeline {
    /// Create a build pipeline for a workspace root.
    #[must_use]
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Run the build pipeline with the provided subprocess runner.
    pub fn run<R: ProcessRunner>(
        &self,
        runner: &R,
        options: PipelineOptions,
    ) -> Result<PipelineReport, PipelineError> {
        let mut events = Vec::new();
        let mut steps = vec![PipelineStep::AnchorBuild];
        if options.run_codama {
            steps.push(PipelineStep::CodamaRun);
        }

        for step in steps {
            let command = step.command(&self.workspace_root);
            events.push(PipelineEvent::started(step, &command, &self.workspace_root));
            let output = runner
                .run(command.clone())
                .map_err(|source| PipelineError { step, source })?;
            events.push(PipelineEvent::finished(
                step,
                &command,
                &self.workspace_root,
                &output,
            ));
            if !output.success() {
                return Ok(PipelineReport {
                    events,
                    exit_code: output.exit_code,
                });
            }
        }

        Ok(PipelineReport {
            events,
            exit_code: 0,
        })
    }
}
