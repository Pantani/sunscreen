//! Scaffolding for Anchor instruction files.
//!
//! Renders the per-instruction `programs/<program>/src/instructions/<name>.rs`
//! file described in ADR-0001 §8.4, plus helper string-builders consumed by
//! `cli-architect`'s rustpatch step to mutate `mod.rs` / `lib.rs` segments in
//! place.
//!
//! ## Marker contract
//!
//! The emitted file is split into two regions whose comments are matched
//! verbatim by the editing layer — bytes, spaces and case must not drift:
//!
//! - `// === sunscreen:auto-generated:begin segment=file version=1 generator=instruction ===`
//!   ... `// === sunscreen:auto-generated:end segment=file ===` — owned by the
//!   generator; re-runs may rewrite this region.
//! - `// === sunscreen:user-region:begin segment=handler ===` ...
//!   `// === sunscreen:user-region:end segment=handler ===` — user-owned; the
//!   generator only emits a stub on first creation and never overwrites it.

use heck::{ToPascalCase, ToSnakeCase};
use rust_embed::RustEmbed;
use serde::Serialize;

use crate::templates::error::TemplateError;
use crate::templates::funcs;

/// Embedded scaffold templates, rooted at `templates/scaffold/`.
#[derive(RustEmbed)]
#[folder = "templates/scaffold/"]
struct ScaffoldAssets;

const INSTRUCTION_TEMPLATE: &str = "instruction/instruction.rs.j2";

/// Kind of Anchor account a scaffolded instruction parameter represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AccountKind {
    /// Plain `AccountInfo` / `Signer` — the catch-all for user-defined
    /// accounts and PDAs.
    Generic,
    /// `Program<'info, System>` (`anchor_lang::system_program::System`).
    SystemProgram,
    /// `Program<'info, anchor_spl::token::Token>`.
    TokenProgram,
    /// `Program<'info, anchor_spl::associated_token::AssociatedToken>`.
    AssociatedTokenProgram,
}

impl AccountKind {
    fn as_tag(self) -> &'static str {
        match self {
            AccountKind::Generic => "Generic",
            AccountKind::SystemProgram => "SystemProgram",
            AccountKind::TokenProgram => "TokenProgram",
            AccountKind::AssociatedTokenProgram => "AssociatedTokenProgram",
        }
    }
}

/// A single argument to an instruction handler.
#[derive(Debug, Clone, Serialize)]
pub struct ArgSpec {
    /// Argument identifier (will be normalised to `snake_case` by templates).
    pub name: String,
    /// Rust type written verbatim — e.g. `u64`, `Pubkey`, `String`.
    pub ty: String,
}

/// One entry in a scaffolded `#[derive(Accounts)]` struct.
#[derive(Debug, Clone, Serialize)]
pub struct AccountSpec {
    /// Account identifier (normalised to `snake_case`).
    pub name: String,
    /// Emits `mut` in the `#[account(...)]` attribute.
    pub mutable: bool,
    /// Marks the account as a `Signer`.
    pub signer: bool,
    /// Optional PDA seeds. Each entry is a *Rust expression* written
    /// verbatim into the `seeds = [...]` list (e.g. `b"vault"`,
    /// `user.key().as_ref()`). Setting seeds also emits `bump`.
    pub seeds: Option<Vec<String>>,
    /// Discriminates the Rust type rendered for this entry.
    pub kind: AccountKind,
}

/// Context required to render a single instruction file.
#[derive(Debug, Clone, Serialize)]
pub struct InstructionCtx {
    /// Parent program (snake_case crate name).
    pub program_name: String,
    /// Instruction identifier (will be PascalCased / snake_cased as needed).
    pub instruction_name: String,
    /// Handler arguments, in declaration order.
    pub args: Vec<ArgSpec>,
    /// Accounts struct fields, in declaration order.
    pub accounts: Vec<AccountSpec>,
    /// If `Some(EventName)`, the generated handler emits that event and the
    /// auto-generated region imports `crate::events::<EventName>`.
    pub emit: Option<String>,
    /// Field names for the emitted event, when the event is known locally.
    pub emit_fields: Vec<String>,
}

/// Dispatch entry consumed by [`render_dispatch_segment`].
#[derive(Debug, Clone)]
pub struct InstructionDispatch {
    /// Instruction identifier (snake_case).
    pub name: String,
    /// Handler argument list (matches the rendered file).
    pub args: Vec<ArgSpec>,
}

/// Render the `instruction.rs.j2` template into a complete file body.
///
/// # Errors
/// [`TemplateError::Render`] on Jinja parse / render failure;
/// [`TemplateError::NotFound`] if the embedded template is missing.
pub fn render_instruction(ctx: &InstructionCtx) -> Result<String, TemplateError> {
    let src = ScaffoldAssets::get(INSTRUCTION_TEMPLATE).ok_or_else(|| TemplateError::NotFound {
        name: INSTRUCTION_TEMPLATE.to_string(),
    })?;
    let src = std::str::from_utf8(src.data.as_ref()).map_err(|e| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            format!("template {INSTRUCTION_TEMPLATE} is not valid UTF-8: {e}"),
        )
    })?;

    let mut env = minijinja::Environment::new();
    funcs::register(&mut env);
    env.add_template(INSTRUCTION_TEMPLATE, src)?;
    let tmpl = env.get_template(INSTRUCTION_TEMPLATE)?;
    let value = to_jinja_ctx(ctx);
    let rendered = tmpl.render(value)?;
    Ok(rendered)
}

fn to_jinja_ctx(ctx: &InstructionCtx) -> serde_json::Value {
    use serde_json::{json, Value};

    let accounts: Vec<Value> = ctx
        .accounts
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "mutable": a.mutable,
                "signer": a.signer,
                "seeds": a.seeds,
                "kind": a.kind.as_tag(),
            })
        })
        .collect();

    json!({
        "program_name": ctx.program_name,
        "instruction_name": ctx.instruction_name,
        "args": ctx.args,
        "accounts": accounts,
        "emit": ctx.emit,
        "emit_fields": ctx.emit_fields,
    })
}

/// Build the inner body of the `segment=instructions` block in
/// `programs/<program>/src/instructions/mod.rs`.
///
/// Emits two lines per instruction, alphabetically sorted:
///
/// ```text
/// pub mod foo;
/// pub use foo::*;
/// pub mod bar;
/// pub use bar::*;
/// ```
///
/// The returned string ends with a trailing newline.
#[must_use]
pub fn render_instructions_mod_segment(instructions: &[String]) -> String {
    let mut sorted: Vec<&String> = instructions.iter().collect();
    sorted.sort();
    sorted.dedup();

    let mut out = String::new();
    for name in sorted {
        out.push_str("pub mod ");
        out.push_str(name);
        out.push_str(";\n");
        out.push_str("pub use ");
        out.push_str(name);
        out.push_str("::*;\n");
    }
    out
}

/// Build the inner body of the `segment=dispatch` block in
/// `programs/<program>/src/lib.rs`.
///
/// One thin wrapper per instruction, forwarding to `instructions::<name>::handler`.
/// Ordering matches the input slice (callers control ordering — typically the
/// declaration order from `sunscreen.yml`).
#[must_use]
pub fn render_dispatch_segment(_program: &str, instructions: &[InstructionDispatch]) -> String {
    let mut out = String::new();
    for ix in instructions {
        // Normalise to snake_case for Rust identifiers so the dispatch wrapper
        // matches the names emitted by the instruction template.
        let name = ix.name.to_snake_case();
        let pascal = name.to_pascal_case();
        let arg_decls: String = ix
            .args
            .iter()
            .map(|a| format!(", {}: {}", a.name.to_snake_case(), a.ty))
            .collect();
        let arg_pass: String = ix
            .args
            .iter()
            .map(|a| format!(", {}", a.name.to_snake_case()))
            .collect();
        out.push_str(&format!(
            "pub fn {name}(ctx: Context<{pascal}>{arg_decls}) -> Result<()> {{\n    instructions::{name}::handler(ctx{arg_pass})\n}}\n",
            name = name,
            pascal = pascal,
            arg_decls = arg_decls,
            arg_pass = arg_pass,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instructions_mod_segment_sorted_and_deduped() {
        let out = render_instructions_mod_segment(&[
            "foo".to_string(),
            "bar".to_string(),
            "foo".to_string(),
        ]);
        assert_eq!(
            out,
            "pub mod bar;\npub use bar::*;\npub mod foo;\npub use foo::*;\n"
        );
    }

    #[test]
    fn dispatch_segment_emits_wrappers() {
        let out = render_dispatch_segment(
            "demo",
            &[InstructionDispatch {
                name: "initialize".to_string(),
                args: vec![ArgSpec {
                    name: "seed".to_string(),
                    ty: "u64".to_string(),
                }],
            }],
        );
        assert!(
            out.contains("pub fn initialize(ctx: Context<Initialize>, seed: u64) -> Result<()>")
        );
        assert!(out.contains("instructions::initialize::handler(ctx, seed)"));
    }
}
