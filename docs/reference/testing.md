# Sunscreen Testing Harness

This project uses a tiered validation model so fast CI stays deterministic while real Solana validation remains explicit.

## Tiers

| Tier | Purpose | Command |
|---|---|---|
| Offline deterministic | fmt, clippy, feature gates, full Rust tests, binary smoke | `bash scripts/integration-heavy.sh` |
| Generated workspace compile | Cargo-check generated workspaces and scaffolded programs | `SUNSCREEN_COMPILE_TESTS=1 bash scripts/integration-heavy.sh` |
| Real Anchor/Codama | Anchor/Solana/Codama/pnpm/node against ignored integration tests | `SUNSCREEN_REAL_TOOLCHAIN=1 bash scripts/integration-heavy.sh` |
| Pinocchio SBF | Real Solana SBF build for Pinocchio workspaces | manual tier in `.claude/skills/sunscreen-test-harness/SKILL.md` |
| Serve runtime | Surfpool or `solana-test-validator`, watcher, NDJSON events, teardown | manual tier in `.claude/skills/sunscreen-test-harness/SKILL.md` |
| Plugin runtime | Lifecycle, sandbox, stdio JSON-RPC, gRPC contract, marketplace | `cargo test --locked --test app_lifecycle -- --nocapture` |
| Release distribution | Release binary and `cargo dist plan` | `SUNSCREEN_DIST=1 bash scripts/integration-heavy.sh` |
| Flake/perf | Repeat CLI smoke and run cold-start bench separately | `SUNSCREEN_FLAKE_RUNS=5 bash scripts/integration-heavy.sh` |

## False Green Policy

Ignored or gated tests do not count as real coverage when they skip because a tool is missing. A real-toolchain run must prove that `anchor`, `solana`, `solana-test-validator`, `pnpm`, `node`, `codama`, `cargo`, and `rustc` were available before accepting `tests/integration_anchor.rs` as executed.

The fake-toolchain integration tests remain valuable: they prove CLI contracts, JSON/NDJSON shapes, sandbox behavior, plugin runtime boundaries, and command-group flows without depending on external network or local Solana installs. They do not replace the real toolchain gate.

## Harness Team

The durable harness lives in:

- `.claude/agents/test-strategist.md`
- `.claude/agents/offline-ci-owner.md`
- `.claude/agents/real-anchor-codama-owner.md`
- `.claude/agents/pinocchio-sbf-owner.md`
- `.claude/agents/serve-runtime-owner.md`
- `.claude/agents/plugin-runtime-qa.md`
- `.claude/agents/frontend-codegen-owner.md`
- `.claude/agents/release-distribution-qa.md`
- `.claude/agents/flake-perf-auditor.md`
- `.claude/skills/sunscreen-test-harness/SKILL.md`
- `.agents/skills/sunscreen-test-harness/SKILL.md`

Use `sunscreen-test-harness` whenever the task is to validate the app with heavy integration, real toolchain, release QA, stress, or anti-flake coverage.
