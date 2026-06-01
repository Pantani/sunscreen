//! Scaffolding for Anchor event declarations.
//!
//! Owns the per-event entry inserted in the `segment=events` block of
//! `programs/<prog>/src/events.rs`, plus the helper used to render a fresh
//! `events.rs` host file when none exists.
//!
//! ## Marker contract
//!
//! - `events.rs` carries a single
//!   `// === sunscreen:auto-generated:begin segment=events version=1 generator=event ===`
//!   region whose body is the concatenation of [`render_event_entry`] outputs
//!   (declaration order preserved).

use rust_embed::RustEmbed;
use serde::Serialize;

use crate::templates::error::TemplateError;
use crate::templates::funcs;

#[derive(RustEmbed)]
#[folder = "templates/scaffold/"]
struct ScaffoldAssets;

const EVENT_ENTRY_TEMPLATE: &str = "event/event_entry.rs.j2";

/// Field of an `#[event]` struct.
#[derive(Debug, Clone, Serialize)]
pub struct FieldSpec {
    /// Field identifier (normalised to `snake_case`).
    pub name: String,
    /// Rust type written verbatim — e.g. `u64`, `Pubkey`.
    pub ty: String,
}

/// Context required to render a single event entry.
#[derive(Debug, Clone, Serialize)]
pub struct EventCtx {
    /// Parent program (snake_case crate name).
    pub program_name: String,
    /// Event identifier (will be PascalCased as needed).
    pub event_name: String,
    /// Struct fields, in declaration order.
    pub fields: Vec<FieldSpec>,
}

/// File-level header rendered when `events.rs` does not yet exist.
///
/// The full host file is laid out as:
/// `<EVENTS_FILE_HEADER><begin marker>\n<entries>\n<end marker>\n`.
pub const EVENTS_FILE_HEADER: &str = "use anchor_lang::prelude::*;\n\n";

/// Canonical `begin` marker for the `events` segment.
pub const EVENTS_SEGMENT_BEGIN: &str =
    "// === sunscreen:auto-generated:begin segment=events version=1 generator=event ===";

/// Canonical `end` marker for the `events` segment.
pub const EVENTS_SEGMENT_END: &str = "// === sunscreen:auto-generated:end segment=events ===";

/// Render the single-event template.
///
/// Output ends with a trailing newline. Multiple entries should be concatenated
/// in declaration order to form the body of the `segment=events` region.
///
/// # Errors
/// [`TemplateError::Render`] on Jinja parse / render failure;
/// [`TemplateError::NotFound`] if the embedded template is missing.
pub fn render_event_entry(ctx: &EventCtx) -> Result<String, TemplateError> {
    render_named(
        EVENT_ENTRY_TEMPLATE,
        serde_json::json!({
            "program_name": ctx.program_name,
            "event_name": ctx.event_name,
            "fields": ctx.fields,
        }),
    )
}

/// Render a brand-new `events.rs` file body containing zero or more events.
///
/// Determinism: entries are emitted in the iterator order provided by the
/// caller. The caller controls ordering (typically the declaration order
/// from `sunscreen.yml`).
///
/// # Errors
/// Same conditions as [`render_event_entry`].
pub fn render_events_file(events: &[EventCtx]) -> Result<String, TemplateError> {
    let mut body = String::new();
    for ev in events {
        body.push_str(&render_event_entry(ev)?);
        body.push('\n');
    }
    Ok(format!(
        "{header}{begin}\n{body}{end}\n",
        header = EVENTS_FILE_HEADER,
        begin = EVENTS_SEGMENT_BEGIN,
        body = body,
        end = EVENTS_SEGMENT_END,
    ))
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
    fn event_entry_renders_struct() {
        let out = render_event_entry(&EventCtx {
            program_name: "demo".into(),
            event_name: "deposited".into(),
            fields: vec![FieldSpec {
                name: "amount".into(),
                ty: "u64".into(),
            }],
        })
        .unwrap();
        assert!(out.contains("#[event]"));
        assert!(out.contains("pub struct Deposited {"));
        assert!(out.contains("pub amount: u64,"));
    }

    #[test]
    fn events_file_wraps_in_markers() {
        let out = render_events_file(&[EventCtx {
            program_name: "demo".into(),
            event_name: "ping".into(),
            fields: vec![],
        }])
        .unwrap();
        assert!(out.starts_with("use anchor_lang::prelude::*;"));
        assert!(out.contains(EVENTS_SEGMENT_BEGIN));
        assert!(out.contains(EVENTS_SEGMENT_END));
        assert!(out.contains("pub struct Ping"));
    }
}
