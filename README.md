# sunscreen

> A Rust CLI for scaffolding, repairing, and orchestrating Solana Anchor workspaces.

`sunscreen` helps Solana developers move from an empty folder to a working Anchor project without hand-stitching `anchor`, `solana`, `cargo`, `codama`, `surfpool`, and frontend tooling. It focuses on deterministic project generation, marker-based incremental edits, and a supervised local development loop.

**Current status:** Phase 5.5 onboarding is complete; Phase 8 distribution and docs are next for v1.0. `sunscreen` is not published to crates.io yet; install from source for now. The live project tracker is [`ROADMAP.md`](ROADMAP.md).

---

## What it does

Today, `sunscreen` can:

- Diagnose your local Solana and Rust toolchain.
- Create a new multi-program Anchor workspace with optional frontend variants.
- Add programs, instructions, accounts, events, and errors to an existing workspace.
- Scaffold complete CRUD, SPL token, and Metaplex NFT recipe slices.
- Repair generated marker regions when safe with `chain doctor --fix-markers`.
- Run a supervised local build/serve loop with Surfpool or `solana-test-validator`, file watching, Anchor builds, optional Codama regeneration, frontend notification, and headless JSON events.
- Generate deterministic IDL artifacts, Codama JavaScript clients, and React/Solid Query frontend hooks from Anchor IDLs.
- Start from beginner-friendly flows with `init`, `quickstart`, embedded examples, wallet helpers, deploy plans, learn topics, and actionable `next_step` errors.

Remaining v1.0 work is distribution and published docs. Plugins and Pinocchio support are intentionally deferred until after v1.0.

The full design rationale lives in [`docs/adr/ADR-0001-solis-cli.md`](docs/adr/ADR-0001-solis-cli.md). CLI conventions, marker protocol, recipes, codegen, and the beginner-onboarding surface live in [`docs/adr/`](docs/adr/) and [`docs/reference/`](docs/reference/).

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

Create a complete starter dApp:

```bash
sunscreen quickstart nft --name mint-demo --cluster devnet --non-interactive
cd mint-demo
```

Or create a plain workspace first:

```bash
sunscreen init my-dapp --non-interactive
cd my-dapp
```

Add generated code incrementally:

```bash
sunscreen scaffold program my_program
sunscreen scaffold instruction transfer --program my_program
sunscreen scaffold account Vault --program my_program
sunscreen scaffold event Transferred --program my_program
sunscreen scaffold error InsufficientFunds --program my_program
```

Add a composite recipe:

```bash
sunscreen scaffold crud Post --program my_program
sunscreen scaffold spl-token Faucet --program my_program
sunscreen scaffold metaplex-nft Collection --program my_program
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

Generate IDL/client/frontend artifacts after a build:

```bash
sunscreen generate idl
sunscreen generate clients
sunscreen generate frontend-hooks
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
| `scaffold crud` | ✅ | Generate a CRUD dApp slice |
| `scaffold spl-token` | ✅ | Generate an SPL token recipe slice |
| `scaffold metaplex-nft` | ✅ | Generate a Metaplex NFT recipe slice |
| `chain serve` | ✅ | Full supervised dev loop (Surfpool/test-validator + watcher + TUI) |
| `generate idl` | ✅ | Export built Anchor IDLs into `clients/idl` |
| `generate clients` | ✅ | Write `codama.json` and run Codama client generation |
| `generate frontend-hooks` | ✅ | Generate IDL/core TypeScript plus React/Solid Query hooks |
| `init` | ✅ | Beginner-friendly workspace wizard/non-interactive bootstrap |
| `quickstart {token,nft,dao,blog}` | ✅ | Create a complete starter dApp from a recipe |
| `examples {list,describe,use}` | ✅ | Browse or copy embedded examples |
| `wallet {new,list,airdrop,balance,set-default}` | ✅ | Manage local Solana wallets and devnet balances |
| `deploy <cluster>` | ✅ | Plan or run Anchor deploys with safety gates |
| `learn [topic]` | ✅ | Render embedded learning topics offline |
| `app {install,uninstall,list,describe,update,commands,run,hook,marketplace}` | ✅ | Plugin lifecycle plus Phase 6 runtime command surface in `sunscreen.yml` |

---

## Configuration

`sunscreen` reads `sunscreen.yml` from the working directory. Environment variables prefixed with `SUNSCREEN_` override config keys. Migrations between schema versions are versioned and round-trip deterministic.

See [`src/config/`](src/config/) for the schema implementation and [`docs/adr/ADR-0002-cli-design-conventions.md`](docs/adr/ADR-0002-cli-design-conventions.md) for CLI conventions.

---

## Generated markers

Incremental scaffolding is marker-based. Generated regions are wrapped in stable comments so `sunscreen` can make future edits without owning the whole file. You can edit normal Rust code around those regions; if a generated region drifts, `sunscreen chain doctor --fix-markers` repairs only the cases it can prove are safe.

The marker contract is documented in [`docs/reference/markers.md`](docs/reference/markers.md). Codegen ownership, recipe behaviour, and onboarding commands are documented in [`docs/reference/codegen.md`](docs/reference/codegen.md), [`docs/reference/recipes.md`](docs/reference/recipes.md), and [`docs/reference/onboarding.md`](docs/reference/onboarding.md).

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
  codegen/     # Codama config, IDL export, frontend hook generation
  config/      # sunscreen.yml schema, loader, migrations
  onboarding/  # init, quickstart, examples, wallet, deploy, learn flows
  runtime/     # subprocess, build pipeline, watcher, validator supervisor
  scaffold/    # composite CRUD/SPL token/Metaplex NFT recipes
  strings/     # centralized user-facing strings
  toolchain/   # external tool detection (anchor, solana, ...)
  templates/   # embedded minijinja templates + render engine
  error.rs     # typed errors
assets/        # embedded examples and learn topics
benches/       # criterion benchmarks (cold-start budget)
tests/golden/  # insta snapshot tests for template output
docs/adr/      # architecture decision records
docs/reference/ # operational command/reference docs
```

---

## Roadmap

Live tracker: [`ROADMAP.md`](ROADMAP.md) is the single source of truth. Total planned time to v1.0 is ~25 focused weeks.

- **Phase 0** — Foundations: CLI shell, config, doctor, template engine. ✅
- **Phase 1** — Workspace bootstrap (`chain new` + Anchor + frontend variants). ✅
- **Phase 2** — Incremental scaffolding (program/instruction/account/event/error + `chain doctor --fix-markers`). ✅
- **Phase 3** — Runtime orchestration (`chain build`, `chain serve`, Surfpool/test-validator, watcher, Codama, ratatui TUI). ✅
- **Phase 4** — Codegen & frontend hooks. ✅
- **Phase 5** — Recipes (CRUD, SPL token, Metaplex NFT). ✅
- **Phase 5.5** — Onboarding layer (`init`, `quickstart`, `examples`, `wallet`, `deploy`, `learn`, actionable errors) — see [ADR-0005](docs/adr/ADR-0005-beginner-onboarding.md). ✅
- **Phase 6** — Plugin system: lifecycle, manifest/runtime, stdio/gRPC transport contract, sandbox, marketplace/reference plugins. ✅
- **Phase 8** — Distribution & docs (cuts v1.0 after plugin closure). ⏳ next
- **Phase 7** — Pinocchio support. 🔮 post-v1.0

Phase 6 is now closed in the v1.0 line. Phase 8 cuts v1.0; Phase 7 remains intentionally deferred.

---

## License

Apache-2.0
