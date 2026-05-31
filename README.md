# sunscreen

> Solana CLI scaffolding & orchestration tool — written in Rust, inspired by Ignite CLI.

`sunscreen` is a greenfield CLI that streamlines building on Solana: incremental scaffolding of Anchor 1.0 programs, dev-loop orchestration (Surfpool + Codama + frontend), and a plugin system for extending the toolchain.

**Status:** Phase 0 — foundations. Not yet published to crates.io.

---

## Why

Building on Solana today means stitching together `anchor`, `solana`, `surfpool`, `codama`, `cargo`, and a frontend toolchain by hand. `sunscreen` provides a single, opinionated entrypoint that:

- **Scaffolds** Anchor programs from typed, deterministic templates.
- **Orchestrates** the dev loop (local validator, IDL → client generation, frontend hot-reload).
- **Diagnoses** your environment so version drift surfaces before it bites.
- **Extends** via a plugin protocol so teams can ship their own commands.

The full design rationale lives in [`docs/adr/ADR-0001-solis-cli.md`](ADR-0001-solis-cli.md). CLI conventions and documentation strategy live in [`docs/adr/`](docs/adr/).

---

## Install

From source (requires Rust 1.75+):

```bash
git clone https://github.com/sunscreen-cli/sunscreen
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

# Scaffold a new project (stub — coming in Phase 1)
sunscreen scaffold <name>
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
| `scaffold` | 🚧 stub | Scaffold a new Solana project |
| `chain` | 🚧 stub | Manage local validator / chain ops |
| `generate` | 🚧 stub | Code generation utilities |
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

See [`IMPLEMENTATION-KICKOFF.md`](IMPLEMENTATION-KICKOFF.md) for the phased plan.

- **Phase 0** — Foundations: CLI shell, config, doctor, template engine. *(in progress)*
- **Phase 1** — Anchor scaffolding from typed templates.
- **Phase 2** — Dev-loop orchestration (Surfpool, Codama, frontend).
- **Phase 3** — Plugin protocol.

---

## License

Apache-2.0
