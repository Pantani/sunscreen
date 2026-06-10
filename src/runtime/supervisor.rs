//! Minimal process supervisor for local Solana runtimes.

use std::io;
use std::path::{Path, PathBuf};

use super::validator::Runtime;
use crate::process::{ManagedProcess, ProcessError, ProcessSpawner};

/// Runtime process start metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStartReport {
    /// Runtime label.
    pub runtime: &'static str,
    /// Operating-system process id.
    pub pid: u32,
    /// JSON-RPC endpoint.
    pub rpc_endpoint: String,
    /// WebSocket endpoint.
    pub ws_endpoint: String,
}

/// Errors from runtime supervision.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeSupervisorError {
    /// The runtime is already started.
    #[error("runtime already started")]
    AlreadyStarted,
    /// The process could not be spawned.
    #[error("start runtime: {0}")]
    Start(#[from] ProcessError),
    /// The process could not be stopped cleanly.
    #[error("stop runtime: {0}")]
    Stop(io::Error),
}

/// Owns one local runtime process.
pub struct RuntimeSupervisor<R: Runtime> {
    runtime: R,
    workspace_root: PathBuf,
    child: Option<Box<dyn ManagedProcess>>,
}

impl<R: Runtime> RuntimeSupervisor<R> {
    /// Create a supervisor for a runtime/workspace pair.
    #[must_use]
    pub fn new(runtime: R, workspace_root: impl AsRef<Path>) -> Self {
        Self {
            runtime,
            workspace_root: workspace_root.as_ref().to_path_buf(),
            child: None,
        }
    }

    /// Start the runtime process.
    pub fn start<S: ProcessSpawner>(
        &mut self,
        spawner: &S,
    ) -> Result<RuntimeStartReport, RuntimeSupervisorError> {
        if self.child.is_some() {
            return Err(RuntimeSupervisorError::AlreadyStarted);
        }
        let command = self.runtime.command(&self.workspace_root);
        let child = spawner.spawn(command)?;
        let pid = child.id();
        let endpoints = self.runtime.endpoints();
        self.child = Some(child);
        Ok(RuntimeStartReport {
            runtime: self.runtime.name(),
            pid,
            rpc_endpoint: endpoints.rpc,
            ws_endpoint: endpoints.ws,
        })
    }

    /// Stop the runtime process if it is running.
    pub fn stop(&mut self) -> Result<(), RuntimeSupervisorError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        child.stop().map_err(RuntimeSupervisorError::Stop)
    }
}
