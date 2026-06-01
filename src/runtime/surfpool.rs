//! Surfpool runtime adapter.

use std::path::Path;

use super::subprocess::CommandSpec;
use super::validator::{Runtime, RuntimeEndpoints, RuntimePorts};

/// Managed Surfpool local runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfpoolRuntime {
    ports: RuntimePorts,
}

impl SurfpoolRuntime {
    /// Create a Surfpool runtime with explicit ports.
    #[must_use]
    pub fn new(ports: RuntimePorts) -> Self {
        Self { ports }
    }
}

impl Runtime for SurfpoolRuntime {
    fn name(&self) -> &'static str {
        "surfpool"
    }

    fn command(&self, workspace_root: &Path) -> CommandSpec {
        CommandSpec::new("surfpool")
            .arg("start")
            .arg("--port")
            .arg(self.ports.rpc.to_string())
            .arg("--ws-port")
            .arg(self.ports.ws.to_string())
            .cwd(workspace_root)
    }

    fn endpoints(&self) -> RuntimeEndpoints {
        RuntimeEndpoints::localhost(self.ports)
    }
}
