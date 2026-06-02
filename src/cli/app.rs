//! `sunscreen app` — declarative lifecycle for application plugins.
//!
//! This MVP only manages **declarations** in `sunscreen.yml` under
//! `plugins[]`. It does **not** execute any external plugin process, fetch
//! remote artifacts, expose gRPC/stdio transports, register dynamic
//! commands, or interact with a marketplace. Those capabilities belong to
//! the deferred Phase 6 plugin runtime. Every JSON success payload that
//! describes a plugin includes `status: "declared"` to make this scope
//! explicit to callers.
//!
//! Lifecycle:
//! - `install <source>[@<version>] [--version V] [--dry-run]` — add or
//!   update an entry.
//! - `uninstall <name-or-source> [--dry-run]` — remove an entry.
//! - `list` — list declared entries.
//! - `describe <name-or-source>` — show one entry.
//! - `update <name-or-source> --version V [--dry-run]` — change only the
//!   version of an existing entry. `--version` is **required** because
//!   "latest" lookup against a registry is out of scope.

use clap::{Args, Subcommand};
use serde_json::json;

use crate::config::schema::PluginCfg;
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::workspace::{self, WorkspaceRoot};

#[derive(Debug, Subcommand)]
pub enum AppCmd {
    /// Declare a plugin in `sunscreen.yml`.
    Install(InstallArgs),
    /// Remove a previously declared plugin.
    Uninstall(UninstallArgs),
    /// List declared plugins.
    List,
    /// Show one declared plugin.
    Describe(DescribeArgs),
    /// Change the pinned version of a declared plugin.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
#[command(disable_version_flag = true)]
pub struct InstallArgs {
    /// Plugin source. Accepts a path, repo URL, or registry ref. May embed
    /// a version via the `<source>@<version>` shorthand.
    pub source: String,
    /// Optional pinned version (overrides the `@<version>` shorthand).
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
    /// Print the planned change without touching `sunscreen.yml`.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Plugin to remove — match by exact source or by normalized basename.
    pub target: String,
    /// Print the planned change without touching `sunscreen.yml`.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct DescribeArgs {
    /// Plugin to describe — match by exact source or by normalized basename.
    pub target: String,
}

#[derive(Debug, Args)]
#[command(disable_version_flag = true)]
pub struct UpdateArgs {
    /// Plugin to update — match by exact source or by normalized basename.
    pub target: String,
    /// New pinned version. **Required**; "latest registry" is out of scope.
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
    /// Print the planned change without touching `sunscreen.yml`.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Dispatch entry invoked from `cli::root`.
pub fn run(cmd: &AppCmd, json_out: bool) -> Result<i32, SunscreenError> {
    match cmd {
        AppCmd::Install(args) => run_install(args, json_out),
        AppCmd::Uninstall(args) => run_uninstall(args, json_out),
        AppCmd::List => run_list(json_out),
        AppCmd::Describe(args) => run_describe(args, json_out),
        AppCmd::Update(args) => run_update(args, json_out),
    }
}

// ---------- helpers ----------

/// Strip path/URL prefix and `.git` suffix, yielding a basename for matching.
pub(crate) fn normalize_basename(source: &str) -> String {
    let trimmed = source.trim().trim_end_matches('/');
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    last.strip_suffix(".git").unwrap_or(last).to_string()
}

/// Validate `version`. Accepts plain semver (`1.2.3`) or a leading `v`.
pub(crate) fn validate_version_str(v: &str) -> Result<(), String> {
    let stripped = v.strip_prefix('v').unwrap_or(v);
    semver::Version::parse(stripped)
        .map(|_| ())
        .map_err(|e| format!("`{v}` is not a valid semver: {e}"))
}

/// Split a `<source>@<version>` shorthand. Returns `(source, version)`.
/// Falls back to `(input, None)` if the suffix is not a valid semver.
fn split_source_version(input: &str) -> (String, Option<String>) {
    if let Some(idx) = input.rfind('@') {
        let (head, tail) = input.split_at(idx);
        let version = &tail[1..];
        if !head.is_empty() && validate_version_str(version).is_ok() {
            return (head.to_string(), Some(version.to_string()));
        }
    }
    (input.to_string(), None)
}

/// Resolve `target` against the workspace's declared plugins. Matches by
/// exact `source` first, then by normalized basename. Returns the index of
/// the matched entry. Ambiguity (multiple basename matches) → `UserInput`.
fn resolve_plugin_index(plugins: &[PluginCfg], target: &str) -> Result<usize, SunscreenError> {
    if let Some((idx, _)) = plugins.iter().enumerate().find(|(_, p)| p.source == target) {
        return Ok(idx);
    }
    let needle = normalize_basename(target);
    let matches: Vec<usize> = plugins
        .iter()
        .enumerate()
        .filter(|(_, p)| normalize_basename(&p.source) == needle)
        .map(|(i, _)| i)
        .collect();
    match matches.as_slice() {
        [] => Err(SunscreenError::UserInput(format!(
            "no declared plugin matches {target:?}"
        ))),
        [only] => Ok(*only),
        many => Err(SunscreenError::UserInput(format!(
            "{target:?} matches {} declared plugins; pass the exact source",
            many.len()
        ))),
    }
}

fn map_tx_err(e: TxError) -> SunscreenError {
    SunscreenError::Other(anyhow::anyhow!("transaction error: {e}"))
}

fn plugin_to_json(plugin: &PluginCfg) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "name".to_string(),
        json!(normalize_basename(&plugin.source)),
    );
    obj.insert("source".to_string(), json!(plugin.source));
    obj.insert(
        "version".to_string(),
        plugin
            .version
            .as_ref()
            .map(|v| json!(v))
            .unwrap_or(serde_json::Value::Null),
    );
    obj.insert("status".to_string(), json!("declared"));
    serde_json::Value::Object(obj)
}

fn workspace_root() -> Result<WorkspaceRoot, SunscreenError> {
    workspace::find_root(None).map_err(SunscreenError::from)
}

/// Atomically replace `plugins[]` in `sunscreen.yml` with `new_plugins`.
///
/// Re-reads the on-disk manifest **without** the `SUNSCREEN_<SECTION>__...`
/// env overlay so transient overrides (e.g. `SUNSCREEN_PROJECT__NAME=ci-demo`
/// used to run a one-shot command) are NOT persisted as a side effect of
/// editing the plugin array. Only the `plugins[]` field is mutated; every
/// other field is taken verbatim from disk.
fn write_plugins(ws: &WorkspaceRoot, new_plugins: Vec<PluginCfg>) -> Result<(), SunscreenError> {
    let raw = std::fs::read_to_string(&ws.config_path).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read {}: {e}", ws.config_path.display()))
    })?;
    let mut on_disk: crate::config::Config = serde_yaml::from_str(&raw)
        .map_err(|e| SunscreenError::ConfigInvalid(format!("{}: {e}", ws.config_path.display())))?;
    on_disk.plugins = new_plugins;
    on_disk
        .validate()
        .map_err(|e| SunscreenError::ConfigInvalid(e.to_string()))?;
    let body = serde_yaml::to_string(&on_disk)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("serialize sunscreen.yml: {e}")))?;
    let mut tx = Transaction::new(&ws.root).map_err(map_tx_err)?;
    tx.stage_replace(&ws.config_path, body.as_bytes())
        .map_err(map_tx_err)?;
    tx.commit().map_err(map_tx_err)?;
    Ok(())
}

// ---------- subcommand impls ----------

fn run_list(json_out: bool) -> Result<i32, SunscreenError> {
    let ws = workspace_root()?;
    let apps: Vec<_> = ws.config.plugins.iter().map(plugin_to_json).collect();
    let config_rel = ws
        .config_path
        .strip_prefix(&ws.root)
        .unwrap_or(&ws.config_path)
        .to_string_lossy()
        .replace('\\', "/");
    if json_out {
        let payload = json!({
            "ok": true,
            "command": "app list",
            "config": config_rel,
            "apps": apps,
            "changed": false,
            "dry_run": false,
        });
        println!("{payload}");
    } else if ws.config.plugins.is_empty() {
        println!("no plugins declared in {config_rel}");
    } else {
        for plugin in &ws.config.plugins {
            let version = plugin.version.as_deref().unwrap_or("(unpinned)");
            println!(
                "{name}  {source}  {version}  declared",
                name = normalize_basename(&plugin.source),
                source = plugin.source,
                version = version,
            );
        }
    }
    Ok(0)
}

fn run_describe(args: &DescribeArgs, json_out: bool) -> Result<i32, SunscreenError> {
    let ws = workspace_root()?;
    let idx = resolve_plugin_index(&ws.config.plugins, &args.target)?;
    let plugin = &ws.config.plugins[idx];
    let config_rel = ws
        .config_path
        .strip_prefix(&ws.root)
        .unwrap_or(&ws.config_path)
        .to_string_lossy()
        .replace('\\', "/");
    if json_out {
        let payload = json!({
            "ok": true,
            "command": "app describe",
            "config": config_rel,
            "app": plugin_to_json(plugin),
            "changed": false,
            "dry_run": false,
        });
        println!("{payload}");
    } else {
        let version = plugin.version.as_deref().unwrap_or("(unpinned)");
        println!(
            "{name}\n  source:  {source}\n  version: {version}\n  status:  declared",
            name = normalize_basename(&plugin.source),
            source = plugin.source,
            version = version,
        );
    }
    Ok(0)
}

fn run_install(args: &InstallArgs, json_out: bool) -> Result<i32, SunscreenError> {
    let (source, parsed_version) = split_source_version(&args.source);
    if source.trim().is_empty() {
        return Err(SunscreenError::UserInput(
            "`source` must not be empty".to_string(),
        ));
    }
    let version = match (&args.version, parsed_version) {
        (Some(flag), _) => Some(flag.clone()),
        (None, parsed) => parsed,
    };
    if let Some(ref v) = version {
        validate_version_str(v).map_err(SunscreenError::UserInput)?;
    }

    let ws = workspace_root()?;
    let mut cfg = ws.config.clone();

    // Conflict / idempotency check:
    // - exact source match → update entry in place.
    // - basename collision with a different source → exit 4.
    let basename = normalize_basename(&source);
    let mut existing_idx: Option<usize> = None;
    for (i, p) in cfg.plugins.iter().enumerate() {
        if p.source == source {
            existing_idx = Some(i);
            break;
        }
        if normalize_basename(&p.source) == basename {
            return Err(SunscreenError::UserInput(format!(
                "plugin name {basename:?} already declared with a different source ({:?}); \
                 uninstall it first or pick a different source",
                p.source
            )));
        }
    }

    let mut changed = false;
    if let Some(i) = existing_idx {
        if cfg.plugins[i].version != version {
            cfg.plugins[i].version = version.clone();
            changed = true;
        }
    } else {
        cfg.plugins.push(PluginCfg {
            source: source.clone(),
            version: version.clone(),
        });
        changed = true;
    }

    if !args.dry_run && changed {
        write_plugins(&ws, cfg.plugins.clone())?;
    } else {
        cfg.validate()
            .map_err(|e| SunscreenError::ConfigInvalid(e.to_string()))?;
    }

    let app = plugin_to_json(&PluginCfg {
        source: source.clone(),
        version: version.clone(),
    });
    let config_rel = ws
        .config_path
        .strip_prefix(&ws.root)
        .unwrap_or(&ws.config_path)
        .to_string_lossy()
        .replace('\\', "/");
    if json_out {
        let payload = json!({
            "ok": true,
            "command": "app install",
            "config": config_rel,
            "app": app,
            "changed": changed,
            "dry_run": args.dry_run,
        });
        println!("{payload}");
    } else if args.dry_run {
        println!("dry-run: would declare plugin {source}");
    } else if changed {
        println!("declared plugin {source}");
    } else {
        println!("plugin {source} already declared (no change)");
    }
    Ok(0)
}

fn run_uninstall(args: &UninstallArgs, json_out: bool) -> Result<i32, SunscreenError> {
    let ws = workspace_root()?;
    let mut cfg = ws.config.clone();
    let idx = resolve_plugin_index(&cfg.plugins, &args.target)?;
    let removed = cfg.plugins[idx].clone();
    if !args.dry_run {
        cfg.plugins.remove(idx);
        write_plugins(&ws, cfg.plugins.clone())?;
    }
    let app = plugin_to_json(&removed);
    let config_rel = ws
        .config_path
        .strip_prefix(&ws.root)
        .unwrap_or(&ws.config_path)
        .to_string_lossy()
        .replace('\\', "/");
    if json_out {
        let payload = json!({
            "ok": true,
            "command": "app uninstall",
            "config": config_rel,
            "app": app,
            "changed": !args.dry_run,
            "dry_run": args.dry_run,
        });
        println!("{payload}");
    } else if args.dry_run {
        println!("dry-run: would remove plugin {}", removed.source);
    } else {
        println!("removed plugin {}", removed.source);
    }
    Ok(0)
}

fn run_update(args: &UpdateArgs, json_out: bool) -> Result<i32, SunscreenError> {
    let Some(version) = args.version.as_ref() else {
        return Err(SunscreenError::UserInput(
            "`--version <VERSION>` is required; \"latest registry\" lookup is out of scope for the MVP"
                .to_string(),
        ));
    };
    validate_version_str(version).map_err(SunscreenError::UserInput)?;
    let ws = workspace_root()?;
    let mut cfg = ws.config.clone();
    let idx = resolve_plugin_index(&cfg.plugins, &args.target)?;
    let changed = cfg.plugins[idx].version.as_deref() != Some(version.as_str());
    if changed {
        cfg.plugins[idx].version = Some(version.clone());
    }
    if !args.dry_run && changed {
        write_plugins(&ws, cfg.plugins.clone())?;
    } else {
        cfg.validate()
            .map_err(|e| SunscreenError::ConfigInvalid(e.to_string()))?;
    }
    let app = plugin_to_json(&cfg.plugins[idx]);
    let config_rel = ws
        .config_path
        .strip_prefix(&ws.root)
        .unwrap_or(&ws.config_path)
        .to_string_lossy()
        .replace('\\', "/");
    if json_out {
        let payload = json!({
            "ok": true,
            "command": "app update",
            "config": config_rel,
            "app": app,
            "changed": changed && !args.dry_run,
            "dry_run": args.dry_run,
        });
        println!("{payload}");
    } else if args.dry_run {
        println!(
            "dry-run: would set {} version to {version}",
            cfg.plugins[idx].source
        );
    } else if changed {
        println!("updated {} to {version}", cfg.plugins[idx].source);
    } else {
        println!(
            "{} already pinned at {version} (no change)",
            cfg.plugins[idx].source
        );
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_basename_strips_git_and_path() {
        assert_eq!(normalize_basename("github.com/org/foo.git"), "foo");
        assert_eq!(normalize_basename("foo"), "foo");
        assert_eq!(normalize_basename("./local/foo"), "foo");
        assert_eq!(normalize_basename("foo/"), "foo");
    }

    #[test]
    fn split_source_version_handles_shorthand() {
        let (s, v) = split_source_version("github.com/org/foo.git@1.2.3");
        assert_eq!(s, "github.com/org/foo.git");
        assert_eq!(v.as_deref(), Some("1.2.3"));

        let (s, v) = split_source_version("foo@v0.1.0");
        assert_eq!(s, "foo");
        assert_eq!(v.as_deref(), Some("v0.1.0"));

        // No version → whole input is the source.
        let (s, v) = split_source_version("user@host:org/foo");
        assert_eq!(s, "user@host:org/foo");
        assert!(v.is_none());

        let (s, v) = split_source_version("foo@not-semver");
        assert_eq!(s, "foo@not-semver");
        assert!(v.is_none());
    }

    #[test]
    fn version_validation_accepts_v_prefix() {
        assert!(validate_version_str("1.2.3").is_ok());
        assert!(validate_version_str("v1.2.3").is_ok());
        assert!(validate_version_str("v0.1.0-alpha.1+meta").is_ok());
        assert!(validate_version_str("latest").is_err());
        assert!(validate_version_str("1.2").is_err());
    }
}
