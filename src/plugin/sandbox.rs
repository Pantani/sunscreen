//! Minimal trust boundary for plugin execution.

use std::path::{Component, Path, PathBuf};

use crate::config::PluginCapabilities;
use crate::error::SunscreenError;

#[derive(Debug, Clone)]
pub struct PluginSandbox {
    pub workspace_root: PathBuf,
    pub scratch_dir: PathBuf,
    pub capabilities: PluginCapabilities,
}

impl PluginSandbox {
    pub fn new(
        workspace_root: impl AsRef<Path>,
        plugin_name: &str,
        capabilities: PluginCapabilities,
    ) -> Self {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let scratch_dir = workspace_root
            .join(".sunscreen")
            .join("plugins")
            .join(plugin_name);
        Self {
            workspace_root,
            scratch_dir,
            capabilities,
        }
    }

    pub fn ensure_entrypoint_allowed(&self, entrypoint: &str) -> Result<(), SunscreenError> {
        if entrypoint.trim().is_empty() {
            return Err(SunscreenError::PluginRuntime(
                "plugin entrypoint must not be empty".to_string(),
            ));
        }
        let path = Path::new(entrypoint);
        if path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(SunscreenError::PluginRuntime(format!(
                "plugin entrypoint {entrypoint:?} escapes its plugin directory"
            )));
        }
        Ok(())
    }

    pub fn prepare(&self) -> Result<(), SunscreenError> {
        std::fs::create_dir_all(&self.scratch_dir).map_err(|err| {
            SunscreenError::PluginRuntime(format!(
                "create plugin scratch dir {}: {err}",
                self.scratch_dir.display()
            ))
        })
    }
}
