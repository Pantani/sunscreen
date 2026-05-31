# toolchain-detector — done

## Files written
- `src/toolchain/mod.rs` — module root, re-exports.
- `src/toolchain/registry.rs` — `ToolSpec` + `known()` (anchor, solana, rustc, cargo, node, pnpm, codama, surfpool).
- `src/toolchain/detect.rs` — `CommandRunner` trait, `RealRunner`, `Status`, `ToolReport`, `detect_all()`, MockRunner tests.
- `src/cli/doctor.rs` — real implementation (overwrote stub). Provides `run(json, config_path)` and `run_compat(json)`.
- `_workspace/deps_toolchain-detector.toml` — extra deps.

## Public API
```rust
// toolchain
pub use registry::{known, ToolSpec};
pub use detect::{detect_all, CommandRunner, RealRunner, Status, ToolReport};

pub fn detect_all<R: CommandRunner>(
    runner: &R,
    specs: &[ToolSpec],
    overrides: &BTreeMap<String, String>,
) -> Vec<ToolReport>;

// cli::doctor
pub fn run(json: bool, config_path: Option<&Path>) -> anyhow::Result<i32>;
pub fn run_compat(json: bool) -> anyhow::Result<i32>; // single-arg shim
```

`Status` variants: `Ok`, `MissingRequired`, `MissingOptional`, `BelowMin`, `UnknownVersion`.
Exit code from `doctor::run`: `2` if any required tool has status `MissingRequired | BelowMin | UnknownVersion`, else `0`.

## Parallelism
Uses `std::thread::scope` (no tokio). One thread per spec.

## Dependencies consumed from other agents
- `sunscreen::config::load(config_path) -> Result<Config, _>` (config-engineer) — falls back to `Config::default()` on error.
- Shape required:
  - `Config::toolchain: ToolchainCfg`
  - `ToolchainCfg::required: BTreeMap<String, String>` (tool name -> semver string).
  Already matches existing `src/config/schema.rs`.

## New dependencies (merge into Cargo.toml)
```toml
which = "6"
regex = "1"
comfy-table = "7"
owo-colors = "4"
semver = "1"
```
Assumed already present: clap, anyhow, thiserror, serde, serde_json.

## ACTION REQUIRED for cli-architect
1. Add `pub mod toolchain;` to `src/lib.rs`.
2. Update `src/cli/root.rs`:
   - Add `--config` propagation (already there as `cli.config: Option<PathBuf>`).
   - Change `Command::Doctor => doctor::run(cli.json)` to
     `Command::Doctor => doctor::run(cli.json, cli.config.as_deref())`.
   - (Optional) Drop `doctor::run_compat` once the call site is updated; it exists as a no-op bridge.

## Tests (in `src/toolchain/detect.rs`)
- `test_all_ok`
- `test_missing_required` (verifies status + that exit-code logic would fire)
- `test_below_min`
- `test_unknown_version`
- `test_optional_missing_doesnt_fail`
- `test_override_min_version` (bonus: confirms BTreeMap overrides override default_min)

Not executed (`cargo build` skipped per instructions). QA will integrate.
