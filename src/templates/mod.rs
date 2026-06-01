//! Embedded template engine for sunscreen.
//!
//! Wraps `rust-embed` (asset bundling) and `minijinja` (rendering) behind a
//! small, deterministic public surface. Templates live under
//! `templates/assets/` and are addressed by their relative path
//! (e.g. `version.txt.jinja`).
//!
//! See [`render`] for the primary entry point.

pub mod account;
pub mod embed;
pub mod engine;
pub mod error;
pub mod event;
pub mod funcs;
pub mod instruction;
pub mod program;
pub mod render;
pub mod workspace;

pub use account::{
    render_account_file, render_account_mod_entry, render_account_mod_segment, AccountCtx,
};
pub use engine::Engine;
pub use error::{
    escape_msg, render_error_variant, render_errors_file, ErrorCtx, ErrorVariant, TemplateError,
    ERROR_VARIANTS_SEGMENT_BEGIN, ERROR_VARIANTS_SEGMENT_END,
};
pub use event::{
    render_event_entry, render_events_file, EventCtx, EVENTS_FILE_HEADER, EVENTS_SEGMENT_BEGIN,
    EVENTS_SEGMENT_END,
};
pub use instruction::{
    render_dispatch_segment, render_instruction, render_instructions_mod_segment, AccountKind,
    AccountSpec, ArgSpec, InstructionCtx, InstructionDispatch,
};
pub use program::render_program;
pub use render::render;
pub use workspace::render_workspace;
