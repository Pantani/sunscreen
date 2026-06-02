//! `sunscreen-plugin.json` parser and validation.

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{PluginCapabilities, PluginTransport};
use crate::error::SunscreenError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub transport: PluginTransport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoint: Vec<String>,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<PluginCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engines: Option<PluginEngines>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginEngines {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunscreen: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginCommand {
    pub name: String,
    pub kind: PluginCommandKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginCommandKind {
    App,
    Scaffold,
}

impl PluginManifest {
    pub fn read(path: &Path) -> Result<Self, SunscreenError> {
        let raw = std::fs::read_to_string(path).map_err(|err| {
            SunscreenError::ConfigInvalid(format!("read plugin manifest {}: {err}", path.display()))
        })?;
        let manifest: Self = serde_json::from_str(&raw).map_err(|err| {
            SunscreenError::ConfigInvalid(format!(
                "parse plugin manifest {}: {err}",
                path.display()
            ))
        })?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    pub fn validate(&self, path: &Path) -> Result<(), SunscreenError> {
        if self.name.trim().is_empty() {
            return Err(SunscreenError::ConfigInvalid(format!(
                "plugin manifest {} has empty name",
                path.display()
            )));
        }
        let stripped = self.version.strip_prefix('v').unwrap_or(&self.version);
        if semver::Version::parse(stripped).is_err() {
            return Err(SunscreenError::ConfigInvalid(format!(
                "plugin manifest {} has invalid version {:?}",
                path.display(),
                self.version
            )));
        }
        if self.entrypoint.iter().any(|item| item.trim().is_empty()) {
            return Err(SunscreenError::ConfigInvalid(format!(
                "plugin manifest {} has an empty entrypoint item",
                path.display()
            )));
        }
        for command in &self.commands {
            if command.name.trim().is_empty() {
                return Err(SunscreenError::ConfigInvalid(format!(
                    "plugin manifest {} has an empty command name",
                    path.display()
                )));
            }
        }
        Ok(())
    }
}

pub fn command_kind_str(kind: PluginCommandKind) -> &'static str {
    match kind {
        PluginCommandKind::App => "app",
        PluginCommandKind::Scaffold => "scaffold",
    }
}
