//! Plugin discovery, command listing, and dynamic command dispatch.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::config::{PluginCapabilities, PluginCfg, PluginTransport};
use crate::error::SunscreenError;
use crate::plugin::manifest::{command_kind_str, PluginCommandKind, PluginManifest};
use crate::plugin::sandbox::PluginSandbox;
use crate::plugin::{grpc, stdio};
use crate::workspace::{self, WorkspaceRoot};

#[derive(Debug, Clone)]
pub struct ResolvedPlugin {
    pub declaration: PluginCfg,
    pub manifest: PluginManifest,
    pub source: String,
    pub plugin_dir: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    workspace: WorkspaceRoot,
    plugins: Vec<ResolvedPlugin>,
}

#[derive(Debug, Clone)]
pub struct CommandRunReport {
    pub plugin: String,
    pub command: String,
    pub kind: PluginCommandKind,
    pub transport: PluginTransport,
    pub result: serde_json::Value,
    pub duration_ms: Option<u128>,
}

#[derive(Debug, Clone)]
pub struct HookRunReport {
    pub plugin: String,
    pub hook: String,
    pub transport: PluginTransport,
    pub result: serde_json::Value,
    pub duration_ms: Option<u128>,
}

impl PluginManager {
    pub fn discover_current_workspace() -> Result<Self, SunscreenError> {
        let workspace = workspace::find_root(None).map_err(SunscreenError::from)?;
        Self::discover(workspace)
    }

    pub fn discover(workspace: WorkspaceRoot) -> Result<Self, SunscreenError> {
        let mut plugins = Vec::new();
        for declaration in &workspace.config.plugins {
            if let Some(plugin) = resolve_declared_plugin(&workspace.root, declaration)? {
                plugins.push(plugin);
            }
        }
        Ok(Self { workspace, plugins })
    }

    pub fn commands_json(&self) -> Vec<serde_json::Value> {
        self.plugins
            .iter()
            .flat_map(|plugin| {
                plugin.manifest.commands.iter().map(|command| {
                    json!({
                        "plugin": plugin.manifest.name,
                        "source": plugin.source,
                        "name": command.name,
                        "kind": command_kind_str(command.kind),
                        "summary": command.summary,
                        "transport": transport_str(plugin.transport()),
                        "status": "available",
                    })
                })
            })
            .collect()
    }

    pub fn run_app_command(
        &self,
        plugin_target: &str,
        command_name: &str,
        args: &[String],
    ) -> Result<CommandRunReport, SunscreenError> {
        let plugin = self.resolve_plugin(plugin_target)?;
        let command = plugin
            .manifest
            .commands
            .iter()
            .find(|command| command.kind == PluginCommandKind::App && command.name == command_name)
            .ok_or_else(|| {
                SunscreenError::UserInput(format!(
                    "plugin {plugin_target:?} does not declare app command {command_name:?}"
                ))
            })?;
        self.run(plugin, command.kind, &command.name, args)
    }

    pub fn run_scaffold_command(
        &self,
        command_name: &str,
        args: &[String],
    ) -> Result<CommandRunReport, SunscreenError> {
        let matches = self
            .plugins
            .iter()
            .filter(|plugin| {
                plugin.manifest.commands.iter().any(|command| {
                    command.kind == PluginCommandKind::Scaffold && command.name == command_name
                })
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(SunscreenError::UserInput(format!(
                "no plugin declares scaffold command {command_name:?}"
            ))),
            [plugin] => self.run(plugin, PluginCommandKind::Scaffold, command_name, args),
            many => Err(SunscreenError::UserInput(format!(
                "scaffold command {command_name:?} is declared by {} plugins; use `sunscreen app run <plugin> {command_name}`",
                many.len()
            ))),
        }
    }

    pub fn run_hook(
        &self,
        hook_name: &str,
        args: &[String],
    ) -> Result<Vec<HookRunReport>, SunscreenError> {
        let matching = self
            .plugins
            .iter()
            .filter(|plugin| plugin.manifest.hooks.iter().any(|hook| hook == hook_name))
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(SunscreenError::UserInput(format!(
                "no plugin declares hook {hook_name:?}"
            )));
        }
        matching
            .into_iter()
            .map(|plugin| self.run_hook_for_plugin(plugin, hook_name, args))
            .collect()
    }

    fn resolve_plugin(&self, target: &str) -> Result<&ResolvedPlugin, SunscreenError> {
        if let Some(plugin) = self.plugins.iter().find(|plugin| plugin.source == target) {
            return Ok(plugin);
        }
        let matches = self
            .plugins
            .iter()
            .filter(|plugin| plugin.manifest.name == target || basename(&plugin.source) == target)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(SunscreenError::UserInput(format!(
                "no available plugin matches {target:?}"
            ))),
            [plugin] => Ok(*plugin),
            many => Err(SunscreenError::UserInput(format!(
                "plugin target {target:?} matches {} available plugins; pass the exact source",
                many.len()
            ))),
        }
    }

    fn run(
        &self,
        plugin: &ResolvedPlugin,
        kind: PluginCommandKind,
        command_name: &str,
        args: &[String],
    ) -> Result<CommandRunReport, SunscreenError> {
        let transport = plugin.transport();
        let sandbox = PluginSandbox::new(
            &self.workspace.root,
            &plugin.manifest.name,
            plugin.capabilities(),
        );
        sandbox.prepare_for_workspace_execution(&plugin.manifest.name)?;
        for item in plugin.entrypoint() {
            sandbox.ensure_entrypoint_allowed(item)?;
        }
        match transport {
            PluginTransport::StdioJsonrpc => {
                let report = stdio::run(
                    &plugin.plugin_dir,
                    plugin.entrypoint(),
                    stdio::StdioInvocation {
                        plugin_name: &plugin.manifest.name,
                        method: "sunscreen.command/run",
                        command_kind: command_kind_str(kind),
                        command_name,
                        args,
                        workspace_root: &self.workspace.root,
                        scratch_dir: &sandbox.scratch_dir,
                    },
                )?;
                Ok(CommandRunReport {
                    plugin: plugin.manifest.name.clone(),
                    command: command_name.to_string(),
                    kind,
                    transport,
                    result: report.result,
                    duration_ms: Some(report.duration_ms),
                })
            }
            PluginTransport::Grpc => {
                let result = grpc::run_command(&plugin.manifest.name)?;
                Ok(CommandRunReport {
                    plugin: plugin.manifest.name.clone(),
                    command: command_name.to_string(),
                    kind,
                    transport,
                    result,
                    duration_ms: None,
                })
            }
        }
    }

    fn run_hook_for_plugin(
        &self,
        plugin: &ResolvedPlugin,
        hook_name: &str,
        args: &[String],
    ) -> Result<HookRunReport, SunscreenError> {
        let transport = plugin.transport();
        let sandbox = PluginSandbox::new(
            &self.workspace.root,
            &plugin.manifest.name,
            plugin.capabilities(),
        );
        sandbox.prepare_for_workspace_execution(&plugin.manifest.name)?;
        for item in plugin.entrypoint() {
            sandbox.ensure_entrypoint_allowed(item)?;
        }
        match transport {
            PluginTransport::StdioJsonrpc => {
                let report = stdio::run(
                    &plugin.plugin_dir,
                    plugin.entrypoint(),
                    stdio::StdioInvocation {
                        plugin_name: &plugin.manifest.name,
                        method: "sunscreen.hook/run",
                        command_kind: "hook",
                        command_name: hook_name,
                        args,
                        workspace_root: &self.workspace.root,
                        scratch_dir: &sandbox.scratch_dir,
                    },
                )?;
                Ok(HookRunReport {
                    plugin: plugin.manifest.name.clone(),
                    hook: hook_name.to_string(),
                    transport,
                    result: report.result,
                    duration_ms: Some(report.duration_ms),
                })
            }
            PluginTransport::Grpc => {
                let result = grpc::run_command(&plugin.manifest.name)?;
                Ok(HookRunReport {
                    plugin: plugin.manifest.name.clone(),
                    hook: hook_name.to_string(),
                    transport,
                    result,
                    duration_ms: None,
                })
            }
        }
    }
}

impl ResolvedPlugin {
    fn transport(&self) -> PluginTransport {
        self.declaration
            .transport
            .unwrap_or(self.manifest.transport)
    }

    fn entrypoint(&self) -> &[String] {
        if self.declaration.entrypoint.is_empty() {
            &self.manifest.entrypoint
        } else {
            &self.declaration.entrypoint
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        if self.declaration.capabilities == PluginCapabilities::default() {
            self.manifest.capabilities.clone()
        } else {
            self.declaration.capabilities.clone()
        }
    }
}

pub fn report_json(report: &CommandRunReport, command_label: String) -> serde_json::Value {
    json!({
        "ok": true,
        "command": command_label,
        "plugin": report.plugin,
        "plugin_command": report.command,
        "kind": command_kind_str(report.kind),
        "transport": transport_str(report.transport),
        "result": report.result,
        "duration_ms": report.duration_ms,
    })
}

pub fn hook_reports_json(reports: &[HookRunReport], hook: &str) -> serde_json::Value {
    json!({
        "ok": true,
        "command": "app hook",
        "hook": hook,
        "reports": reports.iter().map(|report| {
            json!({
                "plugin": report.plugin,
                "transport": transport_str(report.transport),
                "result": report.result,
                "duration_ms": report.duration_ms,
            })
        }).collect::<Vec<_>>(),
        "changed": false,
        "dry_run": false,
    })
}

pub fn transport_str(transport: PluginTransport) -> &'static str {
    match transport {
        PluginTransport::StdioJsonrpc => "stdio-jsonrpc",
        PluginTransport::Grpc => "grpc",
    }
}

fn resolve_declared_plugin(
    workspace_root: &Path,
    declaration: &PluginCfg,
) -> Result<Option<ResolvedPlugin>, SunscreenError> {
    let source_path = resolve_path(workspace_root, &declaration.source);
    let manifest_path = declaration
        .manifest
        .as_deref()
        .map(|path| resolve_path(workspace_root, path))
        .unwrap_or_else(|| source_path.join("sunscreen-plugin.json"));
    if !manifest_path.exists() {
        return Ok(None);
    }
    let manifest = PluginManifest::read(&manifest_path)?;
    let plugin_dir = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root.to_path_buf());
    Ok(Some(ResolvedPlugin {
        declaration: declaration.clone(),
        manifest,
        source: declaration.source.clone(),
        plugin_dir,
        manifest_path,
    }))
}

fn resolve_path(workspace_root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

fn basename(source: &str) -> String {
    let trimmed = source.trim().trim_end_matches('/');
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    last.strip_suffix(".git").unwrap_or(last).to_string()
}
