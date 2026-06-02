# sunscreen

> A Rust CLI for scaffolding, repairing, and orchestrating Solana Anchor and Pinocchio workspaces.

`sunscreen` helps Solana developers move from an empty folder to a working Anchor or Pinocchio project without hand-stitching `anchor`, `solana`, `cargo`, `codama`, `surfpool`, and frontend tooling. It focuses on deterministic project generation, marker-based incremental edits, plugins, and a supervised local development loop.

**Current status:** Phase 7 Pinocchio support is complete; Phase 8 distribution and docs are next for v1.0. `sunscreen` is not published to crates.io yet; install from source for now. The live project tracker is [`ROADMAP.md`](ROADMAP.md).

---

## What it does

Today, `sunscreen` can:

- Diagnose your local Solana, Rust, and Cargo toolchain.
- Create a new multi-program Anchor workspace or minimal Pinocchio workspace with optional frontend variants.
- Add programs, instructions, accounts, events, and errors to an existing workspace.
- Scaffold complete CRUD, SPL token, and Metaplex NFT recipe slices.
- Repair generated marker regions when safe with `chain doctor --fix-markers`.
- Run a supervised local build/serve loop with Surfpool or `solana-test-validator`, file watching, Anchor builds, optional Codama regeneration, frontend notification, and headless JSON events.
- Generate deterministic IDL artifacts, Codama JavaScript clients, and React/Solid Query frontend hooks from Anchor IDLs.
- Start from beginner-friendly flows with `init`, `quickstart`, embedded examples, wallet helpers, deploy plans, learn topics, and actionable `next_step` errors.
- Manage local plugins, run plugin commands/hooks, list the reference marketplace, and route plugin-backed `scaffold <noun>` commands.
- Bootstrap Pinocchio programs with `chain new --framework pinocchio` and build them through `cargo build-sbf`.

Remaining v1.0 work is distribution and published docs. Remote plugin artifact download and richer Pinocchio-native scaffold/codegen flows remain follow-up work.

The full design rationale lives in [`docs/adr/ADR-0001-solis-cli.md`](docs/adr/ADR-0001-solis-cli.md). CLI conventions, marker protocol, recipes, codegen, plugins, Pinocchio, and the beginner-onboarding surface live in [`docs/adr/`](docs/adr/) and [`docs/reference/`](docs/reference/).

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

Create a Pinocchio workspace:

```bash
sunscreen chain new fast-program --framework pinocchio --frontend none
cd fast-program
sunscreen chain build --headless
```

Pinocchio workspaces declare the Rust/Cargo/Solana requirements in `sunscreen.yml`, use Solana SBF-aware entrypoint cfgs, and keep generated entrypoint regions marker-wrapped for future repair/scaffold flows. Anchor-only scaffolders and `generate` commands stop before writing when run inside a Pinocchio workspace.

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
| `chain new` | ✅ | Bootstrap a compilable Anchor or Pinocchio workspace (+ frontend variants) |
| `chain doctor --fix-markers` | ✅ | Repair drifted scaffolder markers in appendable hosts |
| `chain build --headless` | ✅ | Run the headless build pipeline (`anchor build` + optional Codama, or Pinocchio `cargo build-sbf`) |
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
| `app`<br>`{install, uninstall, list, describe, update}` | ✅ | Declarative plugin lifecycle in `sunscreen.yml` |
| `app`<br>`{commands, run, hook, marketplace}` | ✅ | Plugin runtime commands, lifecycle hooks, and reference marketplace |
| `scaffold <plugin-noun>` | ✅ | Route plugin-declared scaffold commands without changing core |

---

## Configuration

`sunscreen` reads `sunscreen.yml` from the working directory. Environment variables prefixed with `SUNSCREEN_` override config keys. Migrations between schema versions are versioned and round-trip deterministic.

See [`src/config/`](src/config/) for the schema implementation and [`docs/adr/ADR-0002-cli-design-conventions.md`](docs/adr/ADR-0002-cli-design-conventions.md) for CLI conventions.

---

## Generated markers

Incremental scaffolding is marker-based. Generated regions are wrapped in stable comments so `sunscreen` can make future edits without owning the whole file. You can edit normal Rust code around those regions; if a generated region drifts, `sunscreen chain doctor --fix-markers` repairs only the cases it can prove are safe.

The marker contract is documented in [`docs/reference/markers.md`](docs/reference/markers.md). Codegen ownership, recipe behaviour, plugin runtime, Pinocchio workspaces, and onboarding commands are documented in [`docs/reference/codegen.md`](docs/reference/codegen.md), [`docs/reference/recipes.md`](docs/reference/recipes.md), [`docs/reference/app.md`](docs/reference/app.md), [`docs/reference/pinocchio.md`](docs/reference/pinocchio.md), and [`docs/reference/onboarding.md`](docs/reference/onboarding.md).

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
  plugin/      # plugin manifests, marketplace, sandbox, stdio/gRPC adapters
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
proto/         # plugin gRPC contract
docs/reference/ # operational command/reference docs
```

---

## Roadmap

Live tracker: [`ROADMAP.md`](ROADMAP.md) is the single source of truth. Total planned time to v1.0 is ~28 focused weeks.

- **Phase 0** — Foundations: CLI shell, config, doctor, template engine. ✅
- **Phase 1** — Workspace bootstrap (`chain new` + Anchor + frontend variants). ✅
- **Phase 2** — Incremental scaffolding (program/instruction/account/event/error + `chain doctor --fix-markers`). ✅
- **Phase 3** — Runtime orchestration (`chain build`, `chain serve`, Surfpool/test-validator, watcher, Codama, ratatui TUI). ✅
- **Phase 4** — Codegen & frontend hooks. ✅
- **Phase 5** — Recipes (CRUD, SPL token, Metaplex NFT). ✅
- **Phase 5.5** — Onboarding layer (`init`, `quickstart`, `examples`, `wallet`, `deploy`, `learn`, actionable errors) — see [ADR-0005](docs/adr/ADR-0005-beginner-onboarding.md). ✅
- **Phase 6** — Plugin system: lifecycle, manifest/runtime, stdio/gRPC transport contract, sandbox, marketplace/reference plugins. ✅
- **Phase 7** — Pinocchio support (`chain new --framework pinocchio`, Cargo/Solana toolchain config, `cargo build-sbf` pipeline, Anchor-only guards). ✅
- **Phase 8** — Distribution & docs (cuts v1.0 after plugin and Pinocchio closure). ⏳ next

Phase 6 and Phase 7 are now closed in the v1.0 line. Phase 8 cuts v1.0.

---

## License

Apache-2.0
