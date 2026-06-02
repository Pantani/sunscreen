//! Strongly-typed schema for `sunscreen.yml` (v1).
//!
//! ## Forward-compatibility policy
//!
//! All structs use `#[serde(deny_unknown_fields)]`. The ADR §5.4 is the
//! contract — unknown keys are treated as drift and surfaced as hard errors
//! rather than silently ignored. Forward-compatibility within v1 is achieved
//! via **additive fields with `#[serde(default)]`**: a new field added to a
//! struct is invisible to older YAML documents (uses the default) but
//! understood by newer ones. Removing or repurposing a field is a v1 -> v2
//! migration, not a struct edit.
//!
//! Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`
//! so round-tripping does not introduce noisy `null`s.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_version() -> u32 {
    1
}

fn default_framework() -> Framework {
    Framework::Anchor
}

fn default_rust_edition() -> String {
    "2021".to_string()
}

fn default_frontend() -> Frontend {
    Frontend::None
}

fn default_localnet() -> ClusterCfg {
    ClusterCfg {
        url: "http://127.0.0.1:8899".to_string(),
        wallet: "~/.config/solana/id.json".to_string(),
    }
}

fn default_devnet() -> ClusterCfg {
    ClusterCfg {
        url: "https://api.devnet.solana.com".to_string(),
        wallet: "~/.config/solana/id.json".to_string(),
    }
}

fn default_mainnet() -> ClusterCfg {
    ClusterCfg {
        url: "https://api.mainnet-beta.solana.com".to_string(),
        wallet: "~/.config/solana/id.json".to_string(),
    }
}

fn default_runtime_engine() -> RuntimeEngine {
    RuntimeEngine::Surfpool
}

fn default_runtime_port() -> u16 {
    8899
}

fn default_faucet_sol() -> u64 {
    100
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Top-level configuration document for `sunscreen.yml` v1.
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub programs: Vec<ProgramCfg>,
    #[serde(default)]
    pub workspace: WorkspaceCfg,
    #[serde(default)]
    pub clusters: ClustersCfg,
    #[serde(default)]
    pub runtime: RuntimeCfg,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginCfg>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            project: ProjectCfg::default(),
            toolchain: ToolchainCfg::default(),
            scaffolding: ScaffoldingCfg::default(),
            programs: Vec::new(),
            workspace: WorkspaceCfg::default(),
            clusters: ClustersCfg::default(),
            runtime: RuntimeCfg::default(),
            plugins: Vec::new(),
        }
    }
}

impl Config {
    /// Construct a workspace-bootstrap configuration. Called by
    /// `cli-architect` / `template-engineer` when materializing a new
    /// workspace via `sunscreen chain new`.
    ///
    /// The returned config has exactly one program (`<project_name>` at
    /// `programs/<project_name>`) and the supplied frontend kind. All
    /// other fields fall back to schema defaults. The result satisfies
    /// `validate()`.
    pub fn new_for_workspace(project_name: &str, framework: Framework, frontend: Frontend) -> Self {
        let mut cfg = Self::default();
        cfg.project.name = project_name.to_string();
        cfg.project.framework = framework;
        cfg.programs.push(ProgramCfg {
            name: project_name.to_string(),
            path: format!("programs/{project_name}"),
            cluster_overrides: BTreeMap::new(),
        });
        cfg.workspace.frontend = frontend;
        cfg.workspace.frontend_path = match cfg.workspace.frontend {
            Frontend::None => None,
            _ => Some("app".to_string()),
        };
        cfg
    }

    /// Run semantic validation on a parsed config. This is intentionally
    /// distinct from serde parsing — `load()` performs structural parsing
    /// only; callers that mutate or construct configs in memory should
    /// invoke `validate()` before persisting.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.version != 1 {
            return Err(ValidationError::UnsupportedVersion(self.version));
        }
        if !self.project.name.is_empty() && !is_kebab_case(&self.project.name) {
            return Err(ValidationError::NotKebabCase {
                field: "project.name",
                value: self.project.name.clone(),
            });
        }
        if self.programs.is_empty() {
            return Err(ValidationError::EmptyPrograms);
        }
        for p in &self.programs {
            if p.name.is_empty() {
                return Err(ValidationError::EmptyProgramName);
            }
            if !is_kebab_case(&p.name) {
                return Err(ValidationError::NotKebabCase {
                    field: "programs[].name",
                    value: p.name.clone(),
                });
            }
            if p.path.is_empty() {
                return Err(ValidationError::EmptyProgramPath {
                    program: p.name.clone(),
                });
            }
        }
        // Plugin semantic validation. Phase 6 extends the original
        // declaration-only entries with optional runtime hints. A manifest
        // can still supply transport/entrypoint/capabilities, so all runtime
        // fields remain optional in `sunscreen.yml`.
        let mut seen_sources: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        for plugin in &self.plugins {
            let trimmed = plugin.source.trim();
            if trimmed.is_empty() {
                return Err(ValidationError::EmptyPluginSource);
            }
            if let Some(ref v) = plugin.version {
                let stripped = v.strip_prefix('v').unwrap_or(v);
                if semver::Version::parse(stripped).is_err() {
                    return Err(ValidationError::InvalidPluginVersion {
                        plugin: plugin.source.clone(),
                        value: v.clone(),
                    });
                }
            }
            if plugin
                .manifest
                .as_deref()
                .is_some_and(|path| path.trim().is_empty())
            {
                return Err(ValidationError::EmptyPluginManifest {
                    plugin: plugin.source.clone(),
                });
            }
            for entry in &plugin.entrypoint {
                if entry.trim().is_empty() {
                    return Err(ValidationError::EmptyPluginEntrypoint {
                        plugin: plugin.source.clone(),
                    });
                }
            }
            let key = normalize_plugin_source_key(trimmed);
            if !seen_sources.insert(key) {
                return Err(ValidationError::DuplicatePluginSource {
                    plugin: plugin.source.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Semantic validation errors (distinct from structural serde errors).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("unsupported schema version: {0} (this binary supports v1)")]
    UnsupportedVersion(u32),
    #[error("field `{field}` must be kebab-case, got: {value:?}")]
    NotKebabCase { field: &'static str, value: String },
    #[error("`programs` must contain at least one entry")]
    EmptyPrograms,
    #[error("program entry has empty `name`")]
    EmptyProgramName,
    #[error("program {program:?} has empty `path`")]
    EmptyProgramPath { program: String },
    #[error("plugin entry has empty `source`")]
    EmptyPluginSource,
    #[error("plugin {plugin:?} has invalid `version`: {value:?} (expected semver, optionally prefixed with `v`)")]
    InvalidPluginVersion { plugin: String, value: String },
    #[error("duplicate plugin source {plugin:?}")]
    DuplicatePluginSource { plugin: String },
    #[error("plugin {plugin:?} has empty `manifest` path")]
    EmptyPluginManifest { plugin: String },
    #[error("plugin {plugin:?} has an empty `entrypoint` item")]
    EmptyPluginEntrypoint { plugin: String },
}

fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    if *bytes.last().unwrap() == b'-' {
        return false;
    }
    let mut prev_dash = false;
    for &b in bytes {
        let ok = b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if !ok {
            return false;
        }
        if b == b'-' && prev_dash {
            return false;
        }
        prev_dash = b == b'-';
    }
    true
}

fn normalize_plugin_source_key(source: &str) -> String {
    let trimmed = source.trim().trim_end_matches('/');
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    trimmed.to_ascii_lowercase()
}

/// Project metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectCfg {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_framework")]
    pub framework: Framework,
    /// Pinned anchor version expected by the project (`MAJOR.MINOR.PATCH`).
    /// Used by `doctor` / preflight to warn on major divergence from the
    /// detected toolchain. Optional — absence disables the check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_version: Option<String>,
    /// Pinned solana version expected by the project (`MAJOR.MINOR.PATCH`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solana_version: Option<String>,
    #[serde(default = "default_rust_edition")]
    pub rust_edition: String,
}

impl Default for ProjectCfg {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            framework: default_framework(),
            anchor_version: None,
            solana_version: None,
            rust_edition: default_rust_edition(),
        }
    }
}

/// Program framework. MVP scopes `anchor`; others are post-MVP placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    Anchor,
    Pinocchio,
    Shank,
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

/// One program entry under `programs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgramCfg {
    pub name: String,
    pub path: String,
    /// Map of cluster name (`localnet`/`devnet`/`mainnet` or any custom)
    /// to program address override.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cluster_overrides: BTreeMap<String, String>,
}

/// Workspace-level options (frontend wiring, monorepo layout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCfg {
    #[serde(default = "default_frontend")]
    pub frontend: Frontend,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontend_path: Option<String>,
}

impl Default for WorkspaceCfg {
    fn default() -> Self {
        Self {
            frontend: default_frontend(),
            frontend_path: None,
        }
    }
}

/// Frontend stack selection. `None` means "no frontend scaffolded".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Frontend {
    Next,
    Vite,
    None,
}

/// Cluster endpoints for the standard Solana triad. Custom clusters can
/// be added via `cluster_overrides` on individual programs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClustersCfg {
    #[serde(default = "default_localnet")]
    pub localnet: ClusterCfg,
    #[serde(default = "default_devnet")]
    pub devnet: ClusterCfg,
    #[serde(default = "default_mainnet")]
    pub mainnet: ClusterCfg,
}

impl Default for ClustersCfg {
    fn default() -> Self {
        Self {
            localnet: default_localnet(),
            devnet: default_devnet(),
            mainnet: default_mainnet(),
        }
    }
}

/// A single cluster endpoint configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClusterCfg {
    pub url: String,
    pub wallet: String,
}

/// Local runtime engine config (used by `chain serve`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCfg {
    #[serde(default = "default_runtime_engine")]
    pub engine: RuntimeEngine,
    #[serde(default = "default_runtime_port")]
    pub port: u16,
    #[serde(default = "default_faucet_sol")]
    pub faucet_sol: u64,
}

impl Default for RuntimeCfg {
    fn default() -> Self {
        Self {
            engine: default_runtime_engine(),
            port: default_runtime_port(),
            faucet_sol: default_faucet_sol(),
        }
    }
}

/// Runtime engine selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeEngine {
    Surfpool,
    TestValidator,
}

/// Plugin reference under `plugins[]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginCfg {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<PluginTransport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoint: Vec<String>,
    #[serde(default, skip_serializing_if = "PluginCapabilities::is_default")]
    pub capabilities: PluginCapabilities,
}

/// Plugin transport advertised either in `sunscreen.yml` or the plugin
/// manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginTransport {
    StdioJsonrpc,
    Grpc,
}

/// Coarse filesystem scopes granted to a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginFilesystemScope {
    Workspace,
    Scratch,
}

/// Coarse capabilities requested by a plugin runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginCapabilities {
    #[serde(default, skip_serializing_if = "is_false")]
    pub network: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filesystem: Vec<PluginFilesystemScope>,
}

impl PluginCapabilities {
    fn is_default(value: &Self) -> bool {
        value == &Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case_accepts_valid() {
        for s in ["a", "foo", "foo-bar", "x1-y2", "my-protocol"] {
            assert!(is_kebab_case(s), "should accept {s}");
        }
    }

    #[test]
    fn kebab_case_rejects_invalid() {
        for s in ["", "-foo", "foo-", "Foo", "foo_bar", "foo--bar", "FOO"] {
            assert!(!is_kebab_case(s), "should reject {s:?}");
        }
    }

    #[test]
    fn new_for_workspace_sets_program_and_frontend() {
        let cfg = Config::new_for_workspace("my-proto", Framework::Anchor, Frontend::Next);
        assert_eq!(cfg.project.name, "my-proto");
        assert_eq!(cfg.project.framework, Framework::Anchor);
        assert_eq!(cfg.programs.len(), 1);
        assert_eq!(cfg.programs[0].name, "my-proto");
        assert_eq!(cfg.programs[0].path, "programs/my-proto");
        assert_eq!(cfg.workspace.frontend, Frontend::Next);
        assert_eq!(cfg.workspace.frontend_path.as_deref(), Some("app"));
        cfg.validate().expect("workspace-bootstrap config is valid");
    }

    #[test]
    fn new_for_workspace_no_frontend_path_when_none() {
        let cfg = Config::new_for_workspace("p1", Framework::Anchor, Frontend::None);
        assert!(cfg.workspace.frontend_path.is_none());
    }

    #[test]
    fn validate_rejects_empty_programs() {
        let mut cfg = Config::default();
        cfg.project.name = "ok-name".into();
        assert_eq!(cfg.validate(), Err(ValidationError::EmptyPrograms));
    }

    #[test]
    fn validate_rejects_non_kebab_project_name() {
        let cfg = Config::new_for_workspace("Bad_Name", Framework::Anchor, Frontend::None);
        match cfg.validate() {
            Err(ValidationError::NotKebabCase { field, .. }) => {
                assert_eq!(field, "project.name");
            }
            other => panic!("expected NotKebabCase, got {other:?}"),
        }
    }

    fn workspace_skeleton() -> Config {
        Config::new_for_workspace("ok-name", Framework::Anchor, Frontend::None)
    }

    #[test]
    fn validate_accepts_plugin_with_v_prefix() {
        let mut cfg = workspace_skeleton();
        cfg.plugins.push(PluginCfg {
            source: "github.com/org/foo.git".into(),
            version: Some("v1.2.3".into()),
            ..PluginCfg::default()
        });
        cfg.plugins.push(PluginCfg {
            source: "local/bar".into(),
            version: None,
            ..PluginCfg::default()
        });
        cfg.validate().expect("valid plugins");
    }

    #[test]
    fn validate_rejects_empty_plugin_source() {
        let mut cfg = workspace_skeleton();
        cfg.plugins.push(PluginCfg {
            source: "   ".into(),
            version: None,
            ..PluginCfg::default()
        });
        assert_eq!(cfg.validate(), Err(ValidationError::EmptyPluginSource));
    }

    #[test]
    fn validate_rejects_invalid_plugin_version() {
        let mut cfg = workspace_skeleton();
        cfg.plugins.push(PluginCfg {
            source: "foo".into(),
            version: Some("latest".into()),
            ..PluginCfg::default()
        });
        match cfg.validate() {
            Err(ValidationError::InvalidPluginVersion { plugin, value }) => {
                assert_eq!(plugin, "foo");
                assert_eq!(value, "latest");
            }
            other => panic!("expected InvalidPluginVersion, got {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_duplicate_plugin_sources() {
        let mut cfg = workspace_skeleton();
        cfg.plugins.push(PluginCfg {
            source: "github.com/org/foo.git".into(),
            version: None,
            ..PluginCfg::default()
        });
        cfg.plugins.push(PluginCfg {
            source: "github.com/org/foo.git".into(),
            version: Some("1.0.0".into()),
            ..PluginCfg::default()
        });
        match cfg.validate() {
            Err(ValidationError::DuplicatePluginSource { plugin }) => {
                assert_eq!(plugin, "github.com/org/foo.git");
            }
            other => panic!("expected DuplicatePluginSource, got {other:?}"),
        }
    }

    #[test]
    fn validate_rejects_duplicate_normalized_plugin_sources() {
        let mut cfg = workspace_skeleton();
        cfg.plugins.push(PluginCfg {
            source: " GitHub.com/Org/Foo.git ".into(),
            version: None,
            ..PluginCfg::default()
        });
        cfg.plugins.push(PluginCfg {
            source: "github.com/org/foo".into(),
            version: None,
            ..PluginCfg::default()
        });
        match cfg.validate() {
            Err(ValidationError::DuplicatePluginSource { plugin }) => {
                assert_eq!(plugin, "github.com/org/foo");
            }
            other => panic!("expected DuplicatePluginSource, got {other:?}"),
        }
    }

    #[test]
    fn validate_accepts_phase6_plugin_runtime_fields() {
        let mut cfg = workspace_skeleton();
        cfg.plugins.push(PluginCfg {
            source: "plugins/hello".into(),
            version: Some("0.1.0".into()),
            manifest: Some("plugins/hello/sunscreen-plugin.json".into()),
            transport: Some(PluginTransport::StdioJsonrpc),
            entrypoint: vec!["./plugin.sh".into()],
            capabilities: PluginCapabilities {
                network: false,
                filesystem: vec![
                    PluginFilesystemScope::Workspace,
                    PluginFilesystemScope::Scratch,
                ],
            },
        });
        cfg.validate().expect("phase6 plugin config is valid");
    }

    #[test]
    fn default_clusters_have_standard_endpoints() {
        let c = ClustersCfg::default();
        assert!(c.localnet.url.contains("127.0.0.1"));
        assert!(c.devnet.url.contains("devnet"));
        assert!(c.mainnet.url.contains("mainnet"));
    }

    #[test]
    fn default_runtime_is_surfpool() {
        let r = RuntimeCfg::default();
        assert_eq!(r.engine, RuntimeEngine::Surfpool);
        assert_eq!(r.port, 8899);
        assert_eq!(r.faucet_sol, 100);
    }

    #[test]
    fn workspace_bootstrap_roundtrips() {
        let cfg = Config::new_for_workspace("my-proto", Framework::Anchor, Frontend::Next);
        let yaml = serde_yaml::to_string(&cfg).expect("serialize");
        let parsed: Config = serde_yaml::from_str(&yaml).expect("re-parse");
        assert_eq!(cfg, parsed);
    }
}

#[cfg(test)]
mod schemagen {
    use super::*;
    /// Regenerate `src/config/schemas/sunscreen.v1.json` by running
    /// `SUNSCREEN_DUMP_SCHEMA=1 cargo test --lib config::schema::schemagen -- --nocapture`
    /// and capturing stdout into the file.
    #[test]
    fn dump_v1_json_schema() {
        if std::env::var("SUNSCREEN_DUMP_SCHEMA").is_err() {
            return;
        }
        let schema = schemars::schema_for!(Config);
        println!("{}", serde_json::to_string_pretty(&schema).unwrap());
    }
}
