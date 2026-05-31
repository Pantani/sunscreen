# config-engineer — DONE

## Files created
- `src/config/mod.rs`
- `src/config/schema.rs`
- `src/config/loader.rs`
- `src/config/migrator.rs`
- `src/config/schemas/sunscreen.v1.json` (manual placeholder; regenerate later via `schemars::schema_for!(Config)`)
- `tests/fixtures/config/valid/minimal.yml`
- `tests/fixtures/config/valid/full.yml`
- `tests/fixtures/config/invalid/unknown_field.yml`
- `tests/fixtures/config/invalid/bad_version.yml`
- `_workspace/deps_config-engineer.toml`

## Public API

```rust
// src/config/mod.rs re-exports:
pub use schema::{Config, ProjectCfg, ScaffoldingCfg, ToolchainCfg};
pub use loader::{load, ConfigError};
pub use migrator::{migrate, registry, Migration};
```

### `Config`
```rust
pub struct Config {
    pub version: u32,                 // default 1
    pub project: ProjectCfg,
    pub toolchain: ToolchainCfg,
    pub scaffolding: ScaffoldingCfg,
}
```
All structs derive `Debug, Clone, Serialize, Deserialize, JsonSchema` and use `#[serde(deny_unknown_fields)]`.

### `load`
```rust
pub fn load(explicit: Option<&std::path::Path>) -> Result<Config, ConfigError>;
```
Resolution order: explicit > `$SUNSCREEN_CONFIG` > `./sunscreen.yml` > `$HOME/.config/sunscreen/config.yml` > built-in defaults.
After parse, env vars `SUNSCREEN_<SECTION>__<KEY>[__<SUBKEY>]` (separator `__`, lowercase keys) are overlaid.

### `ConfigError` (thiserror)
- `NotFound(PathBuf)`
- `Parse { path: PathBuf, source: serde_yaml::Error }`
- `Validation { msg: String }`
- `Io(#[from] std::io::Error)`

## For toolchain-detector

```rust
use std::collections::BTreeMap;

pub struct ToolchainCfg {
    pub required: BTreeMap<String, String>, // tool name → semver minimum
}
```
Tool name keys are lowercase by convention (e.g. `"anchor"`, `"solana"`, `"rustc"`); values are semver strings such as `"0.31.0"`. Env overlay path: `SUNSCREEN_TOOLCHAIN__REQUIRED__<TOOL>=<version>`.

## ACTION REQUIRED for cli-architect
Add `pub mod config;` to `src/lib.rs` (alongside existing `pub mod cli; pub mod error;`). Optionally re-export `pub use config::{Config, ConfigError};` for ergonomic top-level access.

## ACTION REQUIRED for build/Cargo owner
Merge `_workspace/deps_config-engineer.toml` into root `Cargo.toml`:
- `[dependencies]` += `serde_yaml = "0.9"`, `schemars = "0.8"`
- `[dev-dependencies]` += `tempfile = "3"` (tempfile not yet used in tests but reserved for future fs-based cases)

## Tests (in `src/config/loader.rs` under `#[cfg(test)]`)
- `parse_valid_minimal`
- `parse_valid_full`
- `reject_unknown_field`
- `reject_bad_version`
- `roundtrip_idempotent`
- `env_overlay_overrides`
- `explicit_path_wins`
- `toolchain_required_is_btreemap` (bonus: compile-time guard for downstream)

## Notes / deviations
- `explicit_path_wins` exercises `resolve_source` directly rather than mutating process env, because `std::env::set_var` is racy in parallel test runs. The precedence chain in `resolve_source` is linear and the explicit branch returns early, so this is a faithful check.
- JSON schema is a hand-written v1 placeholder (per task instructions) to avoid needing a build/generator binary; structure mirrors the Rust types and `deny_unknown_fields` is represented as `additionalProperties: false`.
- Migrator registry is intentionally empty; `migrate()` is fully wired and will refuse to silently no-op when a non-default `target` is requested without a registered step.
