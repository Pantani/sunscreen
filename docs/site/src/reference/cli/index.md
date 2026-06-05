# CLI overview

```
sunscreen [GLOBAL_FLAGS] <COMMAND> [ARGS] [FLAGS]
```

## Top-level commands

| Command | What it does |
|---------|-------------|
| [`chain new`](./chain.md#new) | Create a new workspace (Anchor or Pinocchio) |
| [`chain build`](./chain.md#build) | Run anchor build + Codama regeneration |
| [`chain serve`](./chain.md#serve) | Supervised dev loop with local validator |
| [`chain doctor`](./chain.md#doctor) | Diagnose and repair workspace markers |
| [`scaffold`](./scaffold.md) | Add instruction, account, event, error, program, or recipe |
| [`generate`](./generate.md) | Generate IDL, Codama clients, frontend hooks |
| [`app`](./app.md) | Manage plugins (install, list, run, hook, marketplace) |
| [`doctor`](./doctor.md) | Detect and optionally repair installed toolchain versions |
| [`init`](./onboarding.md#init) | Interactive wizard for new users |
| [`examples`](./onboarding.md#examples) | Browse embedded example projects |
| [`quickstart`](./onboarding.md#quickstart) | Composite recipe shortcuts (token, nft, dao, blog) |
| [`wallet`](./onboarding.md#wallet) | Wallet helpers (new, airdrop) |
| [`deploy`](./onboarding.md#deploy) | Deploy programs to a network |
| [`learn`](./onboarding.md#learn) | Open a topic in the embedded learn index |

## Global flags

| Flag | What it does |
|------|-------------|
| `--json` | machine-readable output on stdout; human messages on stderr |
| `-v / -vv / -vvv` | verbosity (warn / info / debug) |
| `--workdir <DIR>` | override working directory |
| `--config <FILE>` | path to an alternative `sunscreen.yml` |
| `--help` | per-command help |
| `--version` | print sunscreen version |

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | success |
| `1` | unexpected error (bug) — please report |
| `2` | toolchain missing (anchor, solana, cargo, pnpm, …) |
| `3` | invalid config (`sunscreen.yml` schema violation) |
| `4` | user input conflict (resource exists, ambiguous flag, …) |
| `5` | missing workspace (no `sunscreen.yml` upward from cwd) |
| `9` | plugin runtime failure |

Full list with `next_step` strings in [Errors & exit codes](../errors.md).

## Environment variables

Sunscreen prefers explicit flags over magic environment variables. Pass `--config <FILE>` and `--workdir <DIR>` as needed. Standard Rust env vars (`RUST_LOG`, `RUST_BACKTRACE`) apply to the binary as usual.

## `--json` contract

When you pass `--json`, sunscreen guarantees:

- **One JSON object per command on stdout**, or **NDJSON** (one object per line) for streaming commands (`chain build --headless`, `chain serve --headless`).
- Human-readable status messages, errors, hints, and progress go to **stderr** — never mixed into stdout.
- Top-level shape: `{ "status": "ok"|"error", "code": "<machine-readable>", "data": {…}, "next_step": "<optional hint>" }`.
- Error objects include `error.code`, `error.message`, and `error.next_step`.

Machine integrations should always pass `--json` and parse stdout only.
