---
name: sunscreen-flow-tests
description: >
  Complete end-to-end user journey tests for sunscreen CLI. Runs the real compiled
  binary from a temp directory and validates full flows: zero-to-NFT, zero-to-token,
  zero-to-smart-contract, doctor check. Use this whenever: testing if the CLI works
  end-to-end, validating a fix didn't break a full flow, "testa o fluxo completo",
  "run full flow tests", "does quickstart really work", "integration tests pass but
  real usage fails", "smoke test the binary", before any release or PR merge.
  These tests catch bugs that unit tests miss because they run the binary as a real
  user would: from any directory, with relative paths, with no --program flag, etc.
---

# sunscreen Flow Tests

The standard for a passing flow: a developer who never touched Solana runs the
command and it works, with zero debugging required.

## Binary location

Always use the release binary. Build first if the binary is stale:
```bash
SUNSCREEN_BIN=$(pwd)/target/release/sunscreen
cargo build --release -q
```

## Flow scripts (bundled)

All flows live in `scripts/`. Run them from the sunscreen project root:

| Script | Flow | Toolchain required |
|--------|------|--------------------|
| `flow-nft.sh` | zero-to-NFT (quickstart + scaffold) | offline (SKIP_PREFLIGHT) |
| `flow-token.sh` | zero-to-token (quickstart token + scaffold) | offline |
| `flow-smart-contract.sh` | blank workspace → scaffold program → build | offline + optional Anchor |
| `flow-runner.sh` | runs all offline flows and reports a summary | offline |

## Environment variables

| Var | Meaning |
|-----|---------|
| `SUNSCREEN_SKIP_PREFLIGHT=1` | skip toolchain checks — required for all offline flows |
| `SUNSCREEN_REAL_TOOLCHAIN=1` | allow flows that need real Anchor/Solana/pnpm |
| `SUNSCREEN_BIN` | path to the binary; defaults to `./target/release/sunscreen` |
| `FLOW_TMPDIR` | override temp dir (default: auto-generated under /tmp) |

## Anatomy of a flow script

Each script:
1. Creates an isolated temp dir under `/tmp/sunscreen-flow-<name>-<timestamp>/`
2. Sets `trap` to clean up on exit (even on failure)
3. Runs commands with `assert_cmd` — which prints PASS/FAIL and exits non-zero on failure
4. Prints a summary line per step: `[PASS] step description` or `[FAIL] step (exit N): output`
5. Exits 0 only when ALL steps pass

## Running manually

```bash
# All offline flows:
bash .claude/skills/sunscreen-flow-tests/scripts/flow-runner.sh

# Single flow:
bash .claude/skills/sunscreen-flow-tests/scripts/flow-nft.sh

# With real toolchain (needs anchor, solana, pnpm):
SUNSCREEN_REAL_TOOLCHAIN=1 bash .claude/skills/sunscreen-flow-tests/scripts/flow-smart-contract.sh
```

## Adding a new flow

1. Copy `flow-nft.sh` as the template.
2. Define steps as `assert_cmd <exit_code> <description> <cmd...>`.
3. Use `SUNSCREEN_SKIP_PREFLIGHT=1` for offline steps.
4. Register the new script in `flow-runner.sh`.
5. Add it to this table and to the agent that owns it.

## Acceptance criteria

A flow PASSES when:
- Every `assert_cmd` exits with the expected code (0 for success, 2 for missing tool, etc.)
- Files expected to exist after the command are present on disk
- No step emits an unhandled Rust panic or `thread 'main' panicked`

A flow FAILS when:
- Any step exits with a different code than expected
- Any file that should have been created is absent
- The binary emits a panic

## What flows do NOT test

- Unit-level logic (covered by `cargo test`)
- Real Anchor compilation (covered by `real-anchor-codama-owner`)
- Real Solana deploy (requires funded wallet + network — outside offline scope)
- Plugin gRPC transport (covered by `plugin-runtime-qa`)

When a real deploy test is needed, set `SUNSCREEN_REAL_TOOLCHAIN=1` and ensure
`anchor`, `solana`, and `pnpm` are installed. The `flow-smart-contract.sh` script
accepts this mode and will attempt a real `anchor build`.
