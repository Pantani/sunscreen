//! Error type for the templates subsystem.

use thiserror::Error;

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
