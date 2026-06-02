//! Unified error type for sunscreen.

use thiserror::Error;

/// Top-level error type returned at the `main` boundary.
#[derive(Debug, Error)]
pub enum SunscreenError {
    /// Configuration file is malformed or semantically invalid.
    #[error("invalid configuration: {0}")]
    ConfigInvalid(String),

    /// A required toolchain component (solana, anchor, rustc, ...) is missing.
    #[error("missing toolchain component: {0}")]
    ToolchainMissing(String),

    /// User-supplied input (flag value, argument) is invalid.
    #[error("invalid input: {0}")]
    UserInput(String),

    /// Operation requires a sunscreen workspace, but none was discovered.
    #[error("workspace not found: {0}")]
    WorkspaceMissing(String),

    /// A target path already exists and sunscreen cannot safely overwrite it.
    #[error("path conflict: {0}")]
    PathConflict(String),

    /// Network or RPC operation failed after the user explicitly requested it.
    #[error("network operation failed: {0}")]
    Network(String),

    /// A plugin process, manifest, or transport failed after discovery.
    #[error("plugin runtime failed: {0}")]
    PluginRuntime(String),

    /// An existing instruction file has no recognizable auto-generated markers,
    /// so sunscreen cannot safely re-apply the scaffold (the file may have been
    /// hand-written or its markers removed/corrupted).
    #[error("instruction drift at {path}: {hint}")]
    InstructionDrift {
        /// Path (workspace-relative) of the drifted file.
        path: String,
        /// Human-readable hint about how to resolve.
        hint: String,
    },

    /// Fallback for arbitrary errors propagated via `anyhow`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SunscreenError {
    /// Map this error to its process exit code.
    ///
    /// - `1` generic
    /// - `2` toolchain/precondition missing
    /// - `3` config invalid
    /// - `4` user input invalid
    /// - `5` workspace missing
    /// - `6` instruction drift
    /// - `7` path conflict
    /// - `8` network / RPC failure
    /// - `9` plugin runtime failure
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            SunscreenError::Other(_) => 1,
            SunscreenError::ToolchainMissing(_) => 2,
            SunscreenError::ConfigInvalid(_) => 3,
            SunscreenError::UserInput(_) => 4,
            SunscreenError::WorkspaceMissing(_) => 5,
            SunscreenError::InstructionDrift { .. } => 6,
            SunscreenError::PathConflict(_) => 7,
            SunscreenError::Network(_) => 8,
            SunscreenError::PluginRuntime(_) => 9,
        }
    }

    /// Stable string discriminant for JSON output.
    #[must_use]
    pub fn kind_str(&self) -> &'static str {
        match self {
            SunscreenError::ConfigInvalid(_) => "config_invalid",
            SunscreenError::ToolchainMissing(_) => "toolchain_missing",
            SunscreenError::UserInput(_) => "user_input",
            SunscreenError::WorkspaceMissing(_) => "workspace_missing",
            SunscreenError::InstructionDrift { .. } => "instruction_drift",
            SunscreenError::PathConflict(_) => "path_conflict",
            SunscreenError::Network(_) => "network",
            SunscreenError::PluginRuntime(_) => "plugin_runtime",
            SunscreenError::Other(_) => "other",
        }
    }

    /// Human- and agent-readable remediation hint.
    #[must_use]
    pub fn next_step(&self) -> Option<&'static str> {
        Some(match self {
            SunscreenError::ConfigInvalid(_) => {
                "open sunscreen.yml, fix the reported field, then run `sunscreen doctor --json`"
            }
            SunscreenError::ToolchainMissing(_) => "run `sunscreen doctor` to see install hints",
            SunscreenError::UserInput(_) => "run the same command with `--help`",
            SunscreenError::WorkspaceMissing(_) => {
                "run `sunscreen init <name>` or change into a sunscreen workspace"
            }
            SunscreenError::PathConflict(_) => {
                "choose a different output path or remove the existing file/directory"
            }
            SunscreenError::Network(_) => {
                "check your RPC URL, wallet balance, and network connectivity, then retry"
            }
            SunscreenError::PluginRuntime(_) => {
                "run `sunscreen app commands --json`, inspect the plugin manifest, then retry"
            }
            SunscreenError::InstructionDrift { .. } => "run `sunscreen chain doctor --fix-markers`",
            SunscreenError::Other(_) => "rerun with `-v` and inspect the error context",
        })
    }
}
