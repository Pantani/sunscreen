//! Plugin runtime for Phase 6.
//!
//! The runtime is intentionally narrow and offline-first: workspace manifests
//! declare plugins, local manifests describe commands, and the stdio JSON-RPC
//! transport executes those commands through an adapter that can be tested
//! without a registry or network.

pub mod grpc;
pub mod manager;
pub mod manifest;
pub mod marketplace;
pub mod sandbox;
pub mod stdio;
