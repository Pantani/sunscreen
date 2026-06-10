//! `solana-test-validator` runtime adapter.

use std::path::Path;

use super::validator::{Runtime, RuntimeEndpoints, RuntimePorts};
use crate::process::CommandSpec;

/// Managed `solana-test-validator` fallback runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct TestValidatorRuntime {
    ports: RuntimePorts,
}

impl TestValidatorRuntime {
    /// Create a test-validator runtime with explicit ports.
    #[must_use]
    pub fn new(ports: RuntimePorts) -> Self {
        Self { ports }
    }
}

impl Runtime for TestValidatorRuntime {
    fn name(&self) -> &'static str {
        "test-validator"
    }

    fn command(&self, workspace_root: &Path) -> CommandSpec {
        CommandSpec::new("solana-test-validator")
            .arg("--rpc-port")
            .arg(self.ports.rpc.to_string())
            .arg("--reset")
            .cwd(workspace_root)
    }

    fn endpoints(&self) -> RuntimeEndpoints {
        RuntimeEndpoints::localhost(self.ports)
    }
}
