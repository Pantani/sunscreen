# sunscreen

> Solana CLI scaffolding & orchestration tool — written in Rust, inspired by Ignite CLI.

`sunscreen` is a greenfield CLI that streamlines building on Solana: incremental scaffolding of Anchor 1.0 programs, dev-loop orchestration (Surfpool + Codama + frontend), and a plugin system for extending the toolchain.

**Status:** Phase 2 — incremental scaffolding (~95%, R4 shipped, R5 polish in flight). Not yet published to crates.io. Live tracker: [`ROADMAP.md`](ROADMAP.md).

---

## Why

Building on Solana today means stitching together `anchor`, `solana`, `surfpool`, `codama`, `cargo`, and a frontend toolchain by hand. `sunscreen` provides a single, opinionated entrypoint that:

- **Scaffolds** Anchor programs from typed, deterministic templates.
- **Orchestrates** the dev loop (local validator, IDL → client generation, frontend hot-reload).
- **Diagnoses** your environment so version drift surfaces before it bites.
- **Extends** via a plugin protocol so teams can ship their own commands.

The full design rationale lives in [`docs/adr/ADR-0001-solis-cli.md`](docs/adr/ADR-0001-solis-cli.md). CLI conventions, marker protocol, and the beginner-onboarding surface live in [`docs/adr/`](docs/adr/).

---

## Install

From source (requires Rust 1.75+):

```bash
git clone https://github.com/Pantani/sunscreen
cd sunscreen
cargo install --path .
```

Verify:

```bash
sunscreen --version
```

---

## Quick start

```bash
# Check your local toolchain (anchor, solana, cargo, node, pnpm, surfpool, codama)
sunscreen doctor

# Print version (supports --json)
sunscreen version --json

# Create a new Anchor workspace (Phase 1)
sunscreen chain new my-dapp

# Scaffold inside an existing workspace (Phase 2)
sunscreen scaffold program  my_program
sunscreen scaffold instruction transfer
sunscreen scaffold account  Vault
sunscreen scaffold event    Transferred
sunscreen scaffold error    InsufficientFunds

# Repair drifted markers in-place
sunscreen chain doctor --fix-markers
```

### Global flags

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Increase logging verbosity (`-v`, `-vv`, `-vvv`) |
| `--workdir <DIR>` | Override working directory |
| `--config <FILE>` | Path to a `sunscreen.yml` config file |
| `--json` | Emit structured JSON output where supported |

---

## Commands

| Command | Status | Description |
|---------|--------|-------------|
| `version` | ✅ | Print sunscreen version (text or JSON) |
| `doctor` | ✅ | Diagnose toolchain & environment |
| `chain new` | ✅ | Bootstrap a compilable Anchor workspace (+ frontend variants) |
| `chain doctor --fix-markers` | ✅ | Repair drifted scaffolder markers in appendable hosts |
| `scaffold program` | ✅ | Add a new program crate to an existing workspace |
| `scaffold instruction` | ✅ | Add an instruction (idempotent, marker-based, `--dry-run`, `--json`) |
| `scaffold account` | ✅ | Add an account struct |
| `scaffold event` | ✅ | Add an event |
| `scaffold error` | ✅ | Add an error variant |
| `chain serve` / `chain build` | 📋 | Dev loop (Surfpool + watcher + codama + TUI) — Phase 3 |
| `generate` | 🚧 stub | Code generation (clients, IDL, frontend hooks) — Phase 4 |
| `app` | 🚧 stub | Application lifecycle commands |

---

## Configuration

`sunscreen` reads `sunscreen.yml` from the working directory. Environment variables prefixed with `SUNSCREEN_` override config keys. Migrations between schema versions are versioned and round-trip deterministic.

See [`src/config/`](src/config/) for the schema and [`docs/adr/ADR-0002-cli-design-conventions.md`](docs/adr/ADR-0002-cli-design-conventions.md) for CLI conventions.

---

## Development

```bash
cargo build              # build
cargo test               # run unit + golden tests
cargo clippy -- -D warnings
cargo fmt --check
cargo bench --bench cold_start
```

Project layout:

```
src/
  cli/         # clap command surface (root, version, doctor, ...)
  config/      # sunscreen.yml schema, loader, migrations
  toolchain/   # external tool detection (anchor, solana, ...)
  templates/   # embedded minijinja templates + render engine
  error.rs     # typed errors
benches/       # criterion benchmarks (cold-start budget)
tests/golden/  # insta snapshot tests for template output
docs/adr/      # architecture decision records
```

---

## Roadmap

Live tracker: [`ROADMAP.md`](ROADMAP.md) (single source of truth). Total to v1.0: ~21 weeks.

- **Phase 0** — Foundations: CLI shell, config, doctor, template engine. ✅
- **Phase 1** — Workspace bootstrap (`chain new` + Anchor + frontend variants). ✅
- **Phase 2** — Incremental scaffolding (program/instruction/account/event/error + `chain doctor --fix-markers`). 🚧 ~95%
- **Phase 3** — Runtime orchestration (`chain serve`, Surfpool + Codama + ratatui TUI). 📋
- **Phase 4** — Codegen & frontend hooks. 📋
- **Phase 5** — Recipes (CRUD, SPL token, Metaplex NFT). 📋
- **Phase 5.5** — Onboarding layer (`init`, `quickstart`, `wallet`, `deploy`, `learn`, actionable errors) — see [ADR-0005](docs/adr/ADR-0005-beginner-onboarding.md). 📋
- **Phase 8** — Distribution & docs (cuts v1.0). 📋
- **Phase 6** (plugins) and **Phase 7** (Pinocchio) — deferred post-v1.0. 🔮

---

## License

Apache-2.0
