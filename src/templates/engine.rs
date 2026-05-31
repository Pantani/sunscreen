//! Owning wrapper around a fully-loaded [`minijinja::Environment`].

use minijinja::Environment;

use crate::templates::embed::Assets;
use crate::templates::error::TemplateError;
use crate::templates::funcs;

/// Pre-loaded template environment.
///
/// On construction every asset returned by [`Assets::iter`] is decoded as
/// UTF-8 and registered with the underlying minijinja environment. Filter
/// registration happens once via [`funcs::register`].
pub struct Engine {
    env: Environment<'static>,
}

impl Engine {
    /// Build a new engine, loading every embedded template.
    ///
    /// # Errors
    /// Returns [`TemplateError::Render`] if a template fails to parse or if an
    /// asset's bytes are not valid UTF-8.
    pub fn new() -> Result<Self, TemplateError> {
        let mut env = Environment::new();
        funcs::register(&mut env);

        for name in Assets::iter() {
            let file = Assets::get(&name).ok_or_else(|| TemplateError::NotFound {
                name: name.to_string(),
            })?;
            let content = std::str::from_utf8(file.data.as_ref())
                .map_err(|e| {
                    minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        format!("template {name} is not valid UTF-8: {e}"),
                    )
                })?
                .to_owned();
            env.add_template_owned(name.to_string(), content)?;
        }

        Ok(Self { env })
    }

    /// Borrow the underlying environment (escape hatch for advanced uses).
    #[must_use]
    pub fn env(&self) -> &Environment<'static> {
        &self.env
    }

    /// Render a template by name with the given JSON context.
    ///
    /// # Errors
    /// Returns [`TemplateError::NotFound`] if the template is not embedded, or
    /// [`TemplateError::Render`] on any rendering failure.
    pub fn render(&self, name: &str, ctx: &serde_json::Value) -> Result<String, TemplateError> {
        let tmpl = self
            .env
            .get_template(name)
            .map_err(|_| TemplateError::NotFound {
                name: name.to_string(),
            })?;
        let rendered = tmpl.render(ctx)?;
        Ok(rendered)
    }
}
