//! Code generation surfaces for Phase 4.

use std::io;
use std::path::{Component, Path, PathBuf};

pub mod codama;
pub mod codama_config;
pub mod frontend_hooks;
pub mod idl;

/// Error raised by code generation helpers before the CLI maps it to a
/// process-level [`crate::SunscreenError`].
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// User-facing invalid input or missing generated precondition.
    #[error("invalid input: {0}")]
    UserInput(String),
    /// Workspace discovery/configuration failed.
    #[error(transparent)]
    Workspace(#[from] crate::workspace::WorkspaceError),
    /// JSON parsing/serialization failed.
    #[error("json error at {}: {source}", path.display())]
    Json {
        /// File being parsed or written.
        path: PathBuf,
        /// Underlying JSON error.
        source: serde_json::Error,
    },
    /// Filesystem failure.
    #[error("I/O error at {}: {source}", path.display())]
    Io {
        /// File being read/written.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// Subprocess failure before the command could complete.
    #[error(transparent)]
    Process(#[from] crate::runtime::subprocess::ProcessError),
}

impl CodegenError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.into(),
            source,
        }
    }
}

/// Result of writing one generated file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWrite {
    /// Absolute path written or checked.
    pub path: PathBuf,
    /// True when the file contents changed.
    pub changed: bool,
}

pub(crate) fn write_if_changed(path: &Path, contents: &str) -> Result<FileWrite, CodegenError> {
    if let Ok(existing) = std::fs::read_to_string(path) {
        if existing == contents {
            return Ok(FileWrite {
                path: path.to_path_buf(),
                changed: false,
            });
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| CodegenError::io(parent, err))?;
    }
    std::fs::write(path, contents).map_err(|err| CodegenError::io(path, err))?;
    Ok(FileWrite {
        path: path.to_path_buf(),
        changed: true,
    })
}

/// Render a path relative to `workspace_root` using forward slashes.
#[must_use]
pub fn relative_path(workspace_root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(workspace_root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

pub(crate) fn sorted_json_files(dir: &Path) -> Result<Vec<PathBuf>, CodegenError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|err| CodegenError::io(dir, err))? {
        let entry = entry.map_err(|err| CodegenError::io(dir, err))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub(crate) fn ensure_safe_relative_subpath(label: &str, path: &Path) -> Result<(), CodegenError> {
    let mut has_component = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => has_component = true,
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(CodegenError::UserInput(format!(
                    "{label} must be a non-empty relative path inside the workspace"
                )));
            }
        }
    }
    if !has_component {
        return Err(CodegenError::UserInput(format!(
            "{label} must be a non-empty relative path inside the workspace"
        )));
    }
    Ok(())
}
