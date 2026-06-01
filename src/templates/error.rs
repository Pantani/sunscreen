//! Error type for the templates subsystem + scaffolding for Anchor
//! `#[error_code]` enums.
//!
//! ## Marker contract
//!
//! `programs/<prog>/src/errors.rs` carries a single
//! `// === sunscreen:auto-generated:begin segment=error_variants version=1 generator=error ===`
//! region whose body is the concatenation of [`render_error_variant`] outputs
//! (declaration order preserved). The surrounding enum scaffolding lives
//! *outside* the marker so the segment may be freely re-rendered.

use heck::ToPascalCase;
use rust_embed::RustEmbed;
use serde::Serialize;
use thiserror::Error;

use crate::templates::funcs;

/// Errors produced by template loading and rendering.
#[derive(Debug, Error)]
pub enum TemplateError {
    /// The requested template name is not present in the embedded bundle.
    #[error("template not found: {name}")]
    NotFound {
        /// Logical template path (relative to `templates/assets/`).
        name: String,
    },

    /// Underlying minijinja parse or render failure.
    #[error(transparent)]
    Render(#[from] minijinja::Error),
}

#[derive(RustEmbed)]
#[folder = "templates/scaffold/"]
struct ScaffoldAssets;

const ERROR_VARIANT_TEMPLATE: &str = "error/error_variant.rs.j2";

/// One variant of an Anchor `#[error_code]` enum.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorVariant {
    /// Variant identifier (will be PascalCased).
    pub name: String,
    /// Human-readable message used in `#[msg("...")]`. Embedded verbatim —
    /// callers are responsible for any escaping required by Rust string
    /// literals (the helper assumes the string contains no `"` characters;
    /// see [`escape_msg`] if you need to harden against arbitrary input).
    pub message: String,
}

/// Context required to render a full `errors.rs` file.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorCtx {
    /// Parent program (snake_case crate name).
    pub program_name: String,
    /// Name of the generated enum (typically `<Program>Error`).
    pub enum_name: String,
    /// Variants, in declaration order.
    pub variants: Vec<ErrorVariant>,
}

/// Canonical `begin` marker for the `error_variants` segment.
pub const ERROR_VARIANTS_SEGMENT_BEGIN: &str =
    "// === sunscreen:auto-generated:begin segment=error_variants version=1 generator=error ===";

/// Canonical `end` marker for the `error_variants` segment.
pub const ERROR_VARIANTS_SEGMENT_END: &str =
    "// === sunscreen:auto-generated:end segment=error_variants ===";

/// Render the single-variant template.
///
/// Output ends with a trailing newline. Multiple entries should be concatenated
/// in declaration order to form the body of the `segment=error_variants`
/// region.
///
/// # Errors
/// [`TemplateError::Render`] on Jinja parse / render failure;
/// [`TemplateError::NotFound`] if the embedded template is missing.
pub fn render_error_variant(variant: &ErrorVariant) -> Result<String, TemplateError> {
    render_named(
        ERROR_VARIANT_TEMPLATE,
        serde_json::json!({
            "variant_name": variant.name,
            "message": escape_msg(&variant.message),
        }),
    )
}

/// Render a brand-new `errors.rs` file body.
///
/// Layout:
///
/// ```text
/// use anchor_lang::prelude::*;
///
/// #[error_code]
/// pub enum <EnumName> {
///     // === sunscreen:auto-generated:begin segment=error_variants version=1 generator=error ===
///     <variants…>
///     // === sunscreen:auto-generated:end segment=error_variants ===
/// }
/// ```
///
/// # Errors
/// Same conditions as [`render_error_variant`].
pub fn render_errors_file(ctx: &ErrorCtx) -> Result<String, TemplateError> {
    let mut body = String::new();
    for v in &ctx.variants {
        let rendered = render_error_variant(v)?;
        // Indent each line of the variant by four spaces so it nests inside
        // the enum.
        for line in rendered.lines() {
            body.push_str("    ");
            body.push_str(line);
            body.push('\n');
        }
    }
    let enum_name = ctx.enum_name.to_pascal_case();
    Ok(format!(
        "use anchor_lang::prelude::*;\n\n#[error_code]\npub enum {enum_name} {{\n    {begin}\n{body}    {end}\n}}\n",
        enum_name = enum_name,
        begin = ERROR_VARIANTS_SEGMENT_BEGIN,
        body = body,
        end = ERROR_VARIANTS_SEGMENT_END,
    ))
}

/// Best-effort sanitisation of a `#[msg("…")]` literal: escapes backslashes
/// and double quotes. The Anchor macro accepts arbitrary Rust string content;
/// callers passing user input should route it through this helper.
#[must_use]
pub fn escape_msg(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c => out.push(c),
        }
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
    fn variant_renders_attr_and_name() {
        let out = render_error_variant(&ErrorVariant {
            name: "insufficient_funds".into(),
            message: "not enough lamports".into(),
        })
        .unwrap();
        assert!(out.contains("#[msg(\"not enough lamports\")]"));
        assert!(out.contains("InsufficientFunds,"));
    }

    #[test]
    fn errors_file_wraps_enum_with_markers() {
        let out = render_errors_file(&ErrorCtx {
            program_name: "demo".into(),
            enum_name: "demo_error".into(),
            variants: vec![ErrorVariant {
                name: "boom".into(),
                message: "kaboom".into(),
            }],
        })
        .unwrap();
        assert!(out.contains("pub enum DemoError {"));
        assert!(out.contains(ERROR_VARIANTS_SEGMENT_BEGIN));
        assert!(out.contains(ERROR_VARIANTS_SEGMENT_END));
        assert!(out.contains("Boom,"));
    }

    #[test]
    fn escape_msg_handles_quotes_and_backslashes() {
        assert_eq!(escape_msg(r#"a"b\c"#), r#"a\"b\\c"#);
    }
}
