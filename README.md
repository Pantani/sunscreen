# sunscreen

> A Rust CLI for scaffolding, repairing, and orchestrating Solana Anchor workspaces.

`sunscreen` helps Solana developers move from an empty folder to a working Anchor project without hand-stitching `anchor`, `solana`, `cargo`, `codama`, `surfpool`, and frontend tooling. It focuses on deterministic project generation, marker-based incremental edits, and a supervised local development loop.

**Current status:** Phase 3 runtime orchestration is complete; Phase 4 codegen and frontend hooks are next. `sunscreen` is not published to crates.io yet; install from source for now. The live project tracker is [`ROADMAP.md`](ROADMAP.md).

---

## What it does

Today, `sunscreen` can:

- Diagnose your local Solana and Rust toolchain.
- Create a new multi-program Anchor workspace with optional frontend variants.
- Add programs, instructions, accounts, events, and errors to an existing workspace.
- Repair generated marker regions when safe with `chain doctor --fix-markers`.
- Run a supervised local build/serve loop with Surfpool or `solana-test-validator`, file watching, Anchor builds, optional Codama regeneration, frontend notification, and headless JSON events.

Planned work adds first-class codegen commands, frontend hooks, recipes, onboarding flows, distribution, and eventually plugins.

The full design rationale lives in [`docs/adr/ADR-0001-solis-cli.md`](docs/adr/ADR-0001-solis-cli.md). CLI conventions, marker protocol, and the beginner-onboarding surface live in [`docs/adr/`](docs/adr/).

---

## Install

Install from source:

```bash
git clone https://github.com/Pantani/sunscreen
cd sunscreen
cargo install --path .
```

Verify the binary:

```bash
sunscreen --version
sunscreen doctor
```

---

## Quick start

Create a new Anchor workspace:

```bash
sunscreen chain new my-dapp
cd my-dapp
```

Add generated code incrementally:

```bash
sunscreen scaffold program my_program
sunscreen scaffold instruction transfer
sunscreen scaffold account Vault
sunscreen scaffold event Transferred
sunscreen scaffold error InsufficientFunds
```

Inspect or repair the workspace:

```bash
sunscreen chain doctor
sunscreen chain doctor --fix-markers
```

Run the local build or serve loop:

```bash
sunscreen chain build --headless
sunscreen chain serve --headless
```

### Global flags

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Increase logging verbosity (`-v`, `-vv`, `-vvv`) |
| `--workdir <DIR>` | Override working directory |
| `--config <FILE>` | Path to a `sunscreen.yml` config file |
| `--json` | Emit structured JSON output where supported |

Most scaffold commands also support `--dry-run` and `--json`, so you can preview generated edits before writing them.

---

## Commands

| Command | Status | Description |
|---------|--------|-------------|
| `version` | ✅ | Print sunscreen version (text or JSON) |
| `doctor` | ✅ | Diagnose toolchain & environment |
| `chain new` | ✅ | Bootstrap a compilable Anchor workspace (+ frontend variants) |
| `chain doctor --fix-markers` | ✅ | Repair drifted scaffolder markers in appendable hosts |
| `chain build --headless` | ✅ | Run the headless build pipeline (`anchor build` + optional Codama) |
| `chain serve --headless` | ✅ | Run the supervised runtime/watch loop with line-delimited JSON events |
| `scaffold program` | ✅ | Add a new program crate to an existing workspace |
| `scaffold instruction` | ✅ | Add an instruction (idempotent, marker-based, `--dry-run`, `--json`) |
| `scaffold account` | ✅ | Add an account struct |
| `scaffold event` | ✅ | Add an event |
| `scaffold error` | ✅ | Add an error variant |
| `chain serve` | ✅ | Full supervised dev loop (Surfpool/test-validator + watcher + TUI) |
| `generate` | 🚧 stub | Code generation (clients, IDL, frontend hooks) — Phase 4 |
| `app` | 🚧 stub | Application lifecycle commands |

---

## Configuration

`sunscreen` reads `sunscreen.yml` from the working directory. Environment variables prefixed with `SUNSCREEN_` override config keys. Migrations between schema versions are versioned and round-trip deterministic.

See [`src/config/`](src/config/) for the schema implementation and [`docs/adr/ADR-0002-cli-design-conventions.md`](docs/adr/ADR-0002-cli-design-conventions.md) for CLI conventions.

---

## Generated markers

Incremental scaffolding is marker-based. Generated regions are wrapped in stable comments so `sunscreen` can make future edits without owning the whole file. You can edit normal Rust code around those regions; if a generated region drifts, `sunscreen chain doctor --fix-markers` repairs only the cases it can prove are safe.

The marker contract is documented in [`docs/reference/markers.md`](docs/reference/markers.md).

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

Live tracker: [`ROADMAP.md`](ROADMAP.md) is the single source of truth. Total planned time to v1.0 is ~21 focused weeks.

- **Phase 0** — Foundations: CLI shell, config, doctor, template engine. ✅
- **Phase 1** — Workspace bootstrap (`chain new` + Anchor + frontend variants). ✅
- **Phase 2** — Incremental scaffolding (program/instruction/account/event/error + `chain doctor --fix-markers`). ✅
- **Phase 3** — Runtime orchestration (`chain build`, `chain serve`, Surfpool/test-validator, watcher, Codama, ratatui TUI). ✅
- **Phase 4** — Codegen & frontend hooks. ⏳ next
- **Phase 5** — Recipes (CRUD, SPL token, Metaplex NFT). 📋
- **Phase 5.5** — Onboarding layer (`init`, `quickstart`, `examples`, `wallet`, `deploy`, `learn`, actionable errors) — see [ADR-0005](docs/adr/ADR-0005-beginner-onboarding.md). 📋
- **Phase 8** — Distribution & docs (cuts v1.0). 📋
- **Phase 6** — Plugin system. 🔮 post-v1.0
- **Phase 7** — Pinocchio support. 🔮 post-v1.0

Phase 8 cuts v1.0. Phases 6 and 7 are intentionally deferred so plugins and Pinocchio support do not block the first stable release.

---

## License

Apache-2.0
