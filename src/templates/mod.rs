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
pub mod render;

pub use engine::Engine;
pub use error::TemplateError;
pub use render::render;
