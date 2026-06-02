//! `sunscreen generate` subcommand group.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::codegen::codama;
use crate::codegen::frontend_hooks::{
    changed_files as changed_hook_files, generate_frontend_hooks, FrontendHooksOptions, HookTarget,
};
use crate::codegen::idl::{export_idls, IdlExportOptions};
use crate::codegen::{relative_path, CodegenError};
use crate::error::SunscreenError;
use crate::runtime::subprocess::SubprocessRunner;
use crate::{
    config::schema::{Framework as ConfigFramework, Frontend as ConfigFrontend},
    workspace,
};

/// Subcommands grouped under `sunscreen generate`.
#[derive(Debug, Subcommand)]
pub enum GenerateCmd {
    /// Run Codama client generation from the managed `codama.json`.
    Clients(GenerateClientsArgs),
    /// Export Anchor IDLs into framework-agnostic managed artifacts.
    Idl(GenerateIdlArgs),
    /// Generate TanStack Query frontend hooks from exported IDLs.
    FrontendHooks(GenerateFrontendHooksArgs),
}

/// Flags for `sunscreen generate clients`.
#[derive(Debug, Args)]
pub struct GenerateClientsArgs {
    /// Program name. Defaults to the first built IDL/workspace program.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
}

/// Flags for `sunscreen generate idl`.
#[derive(Debug, Args)]
pub struct GenerateIdlArgs {
    /// Program name. Defaults to every built IDL in `target/idl`.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Output directory relative to the workspace root.
    #[arg(long, value_name = "DIR", default_value = "clients/idl")]
    pub out_dir: PathBuf,
}

/// Flags for `sunscreen generate frontend-hooks`.
#[derive(Debug, Args)]
pub struct GenerateFrontendHooksArgs {
    /// Program name. Defaults to every built IDL in `target/idl`.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Frontend root relative to the workspace. Required when the workspace
    /// was scaffolded with `--frontend none`.
    #[arg(long, value_name = "DIR")]
    pub frontend_path: Option<PathBuf>,
    /// Hook target to generate.
    #[arg(long, value_enum)]
    pub target: Option<HookTargetArg>,
}

/// CLI hook target selector.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HookTargetArg {
    /// Generate both React Query and Solid Query wrappers.
    All,
    /// Generate React Query wrappers.
    React,
    /// Generate Solid Query wrappers.
    Solid,
}

impl From<HookTargetArg> for HookTarget {
    fn from(value: HookTargetArg) -> Self {
        match value {
            HookTargetArg::All => Self::All,
            HookTargetArg::React => Self::React,
            HookTargetArg::Solid => Self::Solid,
        }
    }
}

/// Dispatch entry point invoked from `cli::root`.
pub fn run(cmd: &GenerateCmd, json: bool) -> Result<i32, SunscreenError> {
    match cmd {
        GenerateCmd::Clients(args) => run_clients(args, json),
        GenerateCmd::Idl(args) => run_idl(args, json),
        GenerateCmd::FrontendHooks(args) => run_frontend_hooks(args, json),
    }
}

fn run_clients(args: &GenerateClientsArgs, json: bool) -> Result<i32, SunscreenError> {
    let ws = workspace::find_root(None)?;
    ensure_anchor_codegen(&ws)?;
    let report = codama::run_clients(&ws.root, &SubprocessRunner, args.program.as_deref())
        .map_err(|err| map_codegen_err(err, "sunscreen generate clients"))?;
    let success = report.output.success();
    let exit_code = report.output.exit_code;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": success,
                "command": "generate_clients",
                "workspace": ws.root.display().to_string(),
                "codama_config": relative_path(&ws.root, &report.config.path),
                "codama_config_changed": report.config.changed,
                "exit_code": exit_code,
                "stdout": report.output.stdout,
                "stderr": report.output.stderr,
            })
        );
    } else {
        if !report.output.stdout.is_empty() {
            print!("{}", report.output.stdout);
        }
        if !report.output.stderr.is_empty() {
            eprint!("{}", report.output.stderr);
        }
        if success {
            println!(
                "generate clients: ok ({})",
                relative_path(&ws.root, &report.config.path)
            );
        } else {
            eprintln!("generate clients: codama failed with exit code {exit_code}");
        }
    }

    Ok(if success { 0 } else { exit_code })
}

fn run_idl(args: &GenerateIdlArgs, json: bool) -> Result<i32, SunscreenError> {
    let ws = workspace::find_root(None)?;
    ensure_anchor_codegen(&ws)?;
    let report = export_idls(
        &ws.root,
        &IdlExportOptions {
            program: args.program.clone(),
            out_dir: args.out_dir.clone(),
        },
    )
    .map_err(|err| map_codegen_err(err, "sunscreen generate idl"))?;
    let changed_files: Vec<_> = report
        .files
        .iter()
        .filter(|file| file.changed)
        .map(|file| relative_path(&ws.root, &file.path))
        .collect();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "generate_idl",
                "workspace": ws.root.display().to_string(),
                "files": report.files.iter().map(|file| relative_path(&ws.root, &file.path)).collect::<Vec<_>>(),
                "changed_files": changed_files,
            })
        );
    } else {
        println!(
            "generate idl: {} file(s), {} changed",
            report.files.len(),
            changed_files.len()
        );
    }
    Ok(0)
}

fn run_frontend_hooks(args: &GenerateFrontendHooksArgs, json: bool) -> Result<i32, SunscreenError> {
    let ws = workspace::find_root(None)?;
    ensure_anchor_codegen(&ws)?;
    let report = generate_frontend_hooks(
        &ws.root,
        &FrontendHooksOptions {
            program: args.program.clone(),
            frontend_path: args.frontend_path.clone(),
            target: args
                .target
                .map(HookTarget::from)
                .unwrap_or_else(|| default_hook_target(ws.config.workspace.frontend)),
        },
    )
    .map_err(|err| map_codegen_err(err, "sunscreen generate frontend-hooks"))?;
    let changed_files = changed_hook_files(&ws.root, &report.files);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "generate_frontend_hooks",
                "workspace": ws.root.display().to_string(),
                "files": report.files.iter().map(|file| relative_path(&ws.root, &file.path)).collect::<Vec<_>>(),
                "changed_files": changed_files,
            })
        );
    } else {
        println!(
            "generate frontend-hooks: {} file(s), {} changed",
            report.files.len(),
            changed_files.len()
        );
    }
    Ok(0)
}

fn default_hook_target(frontend: ConfigFrontend) -> HookTarget {
    match frontend {
        ConfigFrontend::Next | ConfigFrontend::Vite => HookTarget::React,
        ConfigFrontend::None => HookTarget::All,
    }
}

fn ensure_anchor_codegen(ws: &workspace::WorkspaceRoot) -> Result<(), SunscreenError> {
    if matches!(ws.config.project.framework, ConfigFramework::Anchor) {
        return Ok(());
    }
    Err(SunscreenError::UserInput(format!(
        "`sunscreen generate` currently consumes Anchor IDLs; workspace framework is `{}`. \
         Build Pinocchio programs with `sunscreen chain build --headless` and add an IDL/Shank plugin when needed.",
        framework_name(ws.config.project.framework)
    )))
}

fn framework_name(framework: ConfigFramework) -> &'static str {
    match framework {
        ConfigFramework::Anchor => "anchor",
        ConfigFramework::Pinocchio => "pinocchio",
        ConfigFramework::Shank => "shank",
    }
}

fn map_codegen_err(err: CodegenError, command: &str) -> SunscreenError {
    match err {
        CodegenError::UserInput(msg) => SunscreenError::UserInput(msg),
        CodegenError::Workspace(err) => SunscreenError::from(err),
        CodegenError::Process(source) if source.is_not_found() => SunscreenError::ToolchainMissing(
            format!("pnpm not found on PATH; install pnpm before running `{command}`"),
        ),
        other => SunscreenError::Other(anyhow::anyhow!("{command}: {other}")),
    }
}
