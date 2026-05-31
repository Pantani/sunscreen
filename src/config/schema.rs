//! Strongly-typed schema for `sunscreen.yml` (v1).
//!
//! All structs use `#[serde(deny_unknown_fields)]` so that drift in user
//! configuration surfaces as a hard error instead of being silently ignored.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_version() -> u32 {
    1
}

/// Top-level configuration document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub project: ProjectCfg,
    #[serde(default)]
    pub toolchain: ToolchainCfg,
    #[serde(default)]
    pub scaffolding: ScaffoldingCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            project: ProjectCfg::default(),
            toolchain: ToolchainCfg::default(),
            scaffolding: ScaffoldingCfg::default(),
        }
    }
}

/// Project metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectCfg {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Toolchain requirements. Map of tool name -> minimum semver requirement.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolchainCfg {
    #[serde(default)]
    pub required: BTreeMap<String, String>,
}

/// Scaffolding / template defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScaffoldingCfg {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_template: Option<String>,
}
