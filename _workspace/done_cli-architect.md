# cli-architect — done (Phase 0 Week 1)

## Files created
- `/Users/pantani/Desktop/projects/rust/sunscreen/Cargo.toml`
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/main.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/lib.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/cli/mod.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/cli/root.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/cli/version.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/cli/doctor.rs` (stub — toolchain-detector overrides)
- `/Users/pantani/Desktop/projects/rust/sunscreen/src/error.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/.gitignore`
- `/Users/pantani/Desktop/projects/rust/sunscreen/README.md`
- `/Users/pantani/Desktop/projects/rust/sunscreen/rustfmt.toml`
- `/Users/pantani/Desktop/projects/rust/sunscreen/clippy.toml`

## Public API
- `sunscreen::cli::execute() -> i32`
- `sunscreen::cli::version::run()`
- `sunscreen::cli::doctor::run(json: bool) -> anyhow::Result<i32>` (stub)
- `sunscreen::cli::root::{Cli, Command}` (clap derive)
- `sunscreen::error::SunscreenError` with:
  - `fn exit_code(&self) -> i32` (1 Other, 2 ToolchainMissing, 3 ConfigInvalid, 4 UserInput)
  - `fn kind_str(&self) -> &'static str`
  - Variants: `ConfigInvalid(String)`, `ToolchainMissing(String)`, `UserInput(String)`, `Other(#[from] anyhow::Error)`

## Persistent flags (global)
- `--verbose` / `-v` (count, u8)
- `--workdir <DIR>` (Option<PathBuf>)
- `--config <FILE>` (Option<PathBuf>)
- `--json` (bool)

## Subcommands
- `version` (implemented)
- `doctor` (stub — delegates to `doctor::run`)
- `scaffold`, `chain`, `generate`, `app` (stubs printing TODO)

## Dependencies declared (base set only)
- `clap = { version = "4.5", features = ["derive"] }`
- `anyhow = "1.0"`
- `thiserror = "1.0"`
- `serde = { version = "1.0", features = ["derive"] }`
- `serde_json = "1.0"`

Other agents must add their own deps (config, toolchain, templates, TUI) without removing these.

## Notes for coordinating agents
- `toolchain-detector`: overwrite `src/cli/doctor.rs`; keep signature `pub fn run(json: bool) -> anyhow::Result<i32>` or change root dispatch accordingly.
- `config-engineer`: `--config: Option<PathBuf>` is already wired on `Cli`; consume from `cli.config` in dispatch when ready.
- `template-engineer`: `Command::Scaffold` is a stub awaiting implementation.
- No `cargo build` was run (qa-integrator owns build verification).
