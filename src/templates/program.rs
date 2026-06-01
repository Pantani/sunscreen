//! Render the `program` scaffold template into a staging directory.
//!
//! Mirrors [`crate::templates::workspace::render_workspace`] but is rooted at
//! `templates/scaffold/program/`. Used by `sunscreen scaffold program <name>`
//! to materialise a fresh Anchor program crate inside an existing workspace.
//!
//! At render time the crate is seeded with only the two markers that exist
//! up front: `segment=dispatch` (inside `#[program] mod {}`) and
//! `segment=instructions` (in `instructions/mod.rs`). The other four
//! segments (`accounts`, `events`, `errors`, `state`) are added lazily by
//! the subsequent scaffolders (`instruction`, `account`, `event`, `error`)
//! when the user first creates one — see `ensure_lib_mod_decl` in
//! `src/cli/scaffold.rs`. This keeps the freshly scaffolded crate
//! `cargo check`-clean (no `mod` declarations pointing at non-existent
//! files).
//!
//! Path placeholders mirror the workspace renderer:
//!
//! - `__program__` → `snake_case(program_name)`
//! - `__project__` → `kebab_case(project_name)`
//!
//! Output ordering is stable (paths sorted before processing) for golden
//! snapshot testing.
//!
//! # Required context keys
//! - `program_name` (string)
//! - `project_name` (string)
//! - `anchor_version` (string)
//! - `rust_edition` (string)
//! - `program_id` (string) — base58 pubkey rendered into `declare_id!`.
//!   Defaults to the canonical dummy id via Jinja `default()` if omitted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use minijinja::Environment;
use rust_embed::RustEmbed;
use serde_json::Value;

use crate::templates::error::TemplateError;
use crate::templates::funcs;

/// Embedded scaffold templates, rooted at `templates/scaffold/`.
#[derive(RustEmbed)]
#[folder = "templates/scaffold/"]
struct ScaffoldAssets;

/// Sub-prefix selecting the `program` scaffold tree.
const PROGRAM_PREFIX: &str = "program/";

/// File extension marking a Jinja-rendered template.
const J2_SUFFIX: &str = ".j2";

/// Render the `program` scaffold into `out_staging`.
///
/// Returns the sorted list of paths written.
///
/// # Errors
/// - [`TemplateError::NotFound`] if the embedded `program/` sub-tree is empty.
/// - [`TemplateError::Render`] on minijinja parse / render failure or IO error.
pub fn render_program(ctx: &Value, out_staging: &Path) -> Result<Vec<PathBuf>, TemplateError> {
    let mut assets: BTreeMap<String, std::borrow::Cow<'static, [u8]>> = BTreeMap::new();
    for name in ScaffoldAssets::iter() {
        if let Some(rel) = name.strip_prefix(PROGRAM_PREFIX) {
            let file = ScaffoldAssets::get(&name).ok_or_else(|| TemplateError::NotFound {
                name: name.to_string(),
            })?;
            assets.insert(rel.to_owned(), file.data);
        }
    }
    if assets.is_empty() {
        return Err(TemplateError::NotFound {
            name: "program".to_string(),
        });
    }

    let mut env = Environment::new();
    funcs::register(&mut env);

    let program_snake = required_string(ctx, "program_name", str_snake)?;
    let project_kebab = required_string(ctx, "project_name", str_kebab)?;

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

fn required_string(ctx: &Value, key: &str, f: fn(&str) -> String) -> Result<String, TemplateError> {
    let raw = ctx.get(key).and_then(Value::as_str).ok_or_else(|| {
        TemplateError::Render(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!("missing required context key: `{key}`"),
        ))
    })?;
    Ok(f(raw))
}

fn str_snake(s: &str) -> String {
    use heck::ToSnakeCase;
    s.to_snake_case()
}

fn str_kebab(s: &str) -> String {
    use heck::ToKebabCase;
    s.to_kebab_case()
}
