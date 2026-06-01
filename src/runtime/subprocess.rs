//! Testable subprocess execution for runtime orchestration.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// A command plus execution context.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Program name or path.
    pub program: OsString,
    /// Command-line arguments.
    pub args: Vec<OsString>,
    /// Optional working directory.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables.
    pub env: Vec<(OsString, OsString)>,
}

impl CommandSpec {
    /// Create a command spec for a program name or path.
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    /// Add one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set the working directory.
    #[must_use]
    pub fn cwd(mut self, cwd: impl AsRef<Path>) -> Self {
        self.cwd = Some(cwd.as_ref().to_path_buf());
        self
    }

    /// Add or override one environment variable for the child process.
    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Render the command as a string vector for JSON/log events.
    #[must_use]
    pub fn display_argv(&self) -> Vec<String> {
        std::iter::once(self.program.as_os_str())
            .chain(self.args.iter().map(OsString::as_os_str))
            .map(|part| part.to_string_lossy().into_owned())
            .collect()
    }
}

/// Captured subprocess result. Non-zero exits are represented here instead of
/// as errors so the caller can decide how to map tool-specific failures.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Process exit code, or 1 if the process terminated without a numeric code.
    pub exit_code: i32,
    /// Captured stdout as UTF-8 with replacement for invalid bytes.
    pub stdout: String,
    /// Captured stderr as UTF-8 with replacement for invalid bytes.
    pub stderr: String,
    /// Runtime duration in milliseconds.
    pub duration_ms: u128,
}

impl CommandOutput {
    /// Whether the process exited with code 0.
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Error that prevented a subprocess from starting or being waited on.
#[derive(Debug)]
pub struct ProcessError {
    program: OsString,
    source: io::Error,
}

impl ProcessError {
    /// True when the executable could not be found.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        self.source.kind() == io::ErrorKind::NotFound
    }

    /// The underlying I/O error.
    #[must_use]
    pub fn source_io(&self) -> &io::Error {
        &self.source
    }

    fn new(program: &OsStr, source: io::Error) -> Self {
        Self {
            program: program.to_os_string(),
            source,
        }
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to run {}: {}",
            self.program.to_string_lossy(),
            self.source
        )
    }
}

impl std::error::Error for ProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Runs a command and captures its output.
pub trait ProcessRunner {
    /// Execute the command spec.
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError>;
}

/// Production subprocess runner backed by [`std::process::Command`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SubprocessRunner;

impl ProcessRunner for SubprocessRunner {
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError> {
        let started = Instant::now();
        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        for (key, value) in &spec.env {
            command.env(key, value);
        }

        let output = command
            .output()
            .map_err(|e| ProcessError::new(&spec.program, e))?;
        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration_ms: started.elapsed().as_millis(),
        })
    }
}
