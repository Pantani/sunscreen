//! Minimal trust boundary for plugin execution.

use std::path::{Component, Path, PathBuf};

use crate::config::{PluginCapabilities, PluginFilesystemScope};
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

    pub fn prepare_for_workspace_execution(&self, plugin_name: &str) -> Result<(), SunscreenError> {
        self.require_filesystem_scope(plugin_name, PluginFilesystemScope::Workspace)?;
        self.require_filesystem_scope(plugin_name, PluginFilesystemScope::Scratch)?;
        self.require_network_capability(plugin_name)?;
        self.prepare()
    }

    fn prepare(&self) -> Result<(), SunscreenError> {
        std::fs::create_dir_all(&self.scratch_dir).map_err(|err| {
            SunscreenError::PluginRuntime(format!(
                "create plugin scratch dir {}: {err}",
                self.scratch_dir.display()
            ))
        })
    }

    fn require_filesystem_scope(
        &self,
        plugin_name: &str,
        scope: PluginFilesystemScope,
    ) -> Result<(), SunscreenError> {
        if self.capabilities.filesystem.contains(&scope) {
            return Ok(());
        }
        Err(SunscreenError::UserInput(format!(
            "plugin {plugin_name:?} cannot run without declaring filesystem capability `{}`",
            filesystem_scope_name(scope)
        )))
    }

    fn require_network_capability(&self, plugin_name: &str) -> Result<(), SunscreenError> {
        if self.capabilities.network {
            return Ok(());
        }
        Err(SunscreenError::UserInput(format!(
            "plugin {plugin_name:?} cannot run without declaring capability `network`; current plugin transports run local processes and cannot disable host networking"
        )))
    }
}

fn filesystem_scope_name(scope: PluginFilesystemScope) -> &'static str {
    match scope {
        PluginFilesystemScope::Workspace => "workspace",
        PluginFilesystemScope::Scratch => "scratch",
    }
}
