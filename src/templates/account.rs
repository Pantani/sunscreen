//! Scaffolding for Anchor account (state) files.
//!
//! Owns the `programs/<prog>/src/state/<account>.rs` body plus the
//! per-account entry inserted in the `segment=accounts` block of
//! `programs/<prog>/src/state/mod.rs`.
//!
//! ## Marker contract
//!
//! - The generated file is wrapped in a single
//!   `// === sunscreen:auto-generated:begin segment=file version=1 generator=account ===`
//!   region — owned by the generator; re-runs may rewrite it.
//! - The `state/mod.rs` host file carries
//!   `// === sunscreen:auto-generated:begin segment=accounts version=1 ===`
//!   whose body is the concatenation of [`render_account_mod_entry`] outputs,
//!   one per registered account, sorted alphabetically.

use heck::ToSnakeCase;
use rust_embed::RustEmbed;
use serde::Serialize;

use crate::templates::error::TemplateError;
use crate::templates::funcs;

#[derive(RustEmbed)]
#[folder = "templates/scaffold/"]
struct ScaffoldAssets;

const ACCOUNT_FILE_TEMPLATE: &str = "account/account.rs.j2";
const ACCOUNT_MOD_ENTRY_TEMPLATE: &str = "account/state_mod_entry.rs.j2";

/// Field of an `#[account]` struct.
#[derive(Debug, Clone, Serialize)]
pub struct FieldSpec {
    /// Field identifier (normalised to `snake_case`).
    pub name: String,
    /// Rust type written verbatim — e.g. `u64`, `Pubkey`, `[u8; 32]`.
    pub ty: String,
}

/// Context required to render a single account file.
#[derive(Debug, Clone, Serialize)]
pub struct AccountCtx {
    /// Parent program (snake_case crate name).
    pub program_name: String,
    /// Account identifier (will be PascalCased / snake_cased as needed).
    pub account_name: String,
    /// Struct fields, in declaration order.
    pub fields: Vec<FieldSpec>,
}

/// Render the `account.rs.j2` template into a full file body.
///
/// # Errors
/// [`TemplateError::Render`] on Jinja parse / render failure;
/// [`TemplateError::NotFound`] if the embedded template is missing.
pub fn render_account_file(ctx: &AccountCtx) -> Result<String, TemplateError> {
    render_named(
        ACCOUNT_FILE_TEMPLATE,
        serde_json::json!({
            "program_name": ctx.program_name,
            "account_name": ctx.account_name,
            "fields": ctx.fields,
        }),
    )
}

/// Render a single entry for the `segment=accounts` block of `state/mod.rs`.
///
/// Output is two lines (`pub mod <name>;` + `pub use <name>::*;`) terminated
/// by a trailing newline.
///
/// # Errors
/// Same conditions as [`render_account_file`].
pub fn render_account_mod_entry(account_name: &str) -> Result<String, TemplateError> {
    render_named(
        ACCOUNT_MOD_ENTRY_TEMPLATE,
        serde_json::json!({ "account_name": account_name }),
    )
}

/// Build the full inner body of the `segment=accounts` block.
///
/// Entries are sorted alphabetically and de-duplicated, mirroring
/// `render_instructions_mod_segment`.
#[must_use]
pub fn render_account_mod_segment(accounts: &[String]) -> String {
    let mut sorted: Vec<String> = accounts.iter().map(|a| a.to_snake_case()).collect();
    sorted.sort();
    sorted.dedup();

    let mut out = String::new();
    for name in sorted {
        out.push_str("pub mod ");
        out.push_str(&name);
        out.push_str(";\n");
        out.push_str("pub use ");
        out.push_str(&name);
        out.push_str("::*;\n");
    }
    out
}

fn render_named(name: &str, ctx: serde_json::Value) -> Result<String, TemplateError> {
    let src = ScaffoldAssets::get(name).ok_or_else(|| TemplateError::NotFound {
        name: name.to_string(),
    })?;
    let src = std::str::from_utf8(src.data.as_ref()).map_err(|e| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!("template {name} is not valid UTF-8: {e}"),
        )
    })?;
    let mut env = minijinja::Environment::new();
    funcs::register(&mut env);
    env.add_template(name, src)?;
    let tmpl = env.get_template(name)?;
    Ok(tmpl.render(ctx)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_file_renders_struct() {
        let out = render_account_file(&AccountCtx {
            program_name: "demo".into(),
            account_name: "vault".into(),
            fields: vec![
                FieldSpec {
                    name: "owner".into(),
                    ty: "Pubkey".into(),
                },
                FieldSpec {
                    name: "total".into(),
                    ty: "u64".into(),
                },
            ],
        })
        .unwrap();
        assert!(out.contains("#[account]"));
        assert!(out.contains("pub struct Vault {"));
        assert!(out.contains("pub owner: Pubkey,"));
        assert!(out.contains("pub total: u64,"));
        assert!(out.contains("segment=file version=1 generator=account"));
    }

    #[test]
    fn mod_segment_sorted_and_deduped() {
        let out = render_account_mod_segment(&["Vault".into(), "Treasury".into(), "vault".into()]);
        assert_eq!(
            out,
            "pub mod treasury;\npub use treasury::*;\npub mod vault;\npub use vault::*;\n"
        );
    }
}
