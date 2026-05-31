//! Embedded template engine for sunscreen.
//!
//! Wraps `rust-embed` (asset bundling) and `minijinja` (rendering) behind a
//! small, deterministic public surface. Templates live under
//! `templates/assets/` and are addressed by their relative path
//! (e.g. `version.txt.jinja`).
//!
//! See [`render`] for the primary entry point.

pub mod embed;
pub mod engine;
pub mod error;
pub mod funcs;
pub mod instruction;
pub mod render;
pub mod workspace;

pub use engine::Engine;
pub use error::TemplateError;
pub use instruction::{
    render_dispatch_segment, render_instruction, render_instructions_mod_segment, AccountKind,
    AccountSpec, ArgSpec, InstructionCtx, InstructionDispatch,
};
pub use render::render;
pub use workspace::render_workspace;
