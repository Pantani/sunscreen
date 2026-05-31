//! Process-wide, lazily-initialised render entry point.

use std::sync::OnceLock;

use crate::templates::engine::Engine;
use crate::templates::error::TemplateError;

static ENGINE: OnceLock<Result<Engine, TemplateError>> = OnceLock::new();

fn engine() -> Result<&'static Engine, TemplateError> {
    match ENGINE.get_or_init(Engine::new) {
        Ok(e) => Ok(e),
        Err(e) => Err(clone_err(e)),
    }
}

fn clone_err(e: &TemplateError) -> TemplateError {
    // `minijinja::Error` is not Clone; surface a NotFound-ish stand-in
    // describing the original failure for callers that retry.
    TemplateError::NotFound {
        name: format!("<engine init failed: {e}>"),
    }
}

/// Render an embedded template by name using the given JSON context.
///
/// # Errors
/// Propagates [`TemplateError`] from engine initialisation or rendering.
///
/// # Example
/// ```no_run
/// let ctx = serde_json::json!({ "version": "0.1.0", "name": "MyProj" });
/// let out = sunscreen::templates::render("version.txt.jinja", &ctx).unwrap();
/// assert!(out.contains("0.1.0"));
/// ```
pub fn render(name: &str, ctx: &serde_json::Value) -> Result<String, TemplateError> {
    engine()?.render(name, ctx)
}
