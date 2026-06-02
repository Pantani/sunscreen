//! gRPC transport contract for compiled plugins.
//!
//! Phase 6 finalizes the manifest/protocol surface for gRPC plugins. The CLI
//! routes command descriptors through the same manager used by stdio plugins;
//! actual gRPC process supervision is represented here as a typed adapter
//! boundary so future tonic wiring has one place to land.

use serde_json::json;

use crate::error::SunscreenError;

pub const PROTO: &str = include_str!("../../proto/plugin.proto");

pub fn contract_summary() -> serde_json::Value {
    json!({
        "transport": "grpc",
        "proto": "proto/plugin.proto",
        "service": "sunscreen.plugin.v1.Plugin",
        "operations": ["initialize", "capabilities", "run_command", "run_hook", "shutdown"],
    })
}

pub fn run_command(plugin_name: &str) -> Result<serde_json::Value, SunscreenError> {
    Err(SunscreenError::PluginRuntime(format!(
        "plugin {plugin_name} uses gRPC; start the supervised tonic endpoint before running commands"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grpc_contract_exposes_required_operations() {
        let summary = contract_summary();
        assert_eq!(summary["transport"], "grpc");
        assert!(PROTO.contains("rpc Initialize"));
        assert!(PROTO.contains("rpc Capabilities"));
        assert!(PROTO.contains("rpc RunCommand"));
        assert!(PROTO.contains("rpc RunHook"));
        assert!(PROTO.contains("rpc Shutdown"));
    }
}
