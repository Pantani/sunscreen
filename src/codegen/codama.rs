//! Thin Codama subprocess wrapper.

use std::path::{Path, PathBuf};

use heck::ToSnakeCase;

use crate::process::{CommandOutput, CommandSpec, ProcessRunner};

use super::codama_config::{codama_config_path, write_codama_config};
use super::{sorted_json_files, CodegenError, FileWrite};

/// Result of preparing `codama.json` and running Codama.
#[derive(Debug, Clone)]
pub struct CodamaRunReport {
    /// `codama.json` write report.
    pub config: FileWrite,
    /// Captured Codama subprocess output.
    pub output: CommandOutput,
}

/// Build the managed Codama command.
#[must_use]
pub fn codama_run_command(workspace_root: &Path) -> CommandSpec {
    CommandSpec::new("pnpm")
        .arg("exec")
        .arg("codama")
        .arg("run")
        .arg("--all")
        .arg("--config")
        .arg(codama_config_path())
        .cwd(workspace_root)
}

/// Ensure `codama.json` exists and is current.
pub fn ensure_codama_config(
    workspace_root: &Path,
    program: Option<&str>,
) -> Result<FileWrite, CodegenError> {
    let idl_stem = infer_idl_stem(workspace_root, program)?;
    write_codama_config(workspace_root, &idl_stem, &idl_stem)
}

/// Run `pnpm exec codama run --all --config codama.json`.
pub fn run_clients<R: ProcessRunner>(
    workspace_root: &Path,
    runner: &R,
    program: Option<&str>,
) -> Result<CodamaRunReport, CodegenError> {
    let config = ensure_codama_config(workspace_root, program)?;
    let output = runner.run(codama_run_command(workspace_root))?;
    Ok(CodamaRunReport { config, output })
}

/// Infer the Anchor IDL stem Codama should read.
pub fn infer_idl_stem(
    workspace_root: &Path,
    program: Option<&str>,
) -> Result<String, CodegenError> {
    if let Some(program) = program {
        return Ok(program.to_snake_case());
    }

    let target_idl = workspace_root.join("target/idl");
    let files = sorted_json_files(&target_idl)?;
    if let Some(path) = files.first() {
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            return Ok(stem.to_snake_case());
        }
    }

    if let Ok(ws) = crate::workspace::find_root(Some(workspace_root)) {
        if let Some(program) = ws.programs.first() {
            return Ok(program.name.to_snake_case());
        }
    }

    workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_snake_case())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            CodegenError::UserInput(
                "cannot infer Codama IDL name; pass --program or run inside a workspace".into(),
            )
        })
}

/// Return the expected source IDL path for a program stem.
#[must_use]
pub fn target_idl_path(workspace_root: &Path, idl_stem: &str) -> PathBuf {
    workspace_root
        .join("target")
        .join("idl")
        .join(format!("{idl_stem}.json"))
}
