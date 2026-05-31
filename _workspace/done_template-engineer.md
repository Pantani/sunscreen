# done: template-engineer

## Files created

- `src/templates/mod.rs` — module surface, re-exports `render`, `Engine`, `TemplateError`.
- `src/templates/embed.rs` — `Assets` struct (`rust-embed`, folder `templates/assets/`).
- `src/templates/funcs.rs` — minijinja filters: `pascal_case`, `camel_case`, `snake_case`, `kebab_case`, `screaming_snake` (backed by `heck`). Exposes `pub fn register(env: &mut Environment)`.
- `src/templates/engine.rs` — `Engine` struct owning `Environment<'static>`; `Engine::new()` loads every embedded asset via `Assets::iter()`/`Assets::get()` and `add_template_owned`.
- `src/templates/render.rs` — process-wide `OnceLock<Engine>` and `pub fn render(name, ctx) -> Result<String, TemplateError>`.
- `src/templates/error.rs` — `TemplateError` (`thiserror`): `NotFound { name }`, `Render(#[from] minijinja::Error)`.
- `templates/assets/version.txt.jinja` — seed template.
- `templates/assets/README.md` — naming convention, JSON context, available filters, determinism notes.
- `tests/golden/render_basic.rs` — `insta::assert_snapshot!` test rendering `version.txt.jinja`.
- `_workspace/deps_template-engineer.toml` — dependency manifest for cli-architect to merge.

## Public API

```rust
pub fn sunscreen::templates::render(
    name: &str,
    ctx: &serde_json::Value,
) -> Result<String, TemplateError>;

pub struct sunscreen::templates::Engine;
impl Engine {
    pub fn new() -> Result<Self, TemplateError>;
    pub fn env(&self) -> &minijinja::Environment<'static>;
    pub fn render(&self, name: &str, ctx: &serde_json::Value) -> Result<String, TemplateError>;
}

pub enum sunscreen::templates::TemplateError {
    NotFound { name: String },
    Render(minijinja::Error),
}
```

## Filters available in templates

`pascal_case`, `camel_case`, `snake_case`, `kebab_case`, `screaming_snake`.

## Actions required (other agents)

- **cli-architect**:
  - Add `pub mod templates;` to `src/lib.rs`.
  - Merge `_workspace/deps_template-engineer.toml` into root `Cargo.toml`:
    - `rust-embed = "8"`, `minijinja = "2"`, `heck = "0.5"`,
      `indexmap = { version = "2", features = ["serde"] }`, `once_cell = "1"`.
    - dev: `insta = "1"`, `serde_json = "1"` (note: `serde_json` likely also a
      runtime dep since `render` takes `&serde_json::Value` — promote to
      `[dependencies]` if not already there).
  - Ensure `tests/golden/render_basic.rs` is picked up (Cargo auto-discovers
    `tests/*.rs`; subdirectory files like `tests/golden/render_basic.rs` need
    either `[[test]] name = "render_basic" path = "tests/golden/render_basic.rs"`
    in `Cargo.toml`, **or** move/symlink to `tests/render_basic.rs`).
- **config-engineer**: template names are addressed by their path relative to
  `templates/assets/` (e.g. `version.txt.jinja`) — reference these strings
  directly from `sunscreen.yml`.

## Notes

- Did not run `cargo build` per instructions.
- Snapshots directory `tests/golden/snapshots/` created; first `cargo test`
  run will populate it (use `INSTA_UPDATE=auto` or `cargo insta review`).
- No unsafe code. No timestamps/hostnames in render path — determinism holds.
