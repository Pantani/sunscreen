//! Framework-agnostic IDL artifact export.

use std::path::{Path, PathBuf};

use heck::ToSnakeCase;

use super::codama::target_idl_path;
use super::{
    ensure_safe_relative_subpath, sorted_json_files, write_if_changed, CodegenError, FileWrite,
};

/// Options for `generate idl`.
#[derive(Debug, Clone)]
pub struct IdlExportOptions {
    /// Optional program name. Defaults to every discovered target IDL.
    pub program: Option<String>,
    /// Workspace-relative output directory.
    pub out_dir: PathBuf,
}

impl Default for IdlExportOptions {
    fn default() -> Self {
        Self {
            program: None,
            out_dir: PathBuf::from("clients/idl"),
        }
    }
}

/// Export report.
#[derive(Debug, Clone)]
pub struct IdlExportReport {
    /// One report per copied/checked IDL file.
    pub files: Vec<FileWrite>,
}

/// Copy Anchor IDLs from `target/idl` to managed framework-agnostic artifacts.
pub fn export_idls(
    workspace_root: &Path,
    options: &IdlExportOptions,
) -> Result<IdlExportReport, CodegenError> {
    ensure_safe_relative_subpath("--out-dir", &options.out_dir)?;
    let sources = source_idls(workspace_root, options.program.as_deref())?;
    let out_dir = workspace_root.join(&options.out_dir);
    let mut files = Vec::new();

    for source in sources {
        let raw = std::fs::read_to_string(&source).map_err(|err| CodegenError::io(&source, err))?;
        let value: serde_json::Value =
            serde_json::from_str(&raw).map_err(|err| CodegenError::json(&source, err))?;
        let mut rendered =
            serde_json::to_string_pretty(&value).map_err(|err| CodegenError::json(&source, err))?;
        rendered.push('\n');
        let file_name = source.file_name().ok_or_else(|| {
            CodegenError::UserInput(format!("invalid IDL path: {}", source.display()))
        })?;
        files.push(write_if_changed(&out_dir.join(file_name), &rendered)?);
    }

    Ok(IdlExportReport { files })
}

fn source_idls(workspace_root: &Path, program: Option<&str>) -> Result<Vec<PathBuf>, CodegenError> {
    if let Some(program) = program {
        let source = target_idl_path(workspace_root, &program.to_snake_case());
        if source.is_file() {
            return Ok(vec![source]);
        }
        return Err(CodegenError::UserInput(format!(
            "missing Anchor IDL {}; run `sunscreen chain build` first",
            source.display()
        )));
    }

    let target_idl = workspace_root.join("target/idl");
    let files = sorted_json_files(&target_idl)?;
    if !files.is_empty() {
        return Ok(files);
    }

    let ws = crate::workspace::find_root(Some(workspace_root))?;
    let mut expected = Vec::new();
    for program in &ws.programs {
        let source = target_idl_path(workspace_root, &program.name.to_snake_case());
        if !source.is_file() {
            return Err(CodegenError::UserInput(format!(
                "missing Anchor IDL {}; run `sunscreen chain build` first",
                source.display()
            )));
        }
        expected.push(source);
    }
    Ok(expected)
}
