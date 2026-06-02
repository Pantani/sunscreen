---
name: sunscreen-test-harness
description: Use whenever the user asks for real tests, heavy validation, real integration, test harness work, end-to-end QA, stress, anti-flake, release QA, cargo-dist, real Anchor/Solana/Codama, real Pinocchio SBF, Surfpool/test-validator, frontend typecheck, plugin runtime, CI hardening, or proof that the sunscreen app actually works. Also use to re-run, update, fix, expand, or audit sunscreen test waves.
---

# Sunscreen Test Harness

Orchestrates the heavy-validation team for `sunscreen`. The goal is to prove real behaviour without turning every test into a fragile network/toolchain dependency. Always separate offline smoke, gated heavy-local, and what actually exercised a real Solana toolchain.

## Team

- `test-harness-orchestrator`: round leader, reads `summary.json`, delegates tiers, and consolidates status.
- `qa-integrator`: quality lead and round closer.
- `test-strategist`: risk matrix, tiers, acceptance criteria, and owners.
- `offline-ci-owner`: fmt/clippy/test/build/no-default and command-group smokes.
- `real-anchor-codama-owner`: real Anchor/Solana/Codama/pnpm/node.
- `pinocchio-sbf-owner`: Pinocchio with real `cargo build-sbf`.
- `serve-runtime-owner`: Surfpool/test-validator, watcher, ports, build trigger, and teardown.
- `plugin-runtime-qa`: manifest, stdio JSON-RPC, gRPC, sandbox, marketplace, and dynamic scaffold.
- `frontend-codegen-owner`: hooks/clients, Next/Vite, pnpm install, and typecheck.
- `release-distribution-qa`: cargo-dist, release binary, installer, changelog, docs, and completions.
- `flake-perf-auditor`: repetition, timeouts, cold-start, and instability.

## Phase 0: Current State

1. Read `AGENTS.md`, `CLAUDE.md`, `ROADMAP.md`, `.github/workflows/ci.yml`, `tests/**`, `scripts/integration-heavy.sh`, and `git status`.
2. Confirm whether the request is a test round, a harness expansion, a CI audit, or a release validation.
3. If logs already exist under `_workspace/test-harness/`, treat them as history, not as current proof.
4. Preserve the user's local changes.

## Execution Mode

Use hybrid mode:

- If subagents are available and the user asked for harness/team work, delegate independent audits to the specialists.
- If subagents are not available, run locally following the ownership map above.
- Never mark a tier as passing just because an ignored/skipped test returned success.

## Orchestrator Flow

1. `test-harness-orchestrator` opens the round and records the scope in `_workspace/test-harness/orchestrator-report.md`.
2. `test-strategist` builds the risk matrix when the request is broad.
3. `offline-ci-owner` runs `bash scripts/integration-heavy.sh`.
4. The orchestrator reads the most recent `*.summary.json` and classifies each tier.
5. Skipped or blocked tiers are delegated to the right specialists only when the user requested that validation.
6. `qa-integrator` closes the round with the final report, evidence, and smallest next step.

## Test Tiers

### Tier 1: Offline Deterministic Gate

Runs on any machine and in normal CI.

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo check --locked --no-default-features --all-targets
cargo test --locked --all --all-features --no-fail-fast
cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
cargo test --locked --test compile_generated_workspace
cargo build --locked --release --all-features
```

Acceptance: everything passes, no snapshot drift, no clippy warnings, no broken feature gate.

### Tier 2: Generated Workspace Compile Gate

Confirms generated workspaces still compile with real dependencies / local cache when applicable.

```bash
SUNSCREEN_COMPILE_TESTS=1 cargo test --locked --test compile_generated -- --nocapture
cargo test --locked --test compile_generated_workspace -- --nocapture
```

Acceptance: the suites actually run. If `compile_generated` skips because of a missing cache/dependency, log a blocker.

### Tier 3: Real Anchor And Codama Gate

Validates real Anchor/Solana/Codama/pnpm/node.

```bash
SUNSCREEN_REAL_TOOLCHAIN=1 bash scripts/integration-heavy.sh
cargo test --locked --test integration_anchor -- --ignored --nocapture
SUNSCREEN_FRONTEND_COMPILE_TESTS=1 cargo test --locked --test generate generated_frontend_hooks_typecheck_vanilla_next_project_when_dependencies_are_installed -- --ignored --nocapture
```

Acceptance: `anchor`, `solana`, `pnpm`, `node`, `cargo`, `rustc`, and `codama` were all found; the ignored tests exercised real scenarios instead of just printing SKIP.

### Tier 4: Pinocchio SBF Gate

Validates Pinocchio with real Solana SBF.

```bash
ROOT="$(pwd)"
cargo build --locked --release
tmp="$(mktemp -d)"
"$ROOT/target/release/sunscreen" chain new real_pin --framework pinocchio --frontend none --path "$tmp/real_pin"
(cd "$tmp/real_pin" && "$ROOT/target/release/sunscreen" --json chain build --headless)
```

Acceptance: real `cargo build-sbf` runs on the Pinocchio workspace and the Anchor-only guards stay unchanged.

### Tier 5: Serve Runtime Gate

Validates runtime, watcher, and teardown with Surfpool/test-validator when the machine has the toolchain.

```bash
cargo test --locked --test chain_serve -- --nocapture
cargo test --locked --test runtime_serve_loop --test runtime_watch_loop --test runtime_validator -- --nocapture
```

Acceptance: the real runtime comes up, ports become ready when verifiable, the watcher triggers builds, NDJSON events are parseable, and Ctrl-C terminates the children.

### Tier 6: Plugin Runtime Gate

Validates runtime, watcher, plugin lifecycle, and dynamic commands.

```bash
cargo test --locked --test app_lifecycle -- --nocapture
cargo test --locked plugin::stdio plugin::grpc plugin::sandbox plugin::manifest
./target/release/sunscreen app marketplace --json
```

Acceptance: a local plugin runs, sandbox rejects traversal, dynamic app/scaffold keep their exit codes, and gRPC is reported as a contract/stub if no real runtime fixture exists yet.

### Tier 7: Release And Install Gate

Validates the binary users would download.

```bash
cargo build --locked --release --all-features
./target/release/sunscreen --help
./target/release/sunscreen version
SUNSCREEN_DIST=1 bash scripts/integration-heavy.sh
cargo dist plan
```

Acceptance: the release binary works, the dist plan matches the expected targets, and changelog/notes/docs stay consistent. Do not create a tag/release without explicit instruction.

### Tier 8: Flake And Performance Gate

Re-runs critical suites and measures cold-start.

```bash
SUNSCREEN_FLAKE_RUNS=5 bash scripts/integration-heavy.sh
RUNS=30 bash scripts/bench.sh
```

Acceptance: no intermittent failures; cold-start p95 stays inside the documented target or any regression is reported.

## Standard Runner

Prefer the single runner for local rounds:

```bash
bash scripts/integration-heavy.sh
SUNSCREEN_COMPILE_TESTS=1 bash scripts/integration-heavy.sh
SUNSCREEN_REAL_TOOLCHAIN=1 SUNSCREEN_COMPILE_TESTS=1 bash scripts/integration-heavy.sh
SUNSCREEN_PINOCCHIO_SBF=1 bash scripts/integration-heavy.sh
SUNSCREEN_FRONTEND_COMPILE_TESTS=1 bash scripts/integration-heavy.sh
SUNSCREEN_REAL_TOOLCHAIN=1 SUNSCREEN_PINOCCHIO_SBF=1 SUNSCREEN_FRONTEND_COMPILE_TESTS=1 SUNSCREEN_DIST=1 SUNSCREEN_FLAKE_RUNS=5 bash scripts/integration-heavy.sh
```

Variables:

- `SUNSCREEN_COMPILE_TESTS=1`: enables the gated compile tests.
- `SUNSCREEN_REAL_TOOLCHAIN=1`: requires a real toolchain and runs `integration_anchor --ignored`.
- `SUNSCREEN_PINOCCHIO_SBF=1`: requires Solana/Cargo SBF and runs the real Pinocchio build.
- `SUNSCREEN_FRONTEND_COMPILE_TESTS=1`: requires Node/pnpm and typechecks the generated frontend hooks.
- `SUNSCREEN_DIST=1`: requires `cargo dist` and runs `cargo dist plan`.
- `SUNSCREEN_FLAKE_RUNS=N`: re-runs the CLI smoke `N` times.
- `SUNSCREEN_HEAVY_LOG_DIR=path`: changes the log directory.

## Reporting

Always report:

- Commands executed.
- Real tool versions.
- Tiers that passed, failed, were skipped, or were blocked.
- Evidence that ignored/gated tests actually ran.
- Files/logs under `_workspace/test-harness/`.
- The round's `*.summary.json`, with per-tier status.
- The smallest next step that converts a blocker into real coverage.

## False Green Rules

- `#[ignore]` + `--ignored` is not real coverage if the body printed `SKIP`.
- A fake `PATH` covers the offline contract, not real Anchor/Solana behaviour.
- `cargo test --all` can hide suites gated by env vars; record that explicitly.
- `compile_generated_workspace` uses local shims; it does not substitute real Anchor/Pinocchio dependencies.
- A local `cargo dist plan` is not equivalent to a published release.
- The plugin gRPC path may be covered as contract/stub; do not call that a real transport without a runtime fixture.
- `doctor --json` reporting a missing tool is a diagnostic, not a CLI failure.

## Test Scenarios

Happy path:

1. The user asks to "validate everything with heavy tests".
2. Run `bash scripts/integration-heavy.sh`.
3. If the user wants a real toolchain, run it with `SUNSCREEN_REAL_TOOLCHAIN=1`.
4. Deliver the per-tier report.

Error flow:

1. `SUNSCREEN_REAL_TOOLCHAIN=1` fails because `anchor` or `codama` is missing.
2. Mark it as `blocked_by_missing_tool`.
3. Do not call the round green; propose installing/provisioning the toolchain or moving that tier to a dedicated runner.
