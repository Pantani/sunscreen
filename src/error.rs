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

    /// A generated artifact on disk diverges from what would be re-rendered.
    /// User edits outside designated regions, or args changed between runs.
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
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            SunscreenError::Other(_) => 1,
            SunscreenError::ToolchainMissing(_) => 2,
            SunscreenError::ConfigInvalid(_) => 3,
            SunscreenError::UserInput(_) => 4,
            SunscreenError::WorkspaceMissing(_) => 5,
            SunscreenError::InstructionDrift { .. } => 6,
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
            SunscreenError::Other(_) => "other",
        }
    }
}
