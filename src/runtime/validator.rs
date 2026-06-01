//! Runtime trait and shared endpoint configuration.

use std::path::Path;

use super::subprocess::CommandSpec;

/// RPC/WebSocket port pair for a local Solana runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePorts {
    /// JSON-RPC port.
    pub rpc: u16,
    /// WebSocket port.
    pub ws: u16,
}

impl RuntimePorts {
    /// Create an explicit port pair.
    #[must_use]
    pub fn new(rpc: u16, ws: u16) -> Self {
        Self { rpc, ws }
    }
}

impl Default for RuntimePorts {
    fn default() -> Self {
        Self {
            rpc: 8899,
            ws: 8900,
        }
    }
}

/// Runtime endpoints exposed by the managed validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEndpoints {
    /// JSON-RPC endpoint.
    pub rpc: String,
    /// WebSocket endpoint.
    pub ws: String,
}

impl RuntimeEndpoints {
    /// Build localhost endpoints for a port pair.
    #[must_use]
    pub fn localhost(ports: RuntimePorts) -> Self {
        Self {
            rpc: format!("http://127.0.0.1:{}", ports.rpc),
            ws: format!("ws://127.0.0.1:{}", ports.ws),
        }
    }
}

/// Local Solana runtime implementation.
pub trait Runtime: Clone {
    /// Stable runtime label.
    fn name(&self) -> &'static str;

    /// Command used to start the runtime in the workspace.
    fn command(&self, workspace_root: &Path) -> CommandSpec;

    /// Endpoints the runtime exposes once started.
    fn endpoints(&self) -> RuntimeEndpoints;
}
