# sunscreen

> A Rust CLI for scaffolding, repairing, and orchestrating Solana Anchor and Pinocchio workspaces.

`sunscreen` helps Solana developers move from an empty folder to a working Anchor or Pinocchio project without hand-stitching `anchor`, `solana`, `cargo`, `codama`, `surfpool`, and frontend tooling. It focuses on deterministic project generation, marker-based incremental edits, plugins, and a supervised local development loop.

**Current status:** all v1.0 phases (0–8) are closed. `v0.1.0` is the first preview release, with GitHub Release binaries for Linux and macOS and publishing across Homebrew, Snap, and APT. `sunscreen` is not published to crates.io yet. The live project tracker is [`ROADMAP.md`](ROADMAP.md).

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

Follow-up work beyond v1.0 includes remote plugin artifact download, richer Pinocchio-native scaffold/codegen flows, Windows distribution, and shell completions.

The full design rationale lives in [`docs/adr/ADR-0001-solis-cli.md`](docs/adr/ADR-0001-solis-cli.md). For the available user and contributor docs, start with the documentation map below.

---

## Documentation

The docs site source lives in [`docs/site/`](docs/site/) and is published from `main` to [Pantani.github.io/sunscreen](https://Pantani.github.io/sunscreen/) by [`.github/workflows/docs.yml`](.github/workflows/docs.yml). Use this README as the project overview; use the docs site for guided learning, task walkthroughs, and searchable reference.

| Track | Start here | Use it for |
|-------|------------|------------|
| Learn | [`What is sunscreen?`](docs/site/src/learn/what-is-sunscreen.md), [`Installing`](docs/site/src/learn/installing.md), [`Your first workspace`](docs/site/src/learn/first-workspace.md), [`Your first NFT`](docs/site/src/learn/your-first-nft.md) | First-time setup and beginner-friendly Solana/Rust context |
| Guides | [`Scaffolding a CRUD resource`](docs/site/src/guides/scaffolding-crud.md), [`The dev loop with chain serve`](docs/site/src/guides/dev-loop.md), [`Working with plugins`](docs/site/src/guides/plugins.md), [`Troubleshooting`](docs/site/src/guides/troubleshooting.md) | Task-oriented walkthroughs with concrete outcomes |
| Reference | [`CLI overview`](docs/site/src/reference/cli/index.md), [`sunscreen.yml` schema](docs/site/src/reference/config/schema.md), [`Recipes`](docs/site/src/reference/recipes/index.md), [`Plugin protocol`](docs/site/src/reference/plugin-protocol/index.md), [`Errors & exit codes`](docs/site/src/reference/errors.md) | Command syntax, flags, events, config, recipes, and machine-readable contracts |
| Concepts | [`Architecture`](docs/site/src/concepts/architecture.md), [`Workspace model`](docs/site/src/concepts/workspace-model.md), [`Build pipeline`](docs/site/src/concepts/build-pipeline.md), [`Plugin runtime`](docs/site/src/concepts/plugin-runtime.md), [`Anchor vs Pinocchio`](docs/site/src/concepts/framework-pinocchio-vs-anchor.md) | The mental model behind the CLI |
| Contributing | [`Roadmap`](docs/site/src/contributing/roadmap.md), [`Architecture decisions`](docs/site/src/contributing/adrs.md), [`Developer setup`](docs/site/src/contributing/dev-setup.md), [`Documentation style`](docs/site/src/contributing/docs-style.md) | Project status, ADRs, local development, and docs conventions |

Operational repo references are also kept in [`docs/reference/`](docs/reference/): [`markers`](docs/reference/markers.md), [`codegen`](docs/reference/codegen.md), [`recipes`](docs/reference/recipes.md), [`onboarding`](docs/reference/onboarding.md), [`app/plugins`](docs/reference/app.md), [`Pinocchio`](docs/reference/pinocchio.md), [`testing`](docs/reference/testing.md), and [`distribution`](docs/reference/distribution.md).

---

## Install

Pick the channel that matches your platform:

```bash
# Homebrew (macOS + Linux)
brew install Pantani/sunscreen/sunscreen

# Snap (Linux, classic confinement)
sudo snap install sunscreen-cli --classic

# APT (Debian / Ubuntu via Cloudsmith)
curl -1sLf 'https://dl.cloudsmith.io/public/pantani/sunscreen/setup.deb.sh' | sudo -E bash
sudo apt-get install sunscreen

# Universal shell installer (any POSIX)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Pantani/sunscreen/releases/latest/download/sunscreen-installer.sh \
  | sh
```

Or build from source:

```bash
git clone https://github.com/Pantani/sunscreen
cd sunscreen
cargo install --path .
```

See [`docs/reference/distribution.md`](docs/reference/distribution.md) for
channel internals, secrets, and re-publish procedure.

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

| Command | Description |
|---------|-------------|
| `version` | Print sunscreen version (text or JSON) |
| `doctor` | Diagnose toolchain & environment |
| `chain new` | Bootstrap a compilable Anchor or Pinocchio workspace (+ frontend variants) |
| `chain doctor --fix-markers` | Repair drifted scaffolder markers in appendable hosts |
| `chain build --headless` | Run the headless build pipeline (`anchor build` + optional Codama, or Pinocchio `cargo build-sbf`) |
| `chain serve --headless` | Run the supervised runtime/watch loop with line-delimited JSON events |
| `scaffold program` | Add a new program crate to an existing workspace |
| `scaffold instruction` | Add an instruction (idempotent, marker-based, `--dry-run`, `--json`) |
| `scaffold account` | Add an account struct |
| `scaffold event` | Add an event |
| `scaffold error` | Add an error variant |
| `scaffold crud` | Generate a CRUD dApp slice |
| `scaffold spl-token` | Generate an SPL token recipe slice |
| `scaffold metaplex-nft` | Generate a Metaplex NFT recipe slice |
| `chain serve` | Full supervised dev loop (Surfpool/test-validator + watcher + TUI) |
| `generate idl` | Export built Anchor IDLs into `clients/idl` |
| `generate clients` | Write `codama.json` and run Codama client generation |
| `generate frontend-hooks` | Generate IDL/core TypeScript plus React/Solid Query hooks |
| `init` | Beginner-friendly workspace wizard/non-interactive bootstrap |
| `quickstart {token,nft,dao,blog}` | Create a complete starter dApp from a recipe |
| `examples {list,describe,use}` | Browse or copy embedded examples |
| `wallet {new,list,airdrop,balance,set-default}` | Manage local Solana wallets and devnet balances |
| `deploy <cluster>` | Plan or run Anchor deploys with safety gates |
| `learn [topic]` | Render embedded learning topics offline |
| `app`<br>`{install, uninstall, list, describe, update}` | Declarative plugin lifecycle in `sunscreen.yml` |
| `app`<br>`{commands, run, hook, marketplace}` | Plugin runtime commands, lifecycle hooks, and reference marketplace |
| `scaffold <plugin-noun>` | Route plugin-declared scaffold commands without changing core |

---

## Configuration

`sunscreen` reads `sunscreen.yml` from the working directory. Environment variables prefixed with `SUNSCREEN_` override config keys. Migrations between schema versions are versioned and round-trip deterministic.

See [`src/config/`](src/config/) for the schema implementation and [`docs/adr/ADR-0002-cli-design-conventions.md`](docs/adr/ADR-0002-cli-design-conventions.md) for CLI conventions.

---

## Generated markers

Incremental scaffolding is marker-based. Generated regions are wrapped in stable comments so `sunscreen` can make future edits without owning the whole file. You can edit normal Rust code around those regions; if a generated region drifts, `sunscreen chain doctor --fix-markers` repairs only the cases it can prove are safe.

The marker contract is documented in [`docs/reference/markers.md`](docs/reference/markers.md). Codegen ownership, recipe behaviour, plugin runtime, Pinocchio workspaces, testing tiers, and onboarding commands are documented in [`docs/reference/codegen.md`](docs/reference/codegen.md), [`docs/reference/recipes.md`](docs/reference/recipes.md), [`docs/reference/app.md`](docs/reference/app.md), [`docs/reference/pinocchio.md`](docs/reference/pinocchio.md), [`docs/reference/testing.md`](docs/reference/testing.md), and [`docs/reference/onboarding.md`](docs/reference/onboarding.md).

---

## Development

```bash
cargo build              # build
cargo test               # run unit + golden tests
cargo clippy -- -D warnings
cargo fmt --check
cargo bench --bench cold_start
mdbook build docs/site
mdbook serve docs/site --open
bash scripts/integration-heavy.sh
SUNSCREEN_REAL_TOOLCHAIN=1 bash scripts/integration-heavy.sh
```

See [`docs/site/README.md`](docs/site/README.md) for the one-time mdBook toolchain setup.

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
docs/site/     # mdBook documentation site
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
- **Phase 8** — Distribution & docs. ✅

All v1.0 phases are now closed.

---

## License

Apache-2.0
