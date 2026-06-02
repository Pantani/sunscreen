//! Idempotent `codama.json` rendering.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{write_if_changed, CodegenError, FileWrite};

/// Codama configuration file name at the workspace root.
pub const CODAMA_CONFIG_FILE: &str = "codama.json";

#[derive(Debug, Serialize)]
struct CodamaConfig {
    idl: String,
    before: Vec<serde_json::Value>,
    scripts: BTreeMap<&'static str, CodamaScript>,
}

#[derive(Debug, Serialize)]
struct CodamaScript {
    from: &'static str,
    args: Vec<serde_json::Value>,
}

/// Render the managed Codama config for a single Anchor IDL.
///
/// The public Solana/Codama contract is a JSON object with `idl`, `before`,
/// and `scripts`. For Phase 4 we manage the JavaScript renderer and keep Rust
/// client rendering out of scope until the workspace also owns a Rust client
/// crate.
pub fn render_codama_config_json(
    _project_name: &str,
    idl_stem: &str,
) -> Result<String, CodegenError> {
    let mut scripts = BTreeMap::new();
    scripts.insert(
        "js",
        CodamaScript {
            from: "@codama/renderers-js",
            args: vec![serde_json::json!("clients/js/src/generated")],
        },
    );
    let cfg = CodamaConfig {
        idl: format!("target/idl/{idl_stem}.json"),
        before: Vec::new(),
        scripts,
    };
    let mut json = serde_json::to_string_pretty(&cfg)
        .map_err(|source| CodegenError::json(CODAMA_CONFIG_FILE, source))?;
    json.push('\n');
    Ok(json)
}

/// Write `codama.json` only when its content changed.
pub fn write_codama_config(
    workspace_root: &Path,
    project_name: &str,
    idl_stem: &str,
) -> Result<FileWrite, CodegenError> {
    if !workspace_root.is_dir() {
        return Err(CodegenError::io(
            workspace_root.join(CODAMA_CONFIG_FILE),
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "workspace directory does not exist",
            ),
        ));
    }
    let path = workspace_root.join(CODAMA_CONFIG_FILE);
    let contents = render_codama_config_json(project_name, idl_stem)?;
    write_if_changed(&path, &contents)
}

/// Return the workspace-relative config path used by Codama.
#[must_use]
pub fn codama_config_path() -> PathBuf {
    PathBuf::from(CODAMA_CONFIG_FILE)
}
