//! Configuration subsystem: schema, loader, migrator.
//!
//! The public surface is intentionally small:
//! - [`Config`] (and nested structs) describe the parsed `sunscreen.yml`.
//! - [`load`] resolves and parses configuration honoring precedence and env overlay.
//! - [`ConfigError`] is the error type returned by the loader.
//! - [`Migration`] / [`registry`] / [`migrate`] manage versioned migrations.

pub mod loader;
pub mod migrations;
pub mod migrator;
pub mod schema;

/// Current schema version supported by the loader. Documents at a lower
/// version are upgraded via [`migrator::migrate`] before deserialization.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

pub use loader::{load, ConfigError};
pub use migrator::{migrate, registry, Migration};
pub use schema::{
    ClusterCfg, ClustersCfg, Config, Framework, Frontend, PluginCapabilities, PluginCfg,
    PluginFilesystemScope, PluginTransport, ProgramCfg, ProjectCfg, RuntimeCfg, RuntimeEngine,
    ScaffoldingCfg, ToolchainCfg, ValidationError, WorkspaceCfg,
};
