# Errors & exit codes

Every sunscreen error has three things: a numeric **exit code**, a stable **string code**, and an actionable **next_step**.

## Exit codes

| Exit | Code | Meaning |
|------|------|---------|
| `0` | — | success |
| `1` | `unexpected` | a bug — please report with `RUST_BACKTRACE=1` output |
| `2` | `toolchain_missing` | a required external tool (anchor, solana, …) was not found |
| `3` | `invalid_config` | `sunscreen.yml` failed schema validation |
| `4` | `user_input` | conflicting / ambiguous / unrecognized user request |
| `5` | `missing_workspace` | no `sunscreen.yml` found upward from cwd |
| `9` | `plugin_runtime` | a plugin crashed, violated the sandbox, or returned a protocol error |

## Common error codes (string-level)

These are stable identifiers you can match on in scripts. Subset, full list lives in [`src/error.rs`](https://github.com/Pantani/sunscreen/blob/main/src/error.rs).

### Toolchain (exit 2)

| `code` | `next_step` |
|--------|-------------|
| `toolchain_missing.anchor` | Install via AVM: `cargo install --git https://github.com/coral-xyz/anchor avm --locked && avm install latest` |
| `toolchain_missing.solana` | `sh -c "$(curl -sSfL https://release.solana.com/stable/install)"` |
| `toolchain_missing.pnpm` | `npm i -g pnpm` |
| `toolchain_missing.cargo_build_sbf` | Comes with the `solana` install — re-run the solana installer |

### Invalid config (exit 3)

| `code` | Trigger |
|--------|---------|
| `invalid_config.schema` | Top-level shape doesn't match v1 |
| `invalid_config.field.<path>` | A specific field is wrong (path follows the YAML, e.g. `invalid_config.field.plugins.0.version`) |
| `invalid_config.version_unknown` | YAML declares a `version` newer than this binary supports |

### User input (exit 4)

| `code` | Trigger |
|--------|---------|
| `user_input.path_conflict` | Target directory exists (sunscreen refuses to overwrite without `--force`) |
| `user_input.symbol_exists` | A scaffold target (instruction, account, …) is already defined |
| `user_input.recipe_preflight` | A recipe's preflight detected conflicts |
| `user_input.ambiguous_flag` | Two flags conflict (e.g. `--dry-run` + `--force`) |
| `user_input.semver` | `--version` argument isn't valid semver |
| `network.rate_limited` | Faucet returned 429 |
| `network.unreachable` | RPC didn't respond |

### Missing workspace (exit 5)

| `code` | Trigger |
|--------|---------|
| `missing_workspace` | No `sunscreen.yml` in cwd or ancestors |

### Plugin runtime (exit 9)

| `code` | Trigger |
|--------|---------|
| `plugin_runtime.protocol` | Plugin emitted invalid JSON-RPC |
| `plugin_runtime.sandbox` | Plugin tried to write outside workspace or open a forbidden network connection |
| `plugin_runtime.crash` | Plugin exited unexpectedly (non-zero, no response) |
| `plugin_runtime.manifest` | Plugin manifest invalid or unreadable |

## `next_step` contract

Every error includes a `next_step` field — a short, imperative sentence telling you what to do. This is part of the stable surface (tested by `tests/errors_contract.rs`).

Human-readable form:
```
error: user_input.symbol_exists: instruction "create_post" already exists (exit 4)
next_step: Pass --force to overwrite, or pick a different name.
```

JSON form:
```json
{
  "status": "error",
  "error": {
    "code": "user_input.symbol_exists",
    "message": "instruction \"create_post\" already exists",
    "next_step": "Pass --force to overwrite, or pick a different name"
  }
}
```

## Reporting bugs

Exit `1` (`unexpected`) is a bug. Include in the issue:

- `sunscreen --version`
- The full command you ran
- Output with `RUST_BACKTRACE=1`
- `sunscreen doctor --json`
