//! Custom minijinja filters for case conversion.
//!
//! Registered globally on the [`minijinja::Environment`] so every template can
//! use them without per-template wiring. Implementations defer to `heck` for
//! Unicode-aware, idiomatic conversions.

use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use minijinja::{Environment, Error as JinjaError, ErrorKind, Value};

fn pascal_case(value: Value) -> Result<String, JinjaError> {
    as_str(&value).map(|s| s.to_pascal_case())
}

fn camel_case(value: Value) -> Result<String, JinjaError> {
    as_str(&value).map(|s| s.to_lower_camel_case())
}

fn snake_case(value: Value) -> Result<String, JinjaError> {
    as_str(&value).map(|s| s.to_snake_case())
}

fn kebab_case(value: Value) -> Result<String, JinjaError> {
    as_str(&value).map(|s| s.to_kebab_case())
}

fn screaming_snake(value: Value) -> Result<String, JinjaError> {
    as_str(&value).map(|s| s.to_shouty_snake_case())
}

fn as_str(value: &Value) -> Result<&str, JinjaError> {
    value.as_str().ok_or_else(|| {
        JinjaError::new(
            ErrorKind::InvalidOperation,
            "case filter expects a string argument",
        )
    })
}

/// Register the full set of case-conversion filters on `env`.
pub fn register(env: &mut Environment<'_>) {
    env.add_filter("pascal_case", pascal_case);
    env.add_filter("camel_case", camel_case);
    env.add_filter("snake_case", snake_case);
    env.add_filter("kebab_case", kebab_case);
    env.add_filter("screaming_snake", screaming_snake);
}
