//! Render a whole workspace template (set of files) into a staging directory.
//!
//! A *workspace template* is a sub-tree under `templates/workspace/<name>/`
//! whose files fall into one of two buckets:
//!
//! - **Jinja-rendered**: any file ending in `.j2`. The suffix is stripped from
//!   the output path and the contents are rendered through the shared
//!   [`Engine`].
//! - **Verbatim**: every other file is copied byte-for-byte.
//!
//! Path placeholders in directory and file names are also substituted:
//!
//! - `__program__`  -> `snake_case(program_name)`
//! - `__project__`  -> `kebab_case(project_name)`
//!
//! Output ordering is stable (paths are sorted before processing), so callers
//! get deterministic results suitable for golden snapshotting.
//!
//! # Example
//! ```no_run
//! use std::path::Path;
//! use serde_json::json;
//!
//! let ctx = json!({
//!     "project_name": "MyDapp",
//!     "program_name": "my_dapp",
//!     "anchor_version": "0.30.1",
//!     "solana_version": "1.18.18",
//!     "rust_edition": "2021",
//!     "frontend": "none",
//!     "cluster": "localnet",
//! });
//! let staging = std::path::PathBuf::from("/tmp/staging");
//! let written = sunscreen::templates::render_workspace(
//!     "anchor-multiple", &ctx, &staging,
//! ).unwrap();
//! assert!(!written.is_empty());
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use minijinja::Environment;
use rust_embed::RustEmbed;
use serde_json::Value;

use crate::templates::error::TemplateError;
use crate::templates::funcs;

/// Embedded workspace templates, rooted at `templates/workspace/`.
#[derive(RustEmbed)]
#[folder = "templates/workspace/"]
pub struct WorkspaceAssets;

/// File extension marking a Jinja-rendered template.
const J2_SUFFIX: &str = ".j2";

/// Render an entire workspace template into `out_staging`.
///
/// Returns the list of absolute paths that were written, sorted.
///
/// # Errors
/// - [`TemplateError::NotFound`] if no files exist under `template_name/`.
/// - [`TemplateError::Render`] on minijinja parse/render failure or IO error.
pub fn render_workspace(
    template_name: &str,
    ctx: &Value,
    out_staging: &Path,
) -> Result<Vec<PathBuf>, TemplateError> {
    let prefix = format!("{template_name}/");

    // Collect candidate assets in deterministic order.
    let mut assets: BTreeMap<String, std::borrow::Cow<'static, [u8]>> = BTreeMap::new();
    for name in WorkspaceAssets::iter() {
        if let Some(rel) = name.strip_prefix(&prefix) {
            let file = WorkspaceAssets::get(&name).ok_or_else(|| TemplateError::NotFound {
                name: name.to_string(),
            })?;
            assets.insert(rel.to_owned(), file.data);
        }
    }
    if assets.is_empty() {
        return Err(TemplateError::NotFound {
            name: template_name.to_string(),
        });
    }

    // Build an ephemeral environment with our filters; templates added on demand.
    let mut env = Environment::new();
    funcs::register(&mut env);

    let program_snake = string_filter(ctx, "program_name", "program", str_snake);
    let project_kebab = string_filter(ctx, "project_name", "project", str_kebab);

    let mut written = Vec::with_capacity(assets.len());

    for (rel, bytes) in assets {
        let rewritten = rewrite_path(&rel, &program_snake, &project_kebab);
        let (final_rel, contents): (String, Vec<u8>) =
            if let Some(stripped) = rewritten.strip_suffix(J2_SUFFIX) {
                let src = std::str::from_utf8(bytes.as_ref()).map_err(|e| {
                    minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        format!("template {rel} is not valid UTF-8: {e}"),
                    )
                })?;
                // Register under its logical name so error messages reference it.
                env.add_template_owned(rel.clone(), src.to_owned())?;
                let tmpl = env.get_template(&rel)?;
                let rendered = tmpl.render(ctx)?;
                (stripped.to_owned(), rendered.into_bytes())
            } else {
                (rewritten, bytes.into_owned())
            };

        let out_path = out_staging.join(&final_rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        std::fs::write(&out_path, &contents).map_err(io_err)?;
        written.push(out_path);
    }

    written.sort();
    Ok(written)
}

fn rewrite_path(rel: &str, program_snake: &str, project_kebab: &str) -> String {
    rel.replace("__program__", program_snake)
        .replace("__project__", project_kebab)
}

fn io_err(e: std::io::Error) -> TemplateError {
    TemplateError::Render(minijinja::Error::new(
        minijinja::ErrorKind::InvalidOperation,
        format!("io error: {e}"),
    ))
}

fn string_filter(ctx: &Value, primary: &str, fallback: &str, f: fn(&str) -> String) -> String {
    let raw = ctx
        .get(primary)
        .and_then(Value::as_str)
        .or_else(|| ctx.get(fallback).and_then(Value::as_str))
        .unwrap_or(fallback);
    f(raw)
}

fn str_snake(s: &str) -> String {
    use heck::ToSnakeCase;
    s.to_snake_case()
}

fn str_kebab(s: &str) -> String {
    use heck::ToKebabCase;
    s.to_kebab_case()
}
