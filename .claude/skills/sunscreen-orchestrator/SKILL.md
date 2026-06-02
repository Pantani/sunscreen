---
name: sunscreen-orchestrator
description: Orchestrates the implementation and validation team for the sunscreen CLI (Rust + Solana tooling). Use whenever the user asks to implement, continue, expand, fix, refactor, update, validate, review, re-run, or finish any part of the sunscreen CLI — including "real tests", "test harness", "heavy integration", "real toolchain", "real Anchor", "real Codama", "Pinocchio SBF", "serve runtime", "plugin runtime", "release QA", "next phase", "pending work", "Phase 6", "plugins", "app", "marketplace", "stdio", "gRPC", "Phase 7", "Pinocchio", "Phase 8", "CI", "integration", "chain serve", "chain build", "generate", "codama", "scaffold", "recipes", "crud", "spl-token", "metaplex-nft", "onboarding", "quickstart", "doctor", "markers", "run it again", "fix it", "update roadmap", or any ongoing work on the project. Coordinates cli-architect, config-engineer, toolchain-detector, template-engineer, docs-writer, qa-integrator, and the sunscreen-test-harness team. Do not use it for simple conceptual questions about Solana — only for concrete changes to the sunscreen codebase.
---

# Sunscreen Orchestrator

Coordinates implementation of the `sunscreen` CLI (Rust, inspired by Ignite CLI, targeting the Solana ecosystem). Live source of truth: `ROADMAP.md`. `docs/adr/ADR-0001-solis-cli.md` and `IMPLEMENTATION-KICKOFF.md` are historical context; preserve the strategic decisions but translate every Go/solis reference to Rust/sunscreen.

## Phase 0: Context

Before doing anything:

1. Re-read `CLAUDE.md`, `AGENTS.md`, `ROADMAP.md`, and `git status`.
2. Treat `ROADMAP.md` as the live source of scope/status.
3. If the harness, AGENTS/CLAUDE, and the roadmap have drifted apart, sync them in the same PR.
4. Preserve the user's local changes.

## Current State

- Phase 0, Phase 1, Phase 2, Phase 3, Phase 4, and Phase 5 are complete.
- Phase 2 has no known carry-overs: marker hardening, the no-accounts instruction compile test, and the R5 polish are all closed.
- Phase 3 is complete: `chain build`, `chain serve`, watcher, supervised runtime, Surfpool→test-validator fallback, frontend notify, serve model, and Ctrl-C teardown.
- Phase 4 is complete: `generate {clients, idl, frontend-hooks}`, the Codama wrapper, deterministic IDL export, React/Solid Query hooks, and the shared pipeline.
- Phase 5 is complete: `scaffold {crud, spl-token, metaplex-nft}` as composite recipes layered on top of the Phase 2 scaffolders.
- Phase 5.5 is complete: `init`, `examples`, `quickstart`, `wallet`, `deploy`, `learn`, and errors carrying `next_step`.
- Phase 6 is complete: `app` lifecycle, `sunscreen-plugin.json` manifest, runtime manager, stdio JSON-RPC, gRPC contract, sandbox/trust model, local/reference marketplace, hooks, and the dynamic `scaffold <noun>` command.
- Phase 7 is complete: `chain new --framework pinocchio`, the `pinocchio-minimal` template, Anchor-free preflight, `chain build` with `cargo build-sbf`, and clear guards on Anchor-only scaffold/generate commands.
- Phase 8 (Distribution & Docs / v1.0) is the next phase: full multi-OS cargo-dist, docs site, shell completions, changelog/SemVer, and release polish.
- The Ignite-style CLI integration layer already exists and runs in CI: `tests/integration_{chain,scaffold,generate,onboarding}.rs` with `tests/support/mod.rs`.
- The main CI job already runs explicit integration smoke tests, `--locked`, a `--no-default-features` check, read-only permissions, concurrency, and timeouts.
- `sunscreen-test-harness` exists for heavy validation: offline deterministic gate, generated workspace compile, real Anchor/Codama, real Pinocchio SBF, serve runtime, plugin runtime, frontend typecheck, release QA, and flake/perf.

## Execution

**Execution mode: hybrid.** Spawn subagents only when the environment supports them and the request authorises harness/team work. In Codex, prefer `multi_agent_v1.spawn_agent` with the QA/docs/architecture agents; without subagents, run locally following the same ownership map below.

### Owners by area

- `cli-architect`: `src/cli/**`, command contracts, exit codes.
- `config-engineer`: `src/config/**`, schemas, migrations.
- `toolchain-detector`: `src/toolchain/**`, `sunscreen doctor`.
- `template-engineer`: `src/templates/**`, `templates/**`, golden tests, and marker templates.
- `docs-writer`: ADRs, `ROADMAP.md`, reference docs.
- `qa-integrator`: cross-module checks, fmt/clippy/build/test, and binary command runs.
- `test-harness-orchestrator`: leads `sunscreen-test-harness` rounds, reads `summary.json`, and consolidates per-tier status.
- `test-strategist`: risk matrix, tiers, and test-harness handoff.
- `offline-ci-owner`: deterministic gates and fake-toolchain smokes.
- `real-anchor-codama-owner`: real Anchor/Solana/Codama/pnpm/node.
- `pinocchio-sbf-owner`: Pinocchio with real `cargo build-sbf`.
- `serve-runtime-owner`: Surfpool/test-validator, watcher, and teardown.
- `plugin-runtime-qa`: plugins, stdio/gRPC, sandbox, and marketplace.
- `frontend-codegen-owner`: frontend hooks/clients and typecheck.
- `release-distribution-qa`: cargo-dist, release binary, installers, docs, and completions.
- `flake-perf-auditor`: re-runs, timeouts, cold-start, and flakes.

## Checklists

### Phase 2 closure

- `tests/rustfmt_roundtrip.rs` preserves every documented segment.
- `chain doctor --fix-markers` repairs `dispatch` and `error_variants` only in safe cases.
- `tests/compile_generated.rs` covers 25 generated-workspace scenarios.
- `tests/integration_anchor.rs` contains 5 real scenarios that are ignored by default with a toolchain skip.

### Phase 3 closure

- `chain build --headless` emits NDJSON and runs build -> Codama.
- The watcher debounces and triggers the pipeline with relative paths.
- `chain serve` launches the Surfpool/test-validator runtime with a fallback when implicit Surfpool is missing.
- Frontend notify touches `app/.sunscreen/reload`.
- `src/tui/serve_model.rs` covers the validator/build/faucet/frontend/logs panels at 80x24.
- Ctrl-C stops the Unix process group with a SIGKILL fallback.

### Phase 4 closure

- `src/codegen/{codama,codama_config,idl,frontend_hooks}.rs` exists and is used by the CLI.
- `sunscreen generate clients`, `generate idl`, and `generate frontend-hooks` are implemented.
- `chain build` and `chain serve` reuse the shared Codama wrapper.
- React Query and Solid Query hooks are deterministic and covered by tests.

### Phase 5 closure

- `sunscreen scaffold crud <Resource> --program <p>` generates state, `create/read/update/delete`, events, errors, a TS test, and an optional frontend hook.
- `sunscreen scaffold spl-token <Name> --program <p>` generates an internal SPL token slice.
- `sunscreen scaffold metaplex-nft <Name> --program <p>` generates an internal Token Metadata slice.
- Recipes dry-run the primitives before writing and keep a single JSON object under `--json`.
- `docs/reference/recipes.md`, `ROADMAP.md`, `AGENTS.md`, and `CLAUDE.md` reflect Phase 5 as closed.

### Phase 5.5 closure

- The `init` wizard and `--non-interactive` mode reuse `chain new`.
- `quickstart {token,nft,dao,blog}` composes the Phase 5 recipes.
- `examples`, `wallet`, `deploy`, `learn`, and the `next_step` contract are implemented.
- ADR-0002 covers `PathConflict` and `Network`.

### Phase 6 closure

- `sunscreen app commands` lists dynamic commands from local manifests without starting any processes.
- `sunscreen app run <plugin> <command> -- ...` runs `kind=app` commands over stdio JSON-RPC with `Content-Length` framing.
- `sunscreen scaffold <noun> -- ...` routes `kind=scaffold` commands declared by plugins without adding each noun to the core.
- `sunscreen app marketplace` lists the reference plugins `spl-token-2022` (gRPC) and `yellowstone-indexer` (stdio).
- `src/plugin/{manifest,manager,stdio,grpc,sandbox,marketplace}.rs` exists and keeps a single internal interface across transports.
- `proto/plugin.proto` defines `initialize`, `capabilities`, `run_command`, `run_hook`, and `shutdown`.
- Runtime/sandbox failures use exit 9 (`plugin_runtime`); exit 7 remains reserved for `path_conflict`.
- `tests/app_lifecycle.rs` covers lifecycle + local runtime, non-zero failure, sandbox traversal, and dynamic scaffold.

### Phase 7 closure

- `sunscreen chain new <name> --framework pinocchio` creates a Pinocchio workspace with no `Anchor.toml` and no `anchor-lang`.
- `templates/workspace/pinocchio-minimal/` contains a Cargo workspace, a `no_std`/BPF-aware program, and a `sunscreen.yml` with `project.framework: pinocchio`.
- Pinocchio preflight requires Rust/Cargo/Solana but not Anchor; the JS frontend still requires Node/pnpm.
- `chain build --headless` on a Pinocchio workspace emits `pinocchio_build`, runs `cargo build-sbf`, and reports `framework: pinocchio` and `codama: false`.
- Built-in scaffolders and `generate` refuse Pinocchio with a `user_input` error before writing; plugin-backed `scaffold <noun>` remains available.
- `docs/adr/ADR-0006-pinocchio-bootstrap.md`, `docs/reference/pinocchio.md`, `ROADMAP.md`, `AGENTS.md`, and `CLAUDE.md` reflect Phase 7 as closed.

### Phase 8 / CI QA

- CI must run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --all --all-features --no-fail-fast`, `cargo build --locked --release --all-features`, and `cargo check --locked --no-default-features --all-targets`.
- The Ignite-style smoke must explicitly run the four groups `integration_chain`, `integration_scaffold`, `integration_generate`, and `integration_onboarding`, plus `app_lifecycle` for the plugin runtime.
- Real Anchor/Codama tests in `tests/integration_anchor.rs` remain gated/ignored by default; when they run, report whether they actually validated or merely skipped because the toolchain was missing.
- For "real tests" requests, invoke `sunscreen-test-harness` and run `bash scripts/integration-heavy.sh`; only set `SUNSCREEN_REAL_TOOLCHAIN=1`, `SUNSCREEN_COMPILE_TESTS=1`, `SUNSCREEN_PINOCCHIO_SBF=1`, `SUNSCREEN_DIST=1`, and `SUNSCREEN_FLAKE_RUNS=N` when that tier is explicitly requested.
- No false greens: fake toolchain, skipped `#[ignore]` tests, `compile_generated` without the env var, and gRPC stubs do not count as real ecosystem validation.
- Phase 8 still has gaps: docs site in CI, completions, changelog/SemVer, Windows/full cargo-dist, Homebrew/binstall, and `cargo dist plan` validation.

## Report

Summarise to the user:

- Files created/changed grouped by module.
- Status of `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo build --locked --release --all-features`, and `cargo test --locked --all --all-features --no-fail-fast`.
- Status of the smoke `cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding`.
- Status of the feature gate `cargo check --locked --no-default-features --all-targets`.
- Remaining roadmap items.
- Suggested next step.

## Error Handling

- A step fails -> 1 retry with the error message.
- It fails again -> report the blocker with the command, output, and likely file.
- Design conflict -> keep the alternatives in the report and pick the path that keeps `ROADMAP.md` coherent.
