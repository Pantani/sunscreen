//! Resolve, parse, and overlay configuration from disk + environment.

use std::env;
use std::io;
use std::path::{Path, PathBuf};

use serde_yaml::Value;
use thiserror::Error;

use crate::config::migrator::migrate;
use crate::config::schema::Config;
use crate::config::CURRENT_SCHEMA_VERSION;

/// Errors emitted while resolving or parsing configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// An explicitly requested path could not be found.
    #[error("config file not found: {0}")]
    NotFound(PathBuf),
    /// A config file existed but failed to parse.
    #[error("failed to parse config at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    /// The parsed document is structurally valid YAML but semantically wrong.
    #[error("invalid config: {msg}")]
    Validation { msg: String },
    /// A migration step failed.
    #[error("migration failed: {0}")]
    Migration(String),
    /// IO error reading a config file.
    #[error(transparent)]
    Io(#[from] io::Error),
}

const ENV_CONFIG_VAR: &str = "SUNSCREEN_CONFIG";
const ENV_OVERLAY_PREFIX: &str = "SUNSCREEN_";
const ENV_OVERLAY_SEP: &str = "__";

/// Load configuration honoring this precedence:
/// 1. `explicit` argument (errors if missing).
/// 2. `$SUNSCREEN_CONFIG`.
/// 3. `./sunscreen.yml` in the current directory.
/// 4. `$HOME/.config/sunscreen/config.yml`.
/// 5. Built-in defaults.
///
/// After parsing, environment variables of the form
/// `SUNSCREEN_<SECTION>__<KEY>[__<SUBKEY>]` are merged on top.
pub fn load(explicit: Option<&Path>) -> Result<Config, ConfigError> {
    let mut raw = match resolve_source(explicit)? {
        Some((path, contents)) => parse_yaml(&path, &contents)?,
        None => default_value(),
    };
    upgrade(&mut raw)?;
    let overlaid = apply_env_overlay(raw, env_iter());
    materialize(overlaid)
}

/// Inspect the raw document's `version` and run migrations up to the
/// current schema version. A missing `version` is treated as the current
/// version (no migration needed).
fn upgrade(raw: &mut Value) -> Result<(), ConfigError> {
    let version = raw
        .as_mapping()
        .and_then(|m| m.get(Value::String("version".to_string())))
        .and_then(Value::as_u64)
        .map(|v| u32::try_from(v).unwrap_or(CURRENT_SCHEMA_VERSION))
        .unwrap_or(CURRENT_SCHEMA_VERSION);
    if version == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }
    migrate(raw, CURRENT_SCHEMA_VERSION).map_err(|e| ConfigError::Migration(e.to_string()))
}

/// Internal: split out so tests can inject a deterministic env.
fn env_iter() -> Vec<(String, String)> {
    env::vars().filter(|(k, _)| is_overlay_key(k)).collect()
}

/// Returns true if an env var name is a config overlay key.
///
/// Overlay vars must follow `SUNSCREEN_<SECTION>__<KEY>[__<SUBKEY>]`.
/// Control vars like `SUNSCREEN_SKIP_PREFLIGHT` (no `__` separator) are read
/// directly by their consumers and must not be folded into the merged config,
/// where they would surface as unknown top-level fields.
fn is_overlay_key(k: &str) -> bool {
    if k == ENV_CONFIG_VAR {
        return false;
    }
    let Some(rest) = k.strip_prefix(ENV_OVERLAY_PREFIX) else {
        return false;
    };
    // Require at least two non-empty segments separated by `__`. This rejects
    // both control vars (no `__`, e.g. `SUNSCREEN_SKIP_PREFLIGHT`) and
    // malformed keys with empty segments (`SUNSCREEN__FOO`, `SUNSCREEN_X__`,
    // `SUNSCREEN___`), which would otherwise produce confusing
    // unknown-field/empty-key errors at materialize time.
    let segments: Vec<&str> = rest.split(ENV_OVERLAY_SEP).collect();
    segments.len() >= 2 && segments.iter().all(|s| !s.is_empty())
}

fn resolve_source(explicit: Option<&Path>) -> Result<Option<(PathBuf, String)>, ConfigError> {
    if let Some(p) = explicit {
        if !p.exists() {
            return Err(ConfigError::NotFound(p.to_path_buf()));
        }
        let body = std::fs::read_to_string(p)?;
        return Ok(Some((p.to_path_buf(), body)));
    }
    if let Ok(env_path) = env::var(ENV_CONFIG_VAR) {
        let p = PathBuf::from(env_path);
        if !p.exists() {
            return Err(ConfigError::NotFound(p));
        }
        let body = std::fs::read_to_string(&p)?;
        return Ok(Some((p, body)));
    }
    let cwd = PathBuf::from("./sunscreen.yml");
    if cwd.exists() {
        let body = std::fs::read_to_string(&cwd)?;
        return Ok(Some((cwd, body)));
    }
    if let Some(home) = home_dir() {
        let p = home.join(".config/sunscreen/config.yml");
        if p.exists() {
            let body = std::fs::read_to_string(&p)?;
            return Ok(Some((p, body)));
        }
    }
    Ok(None)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn parse_yaml(path: &Path, body: &str) -> Result<Value, ConfigError> {
    serde_yaml::from_str::<Value>(body).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn default_value() -> Value {
    serde_yaml::to_value(Config::default()).expect("default Config serializes")
}

fn materialize(value: Value) -> Result<Config, ConfigError> {
    // Re-serialize through string so we get serde_yaml's strict deny_unknown_fields behavior
    // with proper error locations.
    let body = serde_yaml::to_string(&value).map_err(|e| ConfigError::Validation {
        msg: format!("failed to re-serialize merged config: {e}"),
    })?;
    serde_yaml::from_str::<Config>(&body).map_err(|source| ConfigError::Parse {
        path: PathBuf::from("<merged>"),
        source,
    })
}

/// Apply env overlay onto a YAML value. Public-in-crate for testability.
fn apply_env_overlay<I>(mut root: Value, vars: I) -> Value
where
    I: IntoIterator<Item = (String, String)>,
{
    if !root.is_mapping() {
        root = Value::Mapping(serde_yaml::Mapping::new());
    }
    for (key, val) in vars {
        let Some(rest) = key.strip_prefix(ENV_OVERLAY_PREFIX) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let path: Vec<String> = rest
            .split(ENV_OVERLAY_SEP)
            .map(|seg| seg.to_ascii_lowercase())
            .collect();
        insert_path(&mut root, &path, parse_scalar(&val));
    }
    root
}

fn parse_scalar(raw: &str) -> Value {
    // Try YAML scalar parse so "true", "42", "1.5" become typed values.
    serde_yaml::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
}

fn insert_path(node: &mut Value, path: &[String], leaf: Value) {
    if path.is_empty() {
        *node = leaf;
        return;
    }
    let (head, tail) = path.split_first().expect("non-empty");
    if !node.is_mapping() {
        *node = Value::Mapping(serde_yaml::Mapping::new());
    }
    let mapping = node.as_mapping_mut().expect("ensured mapping");
    let key = Value::String(head.clone());
    if tail.is_empty() {
        mapping.insert(key, leaf);
    } else {
        let child = mapping
            .entry(key)
            .or_insert_with(|| Value::Mapping(serde_yaml::Mapping::new()));
        insert_path(child, tail, leaf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;

    const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/config");

    fn fixture(rel: &str) -> PathBuf {
        PathBuf::from(FIXTURES).join(rel)
    }

    fn parse_file(rel: &str) -> Result<Config, ConfigError> {
        // Direct path resolution; no env overlay (tests opt-in).
        let path = fixture(rel);
        let body = fs::read_to_string(&path)?;
        let mut raw = parse_yaml(&path, &body)?;
        upgrade(&mut raw)?;
        materialize(raw)
    }

    #[test]
    fn parse_valid_minimal() {
        let cfg = parse_file("valid/minimal.yml").expect("minimal parses");
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.project.name, "demo");
        assert!(cfg.project.description.is_none());
        assert!(cfg.toolchain.required.is_empty());
    }

    #[test]
    fn parse_valid_full() {
        let cfg = parse_file("valid/full.yml").expect("full parses");
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.project.name, "fullproj");
        assert_eq!(cfg.project.description.as_deref(), Some("A full example"));
        assert_eq!(
            cfg.toolchain.required.get("anchor").map(String::as_str),
            Some("0.31.0")
        );
        assert_eq!(
            cfg.scaffolding.default_template.as_deref(),
            Some("anchor-basic")
        );
    }

    #[test]
    fn reject_unknown_field() {
        let err = parse_file("invalid/unknown_field.yml").expect_err("unknown field rejected");
        let msg = format!("{err}");
        assert!(
            msg.contains("unknown field") || msg.contains("unknown"),
            "expected unknown-field error, got: {msg}"
        );
    }

    #[test]
    fn reject_bad_version() {
        let err = parse_file("invalid/bad_version.yml").expect_err("bad version rejected");
        let msg = format!("{err}");
        assert!(
            msg.to_lowercase().contains("version")
                || msg.contains("invalid type")
                || msg.contains("expected"),
            "expected version typing error, got: {msg}"
        );
    }

    #[test]
    fn roundtrip_idempotent() {
        let cfg = parse_file("valid/full.yml").expect("full parses");
        let serialized = serde_yaml::to_string(&cfg).expect("serialize");
        let cfg2: Config = serde_yaml::from_str(&serialized).expect("reparse");
        let serialized2 = serde_yaml::to_string(&cfg2).expect("serialize2");
        assert_eq!(
            serialized, serialized2,
            "round-trip serialization must be stable"
        );
        assert_eq!(cfg, cfg2, "round-trip must yield structurally equal Config");
    }

    #[test]
    fn migration_v0_to_v1_renames_id_to_name() {
        let cfg = parse_file("v0/old_format.yml").expect("v0 parses with migration");
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.project.name, "myproj");
    }

    #[test]
    fn migration_preserves_other_fields() {
        let cfg = parse_file("v0/old_format.yml").expect("v0 parses with migration");
        assert_eq!(
            cfg.project.description.as_deref(),
            Some("Legacy v0 project")
        );
        assert_eq!(
            cfg.toolchain.required.get("anchor").map(String::as_str),
            Some("0.31.0")
        );
        assert_eq!(
            cfg.scaffolding.default_template.as_deref(),
            Some("anchor-basic")
        );
    }

    #[test]
    fn already_at_current_version_no_migration() {
        // A document already at v1 must round-trip identically through upgrade().
        let path = fixture("valid/full.yml");
        let body = fs::read_to_string(&path).expect("read");
        let raw_before = parse_yaml(&path, &body).expect("parse");
        let mut raw_after = raw_before.clone();
        upgrade(&mut raw_after).expect("no-op upgrade");
        assert_eq!(
            raw_before, raw_after,
            "upgrade on current-version doc must be a no-op"
        );
    }

    #[test]
    fn env_overlay_overrides() {
        let path = fixture("valid/full.yml");
        let body = fs::read_to_string(&path).expect("read");
        let raw = parse_yaml(&path, &body).expect("parse");
        let env = vec![
            (
                "SUNSCREEN_TOOLCHAIN__REQUIRED__ANCHOR".to_string(),
                "2.0.0".to_string(),
            ),
            (
                "SUNSCREEN_PROJECT__DESCRIPTION".to_string(),
                "overlaid".to_string(),
            ),
        ];
        let merged = apply_env_overlay(raw, env);
        let cfg = materialize(merged).expect("materialize");
        assert_eq!(
            cfg.toolchain.required.get("anchor").map(String::as_str),
            Some("2.0.0")
        );
        assert_eq!(cfg.project.description.as_deref(), Some("overlaid"));
    }

    #[test]
    fn explicit_path_wins() {
        // Two fixtures: explicit points at "minimal", env points at "full".
        // Explicit must win.
        let explicit = fixture("valid/minimal.yml");
        let other = fixture("valid/full.yml");

        // We avoid touching real process env in tests; instead, exercise resolve_source
        // directly using the explicit path. Then assert it returned the explicit one.
        let resolved = resolve_source(Some(&explicit)).expect("resolves");
        let (path, _body) = resolved.expect("some");
        assert_eq!(path, explicit);
        assert_ne!(path, other);
    }

    #[test]
    fn parse_full_workspace_schema() {
        let cfg = parse_file("valid/workspace.yml").expect("workspace fixture parses");
        assert_eq!(cfg.project.name, "my-protocol");
        assert_eq!(cfg.project.framework, crate::config::Framework::Anchor);
        assert_eq!(cfg.programs.len(), 1);
        assert_eq!(cfg.programs[0].name, "my-protocol");
        assert_eq!(
            cfg.programs[0]
                .cluster_overrides
                .get("devnet")
                .map(String::as_str),
            Some("EscrowDevnetkD3aQVgPdMxbAv7XmgFK5Q6n8jR2")
        );
        assert_eq!(cfg.workspace.frontend, crate::config::Frontend::Next);
        assert_eq!(cfg.workspace.frontend_path.as_deref(), Some("app"));
        assert!(cfg.clusters.devnet.url.contains("devnet"));
        assert_eq!(cfg.runtime.engine, crate::config::RuntimeEngine::Surfpool);
        assert_eq!(cfg.runtime.faucet_sol, 100);
        assert!(cfg.plugins.is_empty());
        cfg.validate().expect("semantic validation passes");
    }

    #[test]
    fn workspace_constructor_roundtrips_through_loader() {
        let cfg = crate::config::Config::new_for_workspace(
            "demo-proj",
            crate::config::Framework::Anchor,
            crate::config::Frontend::Next,
        );
        let yaml = serde_yaml::to_string(&cfg).expect("serialize");
        let raw = serde_yaml::from_str::<Value>(&yaml).expect("re-parse yaml");
        let cfg2 = materialize(raw).expect("materialize");
        assert_eq!(cfg, cfg2);
    }

    #[test]
    fn env_overlay_clusters_devnet_url() {
        let path = fixture("valid/workspace.yml");
        let body = fs::read_to_string(&path).expect("read");
        let raw = parse_yaml(&path, &body).expect("parse");
        let env = vec![(
            "SUNSCREEN_CLUSTERS__DEVNET__URL".to_string(),
            "https://custom.devnet.example.com".to_string(),
        )];
        let merged = apply_env_overlay(raw, env);
        let cfg = materialize(merged).expect("materialize");
        assert_eq!(cfg.clusters.devnet.url, "https://custom.devnet.example.com");
        // Other fields preserved.
        assert!(cfg.clusters.mainnet.url.contains("mainnet"));
    }

    #[test]
    fn env_overlay_runtime_engine() {
        let path = fixture("valid/workspace.yml");
        let body = fs::read_to_string(&path).expect("read");
        let raw = parse_yaml(&path, &body).expect("parse");
        let env = vec![(
            "SUNSCREEN_RUNTIME__ENGINE".to_string(),
            "test-validator".to_string(),
        )];
        let merged = apply_env_overlay(raw, env);
        let cfg = materialize(merged).expect("materialize");
        assert_eq!(
            cfg.runtime.engine,
            crate::config::RuntimeEngine::TestValidator
        );
    }

    #[test]
    fn is_overlay_key_filters_control_vars() {
        // Control vars (no `__` separator) must be ignored by the overlay.
        assert!(!is_overlay_key("SUNSCREEN_SKIP_PREFLIGHT"));
        assert!(!is_overlay_key("SUNSCREEN_CONFIG"));
        assert!(!is_overlay_key("SUNSCREEN_"));
        assert!(!is_overlay_key("PATH"));
        // Malformed keys with empty segments must also be rejected.
        assert!(!is_overlay_key("SUNSCREEN__FOO"));
        assert!(!is_overlay_key("SUNSCREEN_SECTION__"));
        assert!(!is_overlay_key("SUNSCREEN___"));
        // Real overlay keys still match.
        assert!(is_overlay_key("SUNSCREEN_PROJECT__DESCRIPTION"));
        assert!(is_overlay_key("SUNSCREEN_TOOLCHAIN__REQUIRED__ANCHOR"));
    }

    #[test]
    fn env_overlay_ignores_control_var_skip_preflight() {
        // Regression: `SUNSCREEN_SKIP_PREFLIGHT=1` is a control var (no `__`).
        // It must not surface as an unknown top-level field during materialize.
        let path = fixture("valid/full.yml");
        let body = fs::read_to_string(&path).expect("read");
        let raw = parse_yaml(&path, &body).expect("parse");
        let env = vec![
            ("SUNSCREEN_SKIP_PREFLIGHT".to_string(), "1".to_string()),
            (
                "SUNSCREEN_PROJECT__DESCRIPTION".to_string(),
                "still applied".to_string(),
            ),
        ];
        let merged = apply_env_overlay(raw, env.into_iter().filter(|(k, _)| is_overlay_key(k)));
        let cfg = materialize(merged).expect("materialize must succeed");
        assert_eq!(cfg.project.description.as_deref(), Some("still applied"));
    }

    #[test]
    fn toolchain_required_is_btreemap() {
        // Compile-time guard for downstream consumers (toolchain-detector).
        let cfg = parse_file("valid/full.yml").expect("full parses");
        let _: &BTreeMap<String, String> = &cfg.toolchain.required;
    }
}
