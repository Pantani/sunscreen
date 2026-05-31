# config-engineer — Week 2 done

## Delivered
- **Migration v0 -> v1** (`src/config/migrations/v0_to_v1.rs`)
  - Implements `Migration` trait: `from() = 0`, `to() = 1`.
  - Renames `project.id` -> `project.name` when `name` is absent; drops legacy
    `id` if both exist so v1's `deny_unknown_fields` does not reject.
  - No-op on docs without `project`. Includes 3 unit tests.
- **Migrations module wiring** (`src/config/migrations/mod.rs`,
  updated `src/config/mod.rs`) and registration in
  `src/config/migrator.rs::registry()`.
- **`CURRENT_SCHEMA_VERSION` constant** exported from `src/config/mod.rs`.
- **Loader integration** (`src/config/loader.rs`):
  - New `upgrade(raw)` step runs after `parse_yaml` and before
    `apply_env_overlay` / `materialize`.
  - Reads `version` from the raw YAML; if `< CURRENT_SCHEMA_VERSION`,
    calls `migrate(raw, CURRENT_SCHEMA_VERSION)`.
  - New `ConfigError::Migration(String)` variant for failure surface.
- **Fixture** `tests/fixtures/config/v0/old_format.yml` (version 0,
  `project.id: myproj`, plus toolchain + scaffolding fields).
- **New loader tests**:
  - `migration_v0_to_v1_renames_id_to_name`
  - `migration_preserves_other_fields`
  - `already_at_current_version_no_migration`
- **PartialEq/Eq derives** on `Config`, `ProjectCfg`, `ToolchainCfg`,
  `ScaffoldingCfg`.
- **Roundtrip test reinforced** to assert structural `Config` equality
  via the new `PartialEq` derive (in addition to byte-for-byte YAML).

## Untouched (per scope)
- `src/cli/`, `src/toolchain/`, `src/templates/`, `.github/`, `docs/`,
  `Cargo.toml`. `src/config/schemas/sunscreen.v1.json` unchanged (v1
  shape did not move; migration is purely raw-YAML).

## Verification
- `cargo build`: clean.
- `cargo test --lib config::`: 14 passed, 0 failed.
- `cargo clippy --all-targets -- -D warnings`: clean.

## Key files (absolute paths)
- /Users/pantani/Desktop/projects/rust/sunscreen/src/config/mod.rs
- /Users/pantani/Desktop/projects/rust/sunscreen/src/config/loader.rs
- /Users/pantani/Desktop/projects/rust/sunscreen/src/config/migrator.rs
- /Users/pantani/Desktop/projects/rust/sunscreen/src/config/migrations/mod.rs
- /Users/pantani/Desktop/projects/rust/sunscreen/src/config/migrations/v0_to_v1.rs
- /Users/pantani/Desktop/projects/rust/sunscreen/src/config/schema.rs
- /Users/pantani/Desktop/projects/rust/sunscreen/tests/fixtures/config/v0/old_format.yml
