# ADR-0001 — `solis`: CLI Scaffolding & Orchestration Tool for the Solana Ecosystem

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-05-31 |
| **Authors** | Danilo Lacombe |
| **Tags** | architecture, tooling, solana, anchor, codama, cli |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | (future ADRs 0002+) |

---

## TL;DR

`solis` is a Go-based CLI for the Solana ecosystem in the spirit of [Ignite CLI](https://github.com/ignite/cli) for Cosmos. It scaffolds Anchor 1.0 workspaces and programs, generates instructions / accounts / events / errors incrementally into existing projects, orchestrates the dev loop (Surfpool + Codama + frontend HMR) via a unified `chain serve`, and exposes a plugin system for ecosystem-driven recipes. The MVP targets Anchor 1.0 with the `multiple` program template, uses **Anchor IDL as source of truth** for client generation via Codama, and uses **marker-based segment editing** for incremental Rust file mutations.

The decision proposed below is to greenfield `solis` in Go rather than fork existing tools, because:
1. The gap is **incremental scaffolding + orchestration + plugins**, not "one-shot project init" — and that gap is not addressed by any current tool.
2. Existing tools (`create-solana-program`, `create-solana-dapp`, `mucho`, `anchor init`) each cover a slice but none compose into a full developer journey.
3. Reusable components already exist for the parts we should not re-invent: **Codama** (codegen), **Surfpool** (runtime), **Anchor 1.0 multiple template** (program layout), **LiteSVM** (in-process testing).

---

## 1. Context

### 1.1 Motivation

The Cosmos SDK ecosystem matured rapidly partly because Ignite CLI compressed the "scaffold chain → scaffold module → scaffold message → scaffold list → serve → deploy" cycle into single commands. Each command generates code that compiles, is idiomatic, and integrates the surrounding stack (proto, gRPC, TypeScript clients, Vue frontend, OpenAPI).

Solana has no equivalent. A developer in 2026 still strings together: `anchor init` (or `create-solana-program`) → manual editing of `lib.rs` for every new instruction → manual `target/idl/*.json` regeneration → manual `codama` config and run → manual `solana-test-validator` invocation with fixture flags → manual frontend wiring. There is friction at every transition.

This ADR proposes filling that gap.

### 1.2 Ignite CLI as Benchmark

Ignite CLI delivers four orthogonal capabilities that are the basis for the comparison:

| Capability | Ignite command examples |
|---|---|
| **One-shot project scaffolding** | `ignite scaffold chain foo` |
| **Incremental code generation** | `ignite scaffold module bar`, `ignite scaffold message send-coin amount:int`, `ignite scaffold list post title:string body:string`, `ignite scaffold map score winner:string` |
| **Dev-loop orchestration** | `ignite chain serve` (validator + faucet + frontend + watch + auto-reload + proto regeneration) |
| **Plugin ecosystem** | `ignite app install`, `ignite/apps` registry, gRPC plugin protocol |

Capabilities 2 and 4 are entirely absent from the Solana ecosystem. Capabilities 1 and 3 exist as fragmented tools (see § 1.3).

### 1.3 Market Research — State of Solana Tooling (Q1–Q2 2026)

#### 1.3.1 Tools Inventory

| Tool | Maintainer | Role | Maturity | Gap vs Ignite |
|---|---|---|---|---|
| **Anchor 1.0** | Solana Foundation | Framework + CLI: `anchor init`, `anchor build`, `anchor deploy`, `anchor idl`, `anchor account`, `anchor keys` | Stable (1.0 released Oct 2025) | One-shot init; no incremental `scaffold instruction`; no orchestration beyond `anchor test` |
| **`create-solana-program`** | Solana Foundation (`solana-program/`) | npm initializer: workspace with Anchor or Shank + multi-language clients via Codama | Stable | One-shot; no incremental gen; no dev loop |
| **`create-solana-dapp`** | Solana Foundation | npm initializer: dApp with Next.js + Anchor + wallet-adapter | Stable | One-shot; no incremental gen |
| **Mucho CLI** | Solana Foundation | Meta-tool: install toolchain, `mucho validator`, `mucho build`, `mucho deploy`, `mucho clone`, `Solana.toml` | Released 2024; ~42 weekly downloads (low adoption) | No scaffolding; just wraps existing tools |
| **Codama** (ex-Kinobi) | Codama IDL / Metaplex / Anza | IDL → clients (JS for Solana Kit, Umi, Rust, Go, Python, Dart, Yellowstone Vixen). SPL programs use it. | Stable, active (v1.x; commits Feb 2026) | Not a scaffolder; complementary |
| **Surfpool** | Hiro / community | "Anvil for Solana": drop-in `solana-test-validator` replacement, JIT mainnet fork via LiteSVM, IaC, MCP server | Active 2025 | Runtime only; no scaffolding |
| **LiteSVM** | community (under solana-foundation umbrella) | In-process Solana VM for tests (Rust/TS/Python). Replaces deprecated `solana-program-test`/`bankrun`. | Stable | Library, not a CLI |
| **Pinocchio** | Anza (Febo) + Blueshift | Zero-dependency `no_std` program library; 13–23× binary reduction; production usage by Exo Tech | Stable; growing adoption among perf-critical teams | Library, not a scaffolder |
| **Anchor IDL `--template multiple`** | Anchor team | Modular layout: `programs/<name>/src/{lib.rs, instructions/, state/, errors.rs, constants.rs}` | Stable since 0.29; default in 1.0 | Scaffolds only at `init` time |
| **Solita** | Metaplex | IDL → TS clients (legacy, Codama is preferred) | Deprecated path | n/a |
| **`anchor-ts-generator`** | Aleph.im | Anchor IDL → GraphQL indexer (moleculer) | Active | Niche; complementary |
| **Solores** | Igneous Labs | IDL → Rust CPI client | Active | Alternative to Codama-Rust |
| **Coda** (`@macalinao/coda`) | Ian Macalinao | Wrapper around Codama for typesafe TS clients | Active | Niche |
| **Trident** | Ackee Blockchain | Rust fuzzer for Solana programs | Active | Complementary (security) |
| **Zest** | community | Code coverage CLI for Solana | Active | Complementary |
| **Seahorse / Solang** | community | Python / Solidity DSLs compiled to Solana | Niche; not for production | Out of scope |

#### 1.3.2 Recurring Developer Pain Points (synthesized from public sources)

> Sources: superteam.fun "Deep Dive of the State of Developer Tooling on Solana" (Aug 2025), Helius pinocchio guide (Jun 2025), Medium articles "State of Dev Tooling on Solana 2025" by multiple authors, Quicknode guides (Nov 2025), Anchor 0.30/0.31/0.32/1.0 release notes.

| # | Pain Point | Affected Workflow |
|---|---|---|
| **P1** | "Anchor's dependency tree is brittle: `blake3 → edition2024` breaks builds on older Cargo" — repeated reports of `cargo update` cascade pain | Build/CI |
| **P2** | Each new instruction means manually editing `lib.rs` (the `#[program]` dispatch), adding the `Accounts` struct, adding state, manually re-running `anchor build`, then editing `codama.json` if needed | Daily dev loop |
| **P3** | "Surfpool is the best first touchpoint" but it is not integrated with project lifecycle (no `solis serve` style command) | Runtime / testing |
| **P4** | CPI testing against mainnet programs (Jupiter, Metaplex, Pyth) requires dumping 40+ accounts manually — Surfpool fixes runtime but devs still write manual scripts to compose fixtures | Integration testing |
| **P5** | "No mobile-specific CI or test flows; wallet adapters exist but no SDKs for mobile-native minting, compressed NFTs, smart account flows" | Mobile development |
| **P6** | Indexer setup remains DIY; `anchor-ts-generator` exists but is not part of the canonical flow | Backend / data |
| **P7** | "Static analysis tooling specifically tailored for Anchor or Pinocchio is needed to catch common bugs before audits" | Security |
| **P8** | Frontend wiring: from IDL → typed React Query hooks still requires hand-rolled boilerplate above Codama | Frontend |
| **P9** | No declarative config for full workspace orchestration. `Solana.toml` (mucho) is a step in the right direction but covers a subset; `Anchor.toml` only covers Anchor concerns; `codama.json` is separate | Project config |
| **P10** | Migrations across Anchor versions are manual (`avm use`, edit `Cargo.toml`, edit `Anchor.toml`, regenerate IDLs) | Maintenance |

#### 1.3.3 What's Hot in 2026

- **Anchor 1.0** released October 2025 with the `multiple` template now the recommended layout. This is the structural prerequisite that makes incremental scaffolding tractable (single-file `lib.rs` would have been a non-starter).
- **Pinocchio** is growing among DeFi/AMM teams who optimize compute units. Production deployments (Exo Tech, Blueshift). Currently has no framework — only a library + Shank IDL workflow.
- **Codama** is now the de-facto codegen standard. SPL programs themselves use it. Has Rust port in WIP (`codama-rs`).
- **Surfpool** has become the preferred local runtime. Its IaC primitives + MCP server make it agent-friendly.
- **Solana Kit** (formerly web3.js v2) is the new official JS lib. Tree-shakable, typed, designed to consume Codama-generated clients.
- **Token-2022** (transfer hooks, confidential transfers) is the active extension surface.
- **Alpenglow / Firedancer** are network-level evolutions; tooling implications are minimal but verifiable builds are increasingly required.

#### 1.3.4 Gap Analysis (vs Ignite)

Mapping § 1.2 capabilities to the inventory:

| Capability | Closest Solana tool | Gap |
|---|---|---|
| One-shot scaffold | `anchor init -t multiple`, `create-solana-program` | ⚠️ Partial — fragmented across two tools, no convergence |
| **Incremental scaffold instruction** | — | ❌ **No tool exists** |
| **Incremental scaffold account / event / error** | — | ❌ **No tool exists** |
| **CRUD recipe** (instruction + account + client + frontend hook) | — | ❌ **No tool exists** |
| **Frontend hook generation** (IDL → React Query) | Codama renderers (raw client) | ⚠️ Raw client only; no opinionated React Query / Solid Query wrapper |
| Dev-loop orchestration | Surfpool + `anchor test` + manual codama + frontend dev server | ⚠️ Fragmented; no single command unifies all four |
| Plugin ecosystem | — | ❌ **No tool exists** |
| Indexer scaffolding | `anchor-ts-generator` (standalone) | ⚠️ Not integrated |
| Verifiable builds | `solana-verify` (standalone) | ⚠️ Not integrated |

This identifies five clear "no-tool-exists" gaps, which together justify a greenfield CLI rather than fork-and-extend.

### 1.4 Why Not Just Use AI to Generate the Code?

A reasonable alternative is "the AI agent writes the instruction directly." This is real but does not displace `solis`:

- **Deterministic, reproducible output is mandatory** for CI and audits. Templates ensure byte-identical output for identical inputs; AI does not.
- **Schema-level operations** (e.g. CRUD recipe = instruction + account + client method + hook) are mechanical, not creative — wasteful to spend tokens on.
- **`solis` and AI compose**: `solis scaffold instruction deposit ...` for the boilerplate, then the human/agent writes the business logic. This is the Rails/Django pattern.

---

## 2. Decision Drivers

The architecture is forced by:

- **DD1** — Anchor IDL **is** the schema; do not invent a parallel one.
- **DD2** — Generated code must `cargo build-sbf` cleanly on first run, every time. No "fix-up after scaffold" steps.
- **DD3** — User-edited code must survive scaffolding of new artifacts. Idempotency is non-negotiable.
- **DD4** — Single binary distribution (no `npm install -g` user-facing dependency tree for the CLI itself).
- **DD5** — Reuse existing tools at the boundary: do not re-implement Codama, Surfpool, Anchor.
- **DD6** — Plugin protocol must allow plugins in Go (perf) and in TS/Node (ecosystem proximity).
- **DD7** — Test strategy must validate not just template correctness but **semantic correctness** (the output compiles and executes).
- **DD8** — The tool must not require network access for offline scaffolding (templates embedded).

---

## 3. Considered Options

### Option A — Fork & extend Mucho CLI

**Pro:** Solana Foundation endorsement; existing `Solana.toml` schema; existing toolchain installer.
**Con:** Mucho is TypeScript / Node. Forking it inherits the npm distribution model (slow startup, version hell). Mucho's design centers on "wrap existing CLIs", not on AST/template generation. Adoption is low (~42 weekly downloads), so the brand equity gain is marginal. Pivoting it to a scaffolder would be a rewrite, not a fork.
**Verdict:** Rejected.

### Option B — Build on top of `create-solana-program`

**Pro:** Reuses well-tested project layout templates; Foundation-blessed.
**Con:** It is purely a `npm init` initializer. It has no execution model for incremental commands, no AST/template engine, no orchestration. Would still require building 90% of `solis` next to it. Better to consume its templates as one of several scaffolding sources.
**Verdict:** Rejected as foundation, accepted as **inspiration / template reference**.

### Option C — Greenfield Go CLI (`solis`)

**Pro:** Direct path to all decision drivers. Reuses Ignite's well-proven design choices (cobra, viper, embed.FS, go-plugin, bubble tea for TUI). Single binary distribution. Familiar stack for the project author. Plugins can be implemented in Go (fast) or TS via stdio JSON-RPC. Aligns with how `mucho` is positioned (meta-tool) but adds real value (scaffolding).
**Con:** Greenfield risk; cold-start adoption; must earn ecosystem trust. Forces dependency on external Node tooling for Codama (acceptable, see § 7.4).
**Verdict:** **Selected.**

### Option D — TypeScript/Node CLI

**Pro:** Native to the Solana JS ecosystem; can `import` Codama directly without subprocess.
**Con:** Distribution friction (Node version, pnpm/yarn/npm wars); slow CLI startup (~200–500 ms cold); plugin system harder to lock down. AST manipulation for Rust is the same problem in any language.
**Verdict:** Rejected. Codama is invoked as subprocess from Go with negligible overhead.

---

## 4. Decision

Adopt **Option C**: greenfield CLI named **`solis`** (Latin *sōlis* — "of the sun"; preserves the `Sol` root, neoclassical naming convention, distinct from existing tools).

The MVP scope, capability priority, and architecture are detailed below. Operational sub-decisions are formalized as nested ADRs in § 7.


---

## 5. High-Level Architecture

### 5.1 Logical Layers

```
┌──────────────────────────────────────────────────────────────────┐
│                          cmd/solis (cobra)                       │
│       chain | scaffold | generate | app | doctor | version       │
└────────────────────────────────┬─────────────────────────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        ▼                        ▼                        ▼
┌───────────────┐       ┌────────────────┐       ┌───────────────┐
│  Scaffolding  │       │  Orchestration │       │   Codegen     │
│  pkg/scaffold │       │  pkg/runtime   │       │  pkg/codama   │
│               │       │                │       │               │
│ workspace,    │       │ surfpool,      │       │ codama config │
│ program,      │       │ test-validator │       │ generator,    │
│ instruction,  │       │ fallback,      │       │ runner,       │
│ account,      │       │ watcher (fsno- │       │ output mux    │
│ event, error, │       │ tify), faucet, │       │               │
│ crud, ...     │       │ TUI (bubbletea)│       │               │
└───────┬───────┘       └────────┬───────┘       └───────┬───────┘
        │                        │                        │
        ▼                        ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│        Shared infrastructure (pkg/* — used across layers)        │
│                                                                  │
│  config (solis.yml)  |  templates (embed.FS + text/template)    │
│  rustpatch (markers) |  idl (anchor IDL r/w)                    │
│  toolchain (anchor,  |  plugin (go-plugin + stdio JSON-RPC)     │
│   solana, cargo)     |  fsutil, processutil, logging            │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Source-of-Truth Hierarchy

The architecture has **one schema of record** and **multiple derived artifacts**.

```
       ┌──────────────────────────┐
       │   Rust source (lib.rs    │  ← human-edited, scaffold-augmented
       │   + instructions/*.rs    │
       │   + state/*.rs)          │
       └──────────────┬───────────┘
                      │  anchor build
                      ▼
       ┌──────────────────────────┐
       │  Anchor IDL JSON         │  ← machine artifact, schema of record
       │  target/idl/<prog>.json  │
       └──────────────┬───────────┘
                      │  codama run
                      ▼
       ┌──────────────────────────────────────────────────┐
       │  Generated clients (TS, Rust, Go, Umi, etc.)     │
       │  + indexer parsers + verifiable build manifests  │
       └──────────────────────────────────────────────────┘
```

The Anchor IDL is **derived** from Rust code (via `anchor build`) but **consumed** as the canonical schema by every downstream artifact. `solis` writes Rust, reads IDL, drives Codama. It never writes the IDL directly.

### 5.3 Generated Workspace Layout

```
my-protocol/
├── solis.yml                         # workspace manifest
├── Anchor.toml                       # standard Anchor config
├── Cargo.toml                        # workspace
├── codama.json                       # managed by solis
├── package.json
├── programs/
│   └── escrow/
│       ├── Cargo.toml
│       ├── Xargo.toml
│       └── src/
│           ├── lib.rs                # entrypoint, declare_id!, #[program] dispatch
│           ├── instructions/
│           │   ├── mod.rs            # solis-managed (markers)
│           │   ├── initialize.rs     # one file per ix; user-editable body
│           │   └── deposit.rs
│           ├── state/
│           │   ├── mod.rs            # solis-managed (markers)
│           │   └── vault.rs
│           ├── events.rs             # solis-managed (markers)
│           ├── errors.rs             # solis-managed (markers)
│           └── constants.rs
├── clients/
│   ├── js/                           # codama renderers-js (Solana Kit)
│   │   ├── package.json
│   │   └── src/generated/            # generated, do not edit
│   └── rust/                         # codama renderers-rust
│       ├── Cargo.toml
│       └── src/generated/
├── app/                              # frontend (Next.js / Vite / Expo)
│   └── ...
├── tests/                            # LiteSVM tests (TS or Rust)
│   ├── deposit.test.ts
│   └── ...
├── migrations/
│   └── deploy.ts
└── .solis/                           # solis scratchpad (gitignored)
    ├── cache/
    └── locks/
```

### 5.4 `solis.yml` Schema (v1)

```yaml
version: 1

workspace:
  name: my-protocol
  framework: anchor             # anchor | pinocchio (post-MVP) | shank (post-MVP)
  anchor_version: "1.0.2"
  solana_version: "2.1.0"
  rust_version: "1.79.0"

programs:
  - name: escrow
    address: "Escrow11111111111111111111111111111111111111"
    upgrade_authority: ~/.config/solana/id.json
    cluster_overrides:
      devnet:  "EscrowDevnetkD3aQVgPdMxbAv7XmgFK5Q6n8jR2"
      mainnet: "EscrowMainnetw11dD7HhT5dKqHnZxQYzN8aF4kP3K1"

clients:
  - kind: js
    renderer: "@codama/renderers-js"     # Solana Kit
    output: clients/js/src/generated
    package_name: "@my-protocol/sdk-js"
    options:
      formatter: prettier
      use_granular_imports: preferRoot
  - kind: rust
    renderer: "@codama/renderers-rust"
    output: clients/rust/src/generated
    crate_name: "my-protocol-client"
  # post-MVP renderers:
  # - kind: go
  # - kind: umi
  # - kind: vixen   (Yellowstone indexer parsers)

frontend:
  enabled: true
  template: next-app-router            # next-app-router | vite-react | expo
  wallet_adapter: solana-kit           # solana-kit | wallet-adapter-legacy
  styling: tailwind                    # tailwind | shadcn | none
  query_library: tanstack              # tanstack | none

serve:
  runtime: surfpool                    # surfpool | test-validator
  surfpool:
    fork: mainnet                      # mainnet | devnet | none
    fork_at_slot: ~                    # null = latest
    fixtures:
      programs:
        - id: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA  # SPL Token
        - id: metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s  # Token Metadata
      accounts:
        - id: 7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs
  faucet:
    amount_sol: 100
    cooldown_seconds: 0
  watch:
    paths:
      - programs/**/*.rs
      - app/**/*.{ts,tsx,js,jsx}
    debounce_ms: 300
    on_change:
      - rust: ["anchor build", "codama run"]
      - frontend: ["pnpm --filter app dev"]

deploy:
  pre_hooks:
    - cmd: "anchor build --verifiable"
  verify: true                          # solana-verify integration

plugins:
  - source: github.com/solis-apps/spl-token-2022
    version: v0.4.1
  - source: github.com/solis-apps/metaplex-nft
    version: v0.2.0
  - source: ./local-plugin              # local path for development

scaffolding:
  conventions:
    naming:
      instructions: snake_case          # enforced
      accounts: PascalCase
      events: PascalCase
    seeds_prefix: optional              # if set, all PDAs auto-prefixed
    signer_default: payer
```

A versioned JSON Schema (`schemas/solis.v1.json`) is checked in and used both for editor validation and for the Go parser (`pkg/config`).

---

## 6. Command Surface (CLI Specification)

All commands follow `solis <noun> <verb> [args] [flags]`.

### 6.1 Top-Level Map

```
solis
├── chain
│   ├── new       <name>                       Bootstrap a new workspace
│   ├── build                                  anchor build + codama run + idl sync
│   ├── serve                                  Surfpool + watch + faucet + frontend dev
│   ├── deploy    [--cluster <c>] [--verify]   solana program deploy with checks
│   ├── upgrade   [--anchor v1.1.0]            Migrate Anchor / Solana / Rust versions
│   └── doctor                                 Diagnose toolchain & config issues
├── scaffold
│   ├── program       <name>                   New program in workspace
│   ├── instruction   <name> --program <p>     New instruction with args/accounts
│   ├── account       <Name> --program <p>     New account state struct
│   ├── event         <Name> --program <p>     New emit! event
│   ├── error         <Name> --program <p>     New custom error
│   ├── crud          <Resource> --program <p> Composite recipe (state + 4 ix + 4 ix tests + client + hook)
│   ├── spl-token     [--ext transfer-hook]    Token-2022 recipe
│   ├── metaplex-nft  [--collection]           Token Metadata recipe
│   └── test          <ix-name>                Generate test scaffold for an existing instruction
├── generate
│   ├── clients       [--kind js|rust|go]      Re-run codama (subset or all)
│   ├── idl                                    anchor idl build (force regen)
│   ├── frontend-hooks                         React Query / Solid Query wrappers
│   ├── indexer       [--target vixen]         Yellowstone indexer scaffold
│   └── verifiable-build                       solana-verify manifest
├── app
│   ├── install  <source>[@<version>]
│   ├── uninstall <name>
│   ├── list
│   ├── update [<name>]
│   └── describe <name>
├── doctor                                     Standalone diagnostic
├── completion <shell>                         bash | zsh | fish | powershell
└── version
```

### 6.2 Representative Command Specifications

#### 6.2.1 `solis chain new`

```
solis chain new <name> [flags]

Flags:
  --framework string          anchor | pinocchio (post-MVP)        (default "anchor")
  --template string           multiple | single (Anchor only)       (default "multiple")
  --frontend string           next | vite | expo | none             (default "next")
  --clients strings           js,rust,go (comma-separated)          (default "js")
  --org string                Organization for package names
  --no-git                    Skip git init
  --no-install                Skip pnpm install
  --solana-version string     Override default Solana version
  --anchor-version string     Override default Anchor version
  --dir string                Output directory (default: <name>)
  --plugin strings            Plugins to install on init

Behavior:
  1. Verify toolchain (anchor, solana, cargo, pnpm/node)
  2. Render workspace template → write files
  3. Write solis.yml with declared versions
  4. Run `anchor build` to verify compile
  5. Run `codama run` to bootstrap clients
  6. Optionally `git init` and commit
  7. Print next-steps cheatsheet

Exit codes:
  0  success
  1  generic error
  2  toolchain missing/version mismatch
  3  filesystem error (target dir exists, no perms)
  4  validation failed (invalid name, reserved keyword)
```

#### 6.2.2 `solis scaffold instruction`

```
solis scaffold instruction <name> --program <program> [flags]

Required flags:
  --program string       Program name (must exist)

Optional flags:
  --args string          "amount:u64,recipient:Pubkey,memo:string"
  --accounts string      "vault:mut,depositor:signer,system_program"
  --signer string        Default signer account (default: "payer")
  --pda string           PDA derivation: "seeds=vault,owner;bump"
  --returns string       Return type (default: none)
  --emit strings         Events to emit (must already exist or be scaffolded)
  --no-test              Skip test scaffold

Account modifiers (suffix after colon, comma-separated):
  :mut          AccountInfo with #[account(mut)]
  :signer       Must sign
  :init         #[account(init, payer=..., space=...)]
  :close        #[account(close = <dest>)]
  :seeds=...    PDA seeds (e.g. seeds="vault,owner.key().as_ref()")
  :token        token::TokenAccount type
  :mint         token::Mint type
  :ata          associated_token::AssociatedToken

Examples:
  solis scaffold instruction deposit \
    --program escrow \
    --args "amount:u64" \
    --accounts "vault:mut:seeds=vault,depositor.key().as_ref(),depositor:signer:mut,system_program" \
    --emit "Deposited"

  solis scaffold instruction initialize \
    --program escrow \
    --args "fee_bps:u16" \
    --accounts "config:init:seeds=config:space=8+2:payer=admin,admin:signer:mut,system_program"

Files touched (delta):
  programs/escrow/src/instructions/deposit.rs        (CREATED)
  programs/escrow/src/instructions/mod.rs            (MARKER: append `pub mod deposit;`)
  programs/escrow/src/lib.rs                         (MARKER: append dispatch entry)
  tests/escrow/deposit.test.ts                       (CREATED unless --no-test)
  target/idl/escrow.json                             (REGEN via `anchor build`)
  clients/js/src/generated/instructions/deposit.ts   (REGEN via codama)
  clients/rust/src/generated/instructions/deposit.rs (REGEN via codama)

Idempotency:
  Re-running the same command produces no diff.
  Re-running with different args fails with a clear error pointing to
  `solis scaffold instruction --help` and recommending manual edit + regen.
```

#### 6.2.3 `solis scaffold crud`

```
solis scaffold crud <Resource> --program <program> [flags]

Generates the composite "create, read, update, delete" pattern:

  Resource:        Post (PascalCase, becomes account name)
  Account state:   programs/<p>/src/state/post.rs
  Instructions:    programs/<p>/src/instructions/{create_post,update_post,delete_post}.rs
  Events:          PostCreated, PostUpdated, PostDeleted
  Errors:          PostNotFound, PostUnauthorized
  Tests:           tests/<p>/post.test.ts
  Frontend hooks:  app/hooks/use-post.ts (if frontend enabled)
                   - usePost(address)        — read (TanStack Query)
                   - useCreatePost()         — mutation
                   - useUpdatePost(address)  — mutation
                   - useDeletePost(address)  — mutation

Flags:
  --program string       (required)
  --fields string        "title:string,body:string,author:Pubkey,published:bool"
  --pda string           Seeds for the resource PDA (default: "<resource>,<first_pubkey_field>")
  --no-update            Skip update_<resource> instruction
  --no-delete            Skip delete_<resource> instruction
  --no-frontend          Skip frontend hooks
  --no-events            Skip event emissions

Idempotency:
  Each generated artifact is independently re-runnable; the composite
  recipe is implemented internally as N atomic scaffolds wrapped in a
  transaction (filesystem-level dry-run + commit pattern, see § 7.7).
```

#### 6.2.4 `solis chain serve`

```
solis chain serve [flags]

Flags:
  --runtime string         surfpool | test-validator        (default from solis.yml)
  --no-fork                Skip mainnet fork
  --no-frontend            Skip frontend dev server
  --no-codama              Skip auto-regenerating clients on rust changes
  --port int               RPC port (default 8899)
  --frontend-port int      Frontend dev port (default 3000)
  --headless               Plain log output (no TUI)
  --reset                  Wipe ledger before start

TUI layout (when not --headless):
  ┌─────────────────────────────────────────────────────────────────┐
  │ solis chain serve                                  q to quit    │
  ├──────────────────────────────┬──────────────────────────────────┤
  │ [validator]                  │ [build]                          │
  │ surfpool 0.7.x  RPC :8899    │ anchor build  ✓ 2.3s             │
  │ slot 1234567 → 1234572       │ codama run    ✓ 0.4s             │
  │ tps 12.3                     │ idl diff: +1 instruction         │
  ├──────────────────────────────┼──────────────────────────────────┤
  │ [faucet]                     │ [frontend]                       │
  │ 3 airdrops served            │ next dev :3000  HMR active       │
  │ last: G3...x9k  100 SOL      │ last reload 0.12s                │
  ├──────────────────────────────┴──────────────────────────────────┤
  │ [logs] (mux of validator, build, frontend)                      │
  │ ...                                                              │
  └─────────────────────────────────────────────────────────────────┘

Exit signals:
  Ctrl-C   Graceful shutdown (drain build queue, stop processes in reverse order)
  Ctrl-R   Force re-run all build pipelines
  Ctrl-L   Clear log panel
```

### 6.3 Exit Code Convention

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Generic runtime error |
| 2 | Toolchain (missing or version mismatch) |
| 3 | Filesystem (permissions, conflict) |
| 4 | Validation (input args, config schema) |
| 5 | Network (codama/surfpool registry unreachable) |
| 6 | Compile (downstream `cargo`/`anchor` failure) |
| 7 | Plugin (plugin process crashed or returned error) |
| 8 | User cancelled (Ctrl-C during interactive flow) |

Used uniformly to allow shell-script and CI consumers to react.


---

## 7. Nested ADRs (Operational Sub-Decisions)

These are the substantive technical decisions implied by § 4 that warrant their own context/decision/consequences treatment. They are nested here rather than split into separate files because they are tightly coupled to this ADR's lifecycle.

### 7.1 Sub-ADR-001 — Rust Code Mutation Strategy

**Context.** Incremental scaffolding requires mutating existing Rust files (`lib.rs` dispatch, `instructions/mod.rs`, `state/mod.rs`, `errors.rs`). Three operational strategies were considered (also discussed in the parent ADR conversation):

1. **(a) Marker-bound segment editing.** Solis writes regions delimited by structured comments like `// solis:instructions:begin` / `// solis:instructions:end`, and only edits inside these regions via line-oriented insertion/removal.
2. **(b) `tree-sitter-rust` via cgo binding.** Parse the file into a CST, locate insertion points by AST query, emit modifications.
3. **(c) Subprocess `syn` daemon.** Run a small Rust binary (`solis-rustfmt`) that exposes `parse`, `add-mod-line`, `add-dispatch-arm` over JSON-RPC stdio.
4. **(d) `ast-grep` subprocess.** Mature CLI (tree-sitter-based) for structural search/replace; YAML rule files.

**Decision.** Use **(a) primarily**, with **(d) as escape hatch for non-marked regions**.

- Marker regions are placed in files **solis generates itself** (the `mod.rs`, the `#[program]` dispatch block in `lib.rs`, the `errors.rs` enum, the `events.rs` block). Outside these regions, the user's code is opaque to `solis`.
- For the rare case where insertion is needed in user-edited territory (e.g. an `#[event]` derive must reference a struct in a user-named module), invoke `ast-grep` via subprocess. `ast-grep` is a single binary, distributable as a sidecar.
- **Never** use raw regex on Rust source. Markers are matched by exact-string scan + line boundaries, not regex.

**Marker format specification** (canonical):

```rust
// === solis:auto-generated:begin segment=instructions version=1 ===
// DO NOT EDIT THIS REGION. Manual changes will be overwritten by `solis scaffold`.
// To extend, use: solis scaffold instruction <name> --program <prog>
pub mod initialize;
pub mod deposit;
pub mod withdraw;
// === solis:auto-generated:end segment=instructions ===
```

- `segment` is required and uniquely identifies the region within a file.
- `version` allows future migration of marker syntax without breaking existing workspaces.
- Markers survive `rustfmt` because they are line comments outside any expression. This is **tested in CI** (§ 9.5.1).
- The CLI validates markers on every scaffold run; corrupted markers fail loudly with an error pointing to recovery (`solis chain doctor --fix-markers`).

**Consequences.**

- Positive: simple implementation, no Rust toolchain dependency at runtime for marker ops, deterministic output, fast.
- Negative: limits scaffolding flexibility — anything that requires modifying user-authored logic is out of scope.
- Mitigation for the negative: this is **by design**. Solis manages structural plumbing; the human writes business logic. Tool stays predictable.

### 7.2 Sub-ADR-002 — Runtime Engine Selection

**Context.** `solis chain serve` needs a local Solana runtime. Options: `solana-test-validator` (Agave), `surfpool` (LiteSVM-based, fork-capable), `solana-program-test` (in-process, library), `mucho validator` (wrapper around test-validator).

**Decision.** **Surfpool is the default;** `solana-test-validator` is a fallback (`--runtime test-validator`).

- Surfpool provides JIT mainnet fork via LiteSVM, IaC primitives, drop-in compatibility with `solana-test-validator` RPC, cheatcodes (time travel, balance manipulation), and an MCP server for agent workflows.
- It is a single binary (`surfpool`), distributable. Surfpool is invoked as a managed subprocess; `solis` reads its stdout JSON events and feeds the TUI.
- Fallback to `solana-test-validator` exists because Surfpool is younger and a small fraction of edge cases (specific RPC methods, validator-specific behaviors) may not be supported. The fallback is automatic when `surfpool` is not installed and `--runtime` is unspecified, with a clear warning recommending Surfpool.

**Consequences.**

- Positive: better DX out of the box; fork mainnet without manual `--clone` flags.
- Negative: hard external dependency. Mitigated by `solis doctor` installing Surfpool when missing.

### 7.3 Sub-ADR-003 — Framework Support Priority

**Decision.**

| Framework | MVP | Phase | Rationale |
|---|---|---|---|
| Anchor 1.0 (template `multiple`) | ✅ Yes | 0–4 | ~85% of mainnet programs; only framework with stable IDL emission for Codama |
| Pinocchio | ❌ No (Phase 5+) | 5 | Production-grade but no framework; Shank IDL flow needed; perf-critical use cases only |
| Native (no framework) | ❌ Out of scope | n/a | Out of scope; users should use Pinocchio |
| Shank standalone | ❌ Out of scope | n/a | Codama supports it but adoption is low; cover via Pinocchio recipes |

**Consequences.** Scope discipline at MVP. Multi-framework support is non-trivial because each has its own IDL semantics; deferring is the only way to ship.

### 7.4 Sub-ADR-004 — Codama as Hard Dependency

**Decision.** Codama is a required runtime dependency for `solis generate clients`.

- `solis` does **not** re-implement Codama. The wrapper (`pkg/codama`) maintains a `codama.json` from `solis.yml`, invokes `pnpm exec codama run` as subprocess.
- `solis chain new` ensures `pnpm` is installed and runs `pnpm install` to bring in `codama` + selected renderers.
- For non-Node users, `solis` also vendors a pinned `node` (~30 MB) inside `~/.solis/runtimes/node-<version>` lazily, downloaded on first need. This keeps the user-facing dependency tree minimal.

**Consequences.**

- Positive: alignment with the ecosystem's de-facto codegen; SPL programs already use Codama; Solana Kit native.
- Negative: Node dependency. Mitigated as above.

### 7.5 Sub-ADR-005 — Plugin System: Multi-Language Protocol

**Decision.** Plugins implement one of two transports; the choice is declared in the plugin manifest.

| Transport | When to use | Tech |
|---|---|---|
| **gRPC over Unix domain socket** (HashiCorp `go-plugin`) | Plugin written in Go; performance-critical hooks (build, watch); needs typed interface | `go-plugin` v1, `.proto` shared schema |
| **JSON-RPC over stdio** (LSP-style framing) | Plugin written in TS/Node/Python/Rust; ecosystem proximity | Headers `Content-Length: N\r\n\r\n{json}`, version negotiation via `initialize` |

**Plugin manifest** (`solis-plugin.json` in plugin root):

```json
{
  "name": "spl-token-2022",
  "version": "0.4.1",
  "description": "Token-2022 extensions recipes for solis",
  "transport": "stdio-jsonrpc",
  "entrypoint": ["node", "dist/index.js"],
  "engines": {
    "solis": ">=0.3.0"
  },
  "capabilities": {
    "commands": [
      {
        "verb": "scaffold",
        "noun": "transfer-hook",
        "flags": [
          { "name": "mint", "type": "string", "required": true },
          { "name": "hook-program", "type": "string", "required": true }
        ]
      }
    ],
    "hooks": ["pre-build", "post-codama"]
  }
}
```

**Plugin discovery.** Solis searches in this order: `./solis.yml` `plugins[]`, `$SOLIS_PLUGIN_PATH`, `~/.solis/plugins/`. Plugins are sandboxed: filesystem access restricted to the workspace root and a per-plugin scratch dir; network access requires explicit declaration in the manifest (`capabilities.network: true`) and triggers a one-time user confirmation.

**Consequences.**

- Positive: ecosystem adoption path that does not force Go on plugin authors. Recipe authors can publish on npm and Go modules alike.
- Negative: two protocols to maintain. Mitigated by a thin Go adapter (`pkg/plugin/adapter`) that exposes a single internal interface to scaffold/generate code paths.

### 7.6 Sub-ADR-006 — Configuration Format: YAML, Versioned

**Decision.** YAML with explicit `version: 1` field; JSON Schema for validation; viper as parser.

- TOML was considered (matches `Anchor.toml`, `Cargo.toml`, `Solana.toml`) but rejected: solis.yml needs nested arrays and conditional sections that TOML expresses awkwardly.
- JSON was considered but rejected for ergonomics (no comments, brittle for humans).
- YAML "version: 1" allows breaking the schema in v2 with an automated `solis chain upgrade` migration path.

**Consequences.** One more config file in user workspaces, alongside `Anchor.toml`, `codama.json`, `Cargo.toml`, `package.json`. Mitigated by `solis chain doctor` cross-validating consistency across all of them.

### 7.7 Sub-ADR-007 — Filesystem Transaction Semantics

**Context.** A composite scaffold (`solis scaffold crud`) touches 10+ files. If step 7 fails, the workspace must not be left half-mutated.

**Decision.** Implement a **two-phase filesystem commit**:

1. **Plan phase:** all file operations are computed into an in-memory `FileSetPlan` (creates, updates, marker-region edits) without touching disk.
2. **Validate phase:** plan is dry-run — schema-check generated content, lint markers, verify destination paths are within workspace, check for conflicts (file exists and would be overwritten).
3. **Commit phase:** writes happen atomically per file (write to `<path>.solis-tmp.<pid>`, rename). On error, all previously written files are rolled back using an undo log.
4. **Post-commit hooks:** `cargo fmt --files-with-diff <changed.rs>`, optional `cargo check`.

This is encoded as `pkg/fsutil.Transaction`:

```go
type Transaction interface {
    Plan(ops ...FileOp) error
    Validate(ctx context.Context) error
    Commit(ctx context.Context) (CommitReport, error)
    Rollback() error
}
```

**Consequences.** Slight implementation overhead, but ensures idempotency and partial-failure recovery. Crucial for CI.

### 7.8 Sub-ADR-008 — IDL Diff & Breaking-Change Detection

**Decision.** Before any `solis chain deploy`, compute an IDL diff between the local `target/idl/<prog>.json` and the on-cluster IDL (if program is upgrade-authority-owned and IDL is on-chain). Classify:

- **Compatible:** new instruction added, new optional account, new error code with higher number.
- **Breaking:** instruction signature changed, account struct field added/removed/reordered, enum variants reordered, custom discriminator changed.

Use `pkg/idl/differ` (pure-Go, no external IDL diff lib exists). Block deploy on breaking changes unless `--allow-breaking` is passed. Print human-readable diff.

**Consequences.** Operates only on IDL semantics; does not catch business-logic regressions. Pairs with `solis scaffold test` and LiteSVM tests for full coverage.


---

## 8. Detailed Component Specifications

### 8.1 Module Layout (Go)

```
solis/
├── go.mod
├── go.sum
├── Makefile
├── .goreleaser.yml
├── cmd/
│   └── solis/
│       └── main.go                    # entrypoint, signal handling, root cobra cmd
├── internal/
│   ├── cli/
│   │   ├── root.go                    # cobra root + global flags
│   │   ├── chain_new.go
│   │   ├── chain_serve.go
│   │   ├── chain_build.go
│   │   ├── chain_deploy.go
│   │   ├── chain_doctor.go
│   │   ├── scaffold_program.go
│   │   ├── scaffold_instruction.go
│   │   ├── scaffold_account.go
│   │   ├── scaffold_event.go
│   │   ├── scaffold_error.go
│   │   ├── scaffold_crud.go
│   │   ├── generate_clients.go
│   │   ├── generate_idl.go
│   │   ├── app_install.go
│   │   ├── app_list.go
│   │   └── ...
│   ├── config/
│   │   ├── config.go                  # Config struct, defaults
│   │   ├── schema.go                  # JSON Schema embed + validator
│   │   ├── loader.go                  # viper integration, env overrides
│   │   ├── migrator.go                # v1 → v2 migrations
│   │   └── schemas/
│   │       └── solis.v1.json
│   ├── scaffold/
│   │   ├── engine.go                  # Scaffolder interface, dispatcher
│   │   ├── workspace.go
│   │   ├── program.go
│   │   ├── instruction.go
│   │   ├── account.go
│   │   ├── event.go
│   │   ├── errors.go
│   │   ├── crud.go
│   │   └── recipes/                   # high-level recipes (spl-token, metaplex-nft)
│   ├── codegen/
│   │   ├── codama.go                  # codama subprocess wrapper
│   │   ├── codama_config.go           # generates codama.json from solis.yml
│   │   └── frontend_hooks.go          # React Query / Solid Query wrappers
│   ├── idl/
│   │   ├── parser.go                  # Anchor IDL JSON parse
│   │   ├── differ.go                  # IDL diff/classify
│   │   ├── onchain.go                 # fetch on-chain IDL
│   │   └── types.go
│   ├── rustpatch/
│   │   ├── marker.go                  # marker scan/insert/remove
│   │   ├── segment.go                 # segment types & registry
│   │   ├── astgrep.go                 # ast-grep subprocess wrapper
│   │   └── fmt.go                     # rustfmt invocation
│   ├── runtime/
│   │   ├── runner.go                  # Runtime interface
│   │   ├── surfpool.go
│   │   ├── testvalidator.go           # fallback
│   │   ├── faucet.go
│   │   ├── watcher.go                 # fsnotify-based file watcher
│   │   └── pipeline.go                # change → build → codama orchestration
│   ├── tui/
│   │   ├── serve_model.go             # bubble tea model for `chain serve`
│   │   ├── panels.go
│   │   └── theme.go
│   ├── plugin/
│   │   ├── manager.go                 # discovery, lifecycle
│   │   ├── grpc/                      # hashicorp go-plugin transport
│   │   ├── stdio/                     # JSON-RPC stdio transport
│   │   ├── manifest.go
│   │   └── sandbox.go
│   ├── toolchain/
│   │   ├── anchor.go                  # anchor invocation, version detection
│   │   ├── solana.go                  # solana CLI invocation
│   │   ├── cargo.go
│   │   ├── codama.go
│   │   ├── surfpool.go
│   │   ├── installer.go               # auto-install missing tools
│   │   └── verifier.go
│   ├── templates/
│   │   ├── embed.go                   # embed.FS root
│   │   ├── funcs.go                   # sprig + custom template funcs
│   │   ├── render.go                  # rendering pipeline
│   │   └── assets/
│   │       ├── workspace/             # files for `solis chain new`
│   │       ├── program/
│   │       ├── instruction/
│   │       ├── account/
│   │       ├── event/
│   │       ├── error/
│   │       ├── crud/
│   │       └── frontend/
│   │           ├── next/
│   │           ├── vite/
│   │           └── expo/
│   ├── fsutil/
│   │   ├── transaction.go             # filesystem two-phase commit
│   │   ├── atomic.go                  # atomic write helpers
│   │   └── undo.go
│   └── version/
│       └── version.go                 # build-time injected version, semver
├── pkg/                               # public API (for plugins to import)
│   └── plugin/
│       ├── proto/
│       │   └── plugin.proto
│       └── api/
│           └── api.go
├── test/
│   ├── e2e/
│   ├── golden/
│   ├── compile/
│   └── fixtures/
└── docs/
    ├── adr/
    ├── reference/
    └── guides/
```

### 8.2 Core Interfaces

```go
// internal/scaffold/engine.go

// Scaffolder is the unit of incremental code generation.
type Scaffolder interface {
    // Name returns the noun (e.g. "instruction").
    Name() string

    // Validate checks args against the workspace state. Pure function;
    // must not touch the filesystem beyond reading config.
    Validate(ctx context.Context, ws Workspace, args Args) error

    // Plan computes the FileSet of operations needed. Idempotent.
    Plan(ctx context.Context, ws Workspace, args Args) (*fsutil.FileSet, error)
}

// Workspace is the in-memory view of a solis project.
type Workspace struct {
    Root         string
    Config       *config.Config
    Programs     []ProgramView
    IDLs         map[string]*idl.IDL    // by program name
    Clients      []ClientView
    Frontend     *FrontendView
}

// Args is a strongly-typed bag of scaffold arguments, validated against
// the JSON Schema embedded for each scaffolder.
type Args map[string]any
```

```go
// internal/rustpatch/marker.go

// Marker delimits a solis-managed region inside a .rs file.
type Marker struct {
    Segment  string // e.g. "instructions", "dispatch", "errors"
    Version  int    // schema version of the marker syntax
    Begin    int    // line number (1-indexed), inclusive
    End      int    // line number, inclusive (the END marker line)
}

// Patch represents a region edit.
type Patch struct {
    File     string
    Segment  string
    Lines    []string // body to write between markers (excludes marker comments)
}

// Apply applies one or more patches in a single pass.
// File is parsed once; markers must exist; mismatched markers return error.
func Apply(file string, patches []Patch) error
```

```go
// internal/runtime/runner.go

type Runtime interface {
    Name() string                       // "surfpool" | "test-validator"
    Start(ctx context.Context, cfg Config) (Handle, error)
}

type Handle interface {
    RPCEndpoint() string
    WSEndpoint() string
    Faucet() Faucet
    Logs() <-chan LogLine
    Health() <-chan Health
    Stop(ctx context.Context) error
}
```

```go
// internal/codegen/codama.go

type Codama struct {
    BinPath string                      // path to `codama` (resolved by toolchain)
    Cwd     string                      // workspace root
}

// EnsureConfig writes codama.json from solis.yml. Idempotent.
func (c *Codama) EnsureConfig(cfg *config.Config) error

// Run executes one or more renderer scripts.
// names empty == --all.
func (c *Codama) Run(ctx context.Context, names ...string) (Report, error)
```

### 8.3 Template Engine

- **Engine:** `text/template` + sprig.
- **Custom functions registry** in `internal/templates/funcs.go`:

| Function | Purpose | Example |
|---|---|---|
| `pascal` | snake_case → PascalCase | `{{ pascal "deposit_funds" }}` → `DepositFunds` |
| `camel` | snake_case → camelCase | `{{ camel "deposit_funds" }}` → `depositFunds` |
| `snake` | PascalCase → snake_case | `{{ snake "DepositFunds" }}` → `deposit_funds` |
| `kebab` | PascalCase → kebab-case | for npm package names |
| `borshSize` | Compute borsh size of a type | `{{ borshSize "Pubkey" }}` → `32` |
| `accountSpace` | Compute `space=` for `init` | `{{ accountSpace .Fields }}` → integer |
| `anchorType` | Map IDL type → Rust type | `{{ anchorType "Pubkey" }}` → `Pubkey` |
| `rustImport` | Build idiomatic `use` line | hides path verbosity |
| `pdaSeeds` | Render seeds tuple | from `vault,owner.key().as_ref()` syntax |

- **Templates are embedded** via `//go:embed assets/*` into the binary. No filesystem reads at runtime.
- **Template debugging:** `solis scaffold instruction --dry-run --debug` prints the parsed args, rendered template, and the resulting `FileSet` without committing.

### 8.4 Sample Generated File (Anchor 1.0 instruction)

For `solis scaffold instruction deposit --program escrow --args "amount:u64" --accounts "vault:mut:seeds=vault,depositor:signer:mut,system_program" --emit "Deposited"`:

```rust
// programs/escrow/src/instructions/deposit.rs
// === solis:auto-generated:begin segment=file version=1 generator=instruction ===
// This file is initial scaffolding. The handler body below the marker can be edited.
// Re-running `solis scaffold instruction deposit` with the same args is a no-op.

use anchor_lang::prelude::*;

use crate::events::Deposited;
use crate::state::Vault;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault"],
        bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    pub system_program: Program<'info, System>,
}
// === solis:auto-generated:end segment=file ===

// === solis:user-region:begin segment=handler ===
// You can freely edit anything inside this region.
pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // TODO: implement
    emit!(Deposited {
        amount,
        depositor: ctx.accounts.depositor.key(),
    });
    Ok(())
}
// === solis:user-region:end segment=handler ===
```

Then `programs/escrow/src/instructions/mod.rs` gets patched:

```rust
// === solis:auto-generated:begin segment=instructions version=1 ===
// DO NOT EDIT. Use `solis scaffold instruction` to extend.
pub mod initialize;
pub mod deposit;       // <-- added
// === solis:auto-generated:end segment=instructions ===

pub use initialize::*;
pub use deposit::*;    // <-- added
```

And `programs/escrow/src/lib.rs`:

```rust
use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Escrow11111111111111111111111111111111111111");

#[program]
pub mod escrow {
    use super::*;

    // === solis:auto-generated:begin segment=dispatch version=1 ===
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::initialize::handler(ctx, fee_bps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {    // <-- added
        instructions::deposit::handler(ctx, amount)
    }
    // === solis:auto-generated:end segment=dispatch ===
}
```

The "user region" marker is the key innovation enabling round-tripping: solis owns the `Accounts` struct (which is mechanically derivable from the CLI args), but the **handler body is the user's**. Re-running the same command preserves user code.


---

## 9. Test Strategy

### 9.1 Test Pyramid

```
                  ┌──────────────────┐
                  │   E2E (LiteSVM)  │      ~5%   slow, semantic truth
                  └──────────────────┘
                ┌────────────────────────┐
                │  Compile tests (cargo) │  ~10%   real toolchain
                └────────────────────────┘
            ┌──────────────────────────────────┐
            │  Integration (subprocess mocked) │   ~15%
            └──────────────────────────────────┘
        ┌────────────────────────────────────────────┐
        │     Golden file tests (snapshots)          │   ~30%
        └────────────────────────────────────────────┘
    ┌──────────────────────────────────────────────────────┐
    │              Unit tests (Go, table-driven)            │   ~40%
    └──────────────────────────────────────────────────────┘
```

### 9.2 Unit Tests (Go)

- **Tooling:** standard `testing` package, `testify/require` for ergonomics, `testify/mock` for boundaries.
- **Targets:**
  - `internal/config`: load valid, reject invalid, default merging, env override, schema validation.
  - `internal/idl`: parse all known IDL variants (0.30, 0.31, 0.32, 1.0); IDL differ produces correct classification.
  - `internal/rustpatch/marker`: scan, validate, apply, idempotency.
  - `internal/templates/funcs`: every function with property-based tests for inverse pairs (`pascal(snake(x)) == x` for valid inputs).
  - `internal/scaffold/*`: each scaffolder's `Validate` and `Plan` (pure, easy to test).
  - `internal/codegen/codama_config`: solis.yml → codama.json mapping.
  - `internal/plugin/manifest`: parse, validate, version negotiation.
  - `internal/fsutil/transaction`: simulate disk errors at each step, verify rollback.
- **Coverage target:** **≥ 85% statement coverage** on `internal/`. Enforced in CI via `go test -coverprofile`.

### 9.3 Golden File Tests (Snapshot)

- **Tooling:** [`github.com/sebdah/goldie/v2`](https://github.com/sebdah/goldie).
- **Layout:** `test/golden/<scaffolder>/<case>/`
  - `args.yaml` — input arguments
  - `expected/` — tree of expected output files (full content)
- **Pattern:**

```go
func TestScaffoldInstruction_Deposit(t *testing.T) {
    cases := []string{
        "deposit_simple",
        "deposit_with_pda",
        "deposit_with_event",
        "deposit_with_token_accounts",
        "deposit_idempotent_second_run",
        "transfer_with_signer_seeds",
        // ... 30+ cases
    }
    for _, name := range cases {
        t.Run(name, func(t *testing.T) {
            ws := loadFixtureWorkspace(t, name)
            args := loadArgs(t, name)
            plan, err := scaffold.NewInstructionScaffolder().Plan(ctx, ws, args)
            require.NoError(t, err)
            for _, op := range plan.Ops {
                g := goldie.New(t, goldie.WithFixtureDir(filepath.Join("testdata", name)))
                g.Assert(t, op.Path, op.Content)
            }
        })
    }
}
```

- **Update mode:** `UPDATE_GOLDEN=1 go test ./...` regenerates golden files. Diffs reviewed in PR.
- **Required cases per scaffolder:**
  - `scaffold_instruction`: ≥ 25 cases covering signer permutations, PDA seeds with/without bump, init/close, token account types, events emitted, error returns, missing optional args.
  - `scaffold_account`: ≥ 15 cases covering field type combinations, PDA seed patterns, space calculation.
  - `scaffold_crud`: ≥ 8 cases (with/without update, with/without delete, with/without frontend, multiple resources in same program).
  - `scaffold_workspace`: ≥ 6 cases (anchor/frontend combinations).

### 9.4 Compile Tests

These are the most expensive but most important tests. They prove that scaffolded code **builds**.

- **Tooling:** Go test invoking `cargo` in a tempdir.
- **Pattern:**

```go
//go:build compile
// +build compile

func TestCompile_InstructionScaffolds(t *testing.T) {
    for _, fx := range listGoldenFixtures(t, "scaffold_instruction") {
        t.Run(fx, func(t *testing.T) {
            t.Parallel()
            tmpWs := setupAnchorWorkspace(t)              // fresh `anchor init`
            applyFixture(t, tmpWs, fx)                    // run the scaffold
            runCargo(t, tmpWs, "check", "--manifest-path",
                filepath.Join(tmpWs, "Cargo.toml"))
        })
    }
}
```

- **Optimization:** workspace is cached (`anchor init` runs once, then `git clean -fdx` between cases).
- **Coverage:** every public scaffold combinatorial dimension (signer × PDA × init × event) gets at least one compile test.
- **CI:** runs in a dedicated job with longer timeout (15 min) on every PR; expected to take ~3–5 min once parallelized.

### 9.5 Integration Tests

Cover the seams between solis and external tools.

#### 9.5.1 Markers survive `rustfmt`

```go
func TestMarkers_PreservedByRustfmt(t *testing.T) {
    src := readGolden(t, "instructions_mod_with_markers.rs")
    require.NoError(t, os.WriteFile(tmpPath, src, 0o644))
    require.NoError(t, runCmd(t, "rustfmt", "--edition=2021", tmpPath))
    after := readFile(t, tmpPath)
    markers := rustpatch.Scan(after)
    require.Len(t, markers, expectedMarkerCount)
    // All marker segments and versions identical:
    for _, m := range markers {
        require.Equal(t, expectedSegments[m.Segment], m)
    }
}
```

#### 9.5.2 IDL round-trip via real `anchor build`

```go
func TestIDLRoundtrip_DepositInstruction(t *testing.T) {
    ws := scaffoldFreshWorkspace(t, "escrow")
    runSolis(t, ws, "scaffold", "instruction", "deposit",
        "--program", "escrow",
        "--args", "amount:u64",
        "--accounts", "vault:mut,depositor:signer:mut,system_program")
    runCmd(t, "anchor", "build", "--manifest-path", ws+"/Cargo.toml")
    idl := parseIDL(t, filepath.Join(ws, "target/idl/escrow.json"))
    require.True(t, idl.HasInstruction("deposit"))
    require.Equal(t, "u64", idl.Instruction("deposit").Args[0].Type)
}
```

#### 9.5.3 Codama produces clients from scaffold

```go
func TestCodama_ClientsFromScaffold(t *testing.T) {
    ws := setupWorkspace(t)
    runSolis(t, ws, "scaffold", "instruction", "deposit", ...)
    runSolis(t, ws, "chain", "build")
    requireFile(t, ws, "clients/js/src/generated/instructions/deposit.ts")
    requireFile(t, ws, "clients/rust/src/generated/src/generated/instructions/deposit.rs")
    requireCompiles(t, ws, "clients/js")    // tsc --noEmit
    requireCompiles(t, ws, "clients/rust")  // cargo check
}
```

### 9.6 E2E Tests with LiteSVM

The deepest validation: a scaffolded instruction is executed in LiteSVM via the scaffolded client.

```go
func TestE2E_DepositExecutes(t *testing.T) {
    ws := setupWorkspace(t)
    runSolis(t, ws, "scaffold", "crud", "Vault",
        "--program", "escrow",
        "--fields", "owner:Pubkey,balance:u64")
    runSolis(t, ws, "chain", "build")
    runCmd(t, "pnpm", "--filter", "tests", "test", "-- --litesvm")
    // tests/escrow/vault.test.ts uses @solana-developers/helpers + litesvm
    // to call createVault / readVault / updateVault / deleteVault.
}
```

Tests use the generated client (not hand-written), proving the full pipeline.

### 9.7 Property-Based & Fuzz Tests

Where applicable:

- `rustpatch.Apply`: fuzz with random insertions/deletions, verify markers stay consistent.
- Template functions: property tests as above.
- `idl.Differ`: fuzz both IDLs, verify classification never panics; specific tests for hand-crafted breaking-change scenarios.

### 9.8 CI Matrix

```yaml
# .github/workflows/ci.yml (excerpt)

jobs:
  unit:
    strategy:
      matrix:
        os: [ubuntu-22.04, ubuntu-24.04, macos-14]
        go: ['1.22', '1.23']
    steps:
      - run: go test -race -coverprofile=cov.out ./internal/...
      - run: |
          coverage=$(go tool cover -func=cov.out | tail -1 | awk '{print $3}' | tr -d '%')
          [[ $(echo "$coverage >= 85" | bc) -eq 1 ]] || exit 1

  golden:
    needs: unit
    runs-on: ubuntu-24.04
    steps:
      - run: go test -tags=golden ./test/golden/...

  compile:
    needs: golden
    runs-on: ubuntu-24.04
    strategy:
      matrix:
        anchor: ['1.0.2']
        solana: ['2.1.0']
        rust: ['1.79.0', 'stable']
    container:
      image: solanalabs/solana:v${{ matrix.solana }}
    steps:
      - run: rustup install ${{ matrix.rust }}
      - run: avm install ${{ matrix.anchor }}
      - run: go test -tags=compile -timeout=20m -p=4 ./test/compile/...

  integration:
    needs: compile
    runs-on: ubuntu-24.04
    steps:
      - uses: ./.github/actions/setup-toolchain
      - run: go test -tags=integration -timeout=30m ./test/integration/...

  e2e:
    needs: integration
    runs-on: ubuntu-24.04
    steps:
      - uses: ./.github/actions/setup-toolchain
      - run: cargo install litesvm-cli --locked
      - run: go test -tags=e2e -timeout=45m ./test/e2e/...

  lint:
    runs-on: ubuntu-24.04
    steps:
      - uses: golangci/golangci-lint-action@v6
      - run: go vet ./...
      - run: gofumpt -d .

  schema:
    runs-on: ubuntu-24.04
    steps:
      - run: npx -y @apidevtools/json-schema-validator-cli validate \
              internal/config/schemas/solis.v1.json
```

### 9.9 Coverage Targets (Summary)

| Layer | Metric | Target |
|---|---|---|
| Unit (Go) | Statement coverage | ≥ 85% |
| Golden | Cases per scaffolder | ≥ table in § 9.3 |
| Compile | % of scaffolders with at least one compile case | 100% |
| Integration | % of external-tool seams covered | 100% |
| E2E | % of MVP scaffolders with at least one happy-path E2E | 100% |
| Mutation testing (post-v1) | Mutation score | ≥ 60% |

### 9.10 Performance Test Targets

| Operation | Target |
|---|---|
| Cold start `solis --help` | < 50 ms |
| `solis scaffold instruction` (no compile) | < 500 ms |
| `solis chain new` (without `pnpm install`) | < 2 s |
| `solis chain new` (full, with `pnpm install` & `anchor build`) | < 90 s |
| Surfpool startup (already installed) | < 3 s |
| File change → codama re-run finished | < 4 s on a 100-instruction program |

Tracked via `benchstat` on PRs.


---

## 10. Implementation Plan

### 10.1 Phasing Overview

| Phase | Theme | Duration | Cumulative | DoD |
|---|---|---|---|---|
| **0** | Foundations | 2 wk | 2 wk | CI green, `solis version` works, golden test infra |
| **1** | Workspace + Anchor 1.0 integration | 2 wk | 4 wk | `chain new` produces compilable Anchor workspace |
| **2** | Incremental scaffolding | 4 wk | 8 wk | `scaffold {instruction, account, event, error}` end-to-end |
| **3** | Runtime orchestration | 3 wk | 11 wk | `chain serve` (Surfpool + watch + codama + TUI) |
| **4** | Codegen & frontend hooks | 2 wk | 13 wk | `generate clients` and React Query hooks |
| **5** | Recipes (CRUD, SPL, Metaplex) | 3 wk | 16 wk | `scaffold crud` works end-to-end |
| **6** | Plugin system | 4 wk | 20 wk | gRPC + stdio plugins; 2 reference plugins |
| **7** | Pinocchio support | 3 wk | 23 wk | `--framework pinocchio` MVP |
| **8** | v1.0 polish (distribution, docs) | 2 wk | 25 wk | Homebrew, snap, docs site |

Total MVP (phases 0–5): **~16 weeks of focused work**.

### 10.2 Phase 0 — Foundations (Weeks 1–2)

**Goal.** Repo skeleton, build/test/release infra, no actual scaffolding logic.

**Deliverables:**
- Go module, `cmd/solis/main.go`, `internal/cli/root.go` with cobra root.
- `solis version`, `solis doctor` (basic — detects anchor, solana, cargo, pnpm presence and versions).
- `internal/config` with `solis.yml` v1 parser, JSON Schema embedded and validated.
- `internal/toolchain` with version detection for anchor, solana, cargo, pnpm, node, surfpool, codama.
- CI workflow (lint + unit) green.
- `golangci-lint` and `gofumpt` configured.
- `goreleaser` configured for binary releases.
- Golden test infrastructure (`goldie` integrated; one trivial test to prove it works).
- ADR-0001 + ADR-0002 (CLI design conventions).

**Done When.**
- `go test ./...` passes locally and in CI.
- `solis version` prints semver from build-injected `version.Version`.
- `solis doctor` reports a useful status table.
- Cold start < 50 ms.

### 10.3 Phase 1 — Workspace Bootstrap (Weeks 3–4)

**Deliverables:**
- `internal/templates/assets/workspace/anchor-multiple/` populated.
- `solis chain new <name>` end-to-end.
- `internal/fsutil/transaction` implemented and unit-tested.
- Compile test infrastructure (`test/compile/...`).
- 3+ workspace golden fixtures (anchor + next, anchor + vite, anchor + no-frontend).

**Done When.**
- `solis chain new foo --framework anchor` produces a workspace where `anchor build` succeeds.
- Compile test on every fixture passes in CI within 5 min.

### 10.4 Phase 2 — Incremental Scaffolding (Weeks 5–8)

**Deliverables:**
- `internal/rustpatch/{marker, segment, fmt}.go` complete with tests including § 9.5.1 rustfmt preservation test.
- Scaffolders: `instruction`, `account`, `event`, `error`, `program`.
- ≥ 75 golden tests across these scaffolders.
- ≥ 25 compile tests.
- 5 integration tests (full anchor build + IDL inspection + codama regen).
- Markers documented in `/docs/reference/markers.md`.

**Done When.**
- A user can run `solis chain new` then `solis scaffold instruction X` repeatedly, with every result compiling.
- Idempotency tests: same command twice = no diff. Different args = clear error or merge as appropriate.
- `solis chain doctor --fix-markers` is implemented (detect & repair corrupted marker comments).

### 10.5 Phase 3 — Runtime Orchestration (Weeks 9–11)

**Deliverables:**
- `internal/runtime/surfpool.go` and `testvalidator.go` implementing the `Runtime` interface.
- `internal/runtime/watcher.go` with debounced fsnotify.
- `internal/runtime/pipeline.go`: file change → anchor build → codama run → notify frontend.
- `internal/tui/serve_model.go` with bubble tea panels (validator, build, faucet, frontend, logs).
- `solis chain serve` and `solis chain build` end-to-end.

**Done When.**
- `solis chain serve` launches Surfpool, regenerates clients on file change, and survives Ctrl-C with clean process tree teardown.
- TUI renders correctly on 80×24 minimum terminals.
- Headless mode (`--headless`) produces parseable logs for CI.

### 10.6 Phase 4 — Codegen & Frontend Hooks (Weeks 12–13)

**Deliverables:**
- `internal/codegen/codama.go` and `codama_config.go` complete.
- `internal/codegen/frontend_hooks.go`: generates TanStack Query / Solid Query wrappers from IDL.
- `solis generate clients`, `solis generate idl`, `solis generate frontend-hooks`.

**Done When.**
- Generated React Query hooks compile in a Next.js project and successfully query a local Surfpool instance.
- Hooks regenerate idempotently.

### 10.7 Phase 5 — Recipes (Weeks 14–16)

**Deliverables:**
- `internal/scaffold/crud.go` (composite recipe, 4 instructions + state + events + errors + tests + hooks).
- `internal/scaffold/recipes/spl_token.go` (SPL token recipe).
- `internal/scaffold/recipes/metaplex_nft.go` (Token Metadata recipe).
- `solis scaffold crud`, `solis scaffold spl-token`, `solis scaffold metaplex-nft`.
- E2E test for CRUD: scaffold a CRUD on a fresh workspace, run the generated test suite, all pass.

**Done When.**
- A user creates a working blog dApp from `solis chain new` + `solis scaffold crud Post` + `solis chain serve` in < 5 minutes, including running the generated frontend.

### 10.8 Phase 6 — Plugin System (Weeks 17–20)

**Deliverables:**
- `pkg/plugin/proto/plugin.proto` finalized.
- `internal/plugin/{grpc, stdio, manager, manifest, sandbox}.go` complete.
- Reference plugin 1: `solis-apps/spl-token-2022` (Go, gRPC) — transfer hook, confidential transfer recipes.
- Reference plugin 2: `solis-apps/indexer-vixen` (TS, stdio) — Yellowstone indexer scaffolding.
- Plugin marketplace conventions documented.

**Done When.**
- A third-party plugin can register a new `solis scaffold <noun>` command without modifying solis core.
- `solis app install` and lifecycle commands work end-to-end.

### 10.9 Out of Scope for v1.0

- Pinocchio support is **Phase 7** (post-v1.0). Scope it as a separate ADR.
- Multi-program-workspace composition beyond what Anchor already supports.
- Verifiable build orchestration beyond shelling out to `solana-verify`.
- Mobile (React Native / Solana Mobile) templates — defer to community plugins.
- AI-assisted scaffolding (LLM-suggested args) — defer indefinitely.

---

## 11. Risks & Mitigations

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| **R1** | Anchor 1.x introduces breaking changes mid-MVP | Medium | High | Pin Anchor 1.0.2 as MVP target; integration tests pin Anchor version; document compatibility matrix; `solis chain upgrade` command in Phase 5 |
| **R2** | Codama API/CLI changes between versions | High | Medium | Pin Codama version in `solis chain new` output; track upstream releases; integration tests use pinned versions; the codama wrapper exposes a thin API that translates to the version in use |
| **R3** | Surfpool too immature for production reliance | Medium | Medium | Fallback to `solana-test-validator` is built in; `surfpool` is auto-detected and only used when present |
| **R4** | rustfmt configuration variants corrupt markers | Low | High | § 9.5.1 enforced in CI on multiple rustfmt configs; markers placed on lines that all standard rustfmt configs preserve (top-of-file or between blocks) |
| **R5** | ast-grep CLI version drift | Low | Low | Pin version in `solis doctor`; ast-grep used only as escape hatch (most ops use markers) |
| **R6** | Two plugin transports (gRPC + stdio) increase maintenance | Medium | Medium | Single internal adapter; conformance test suite both transports run against; Phase 6 only ships **one transport (Go/gRPC)**, stdio added in 6.5 |
| **R7** | Naming collision with existing project ("solis" search results) | Low | Low | Quick GitHub/npm search confirms availability; if not, fall back to alternatives (`heliós`, `aurum`, `solid`) — recorded in this ADR |
| **R8** | Plugin sandboxing weak — malicious plugin reads private keys from `~/.config/solana/` | Medium | High | Sandbox restricts plugin FS access to workspace + scratch dir; network access opt-in; signing capabilities require explicit `--allow-plugin-sign` flag; document a plugin trust model |
| **R9** | Adoption blocked by overlap with Mucho CLI (Foundation tool) | High | Medium | Position solis as the **incremental + plugins** layer; integrate with Mucho (call `mucho clone` to populate fixtures when appropriate); approach Foundation early to align |
| **R10** | Compile tests slow the CI to the point of blocking PRs | Medium | Medium | Parallelize (Go test `-p`); cache anchor build artifacts; tier the test stages (unit < 1 min, golden < 2 min, compile < 5 min, integration < 10 min, e2e < 15 min); only unit + golden are required for merging — others run on `main` post-merge with revert on red |
| **R11** | Multi-program workspaces have subtle bugs in IDL diffing | Low | Medium | Differ tests include multi-program fixtures; per-program IDL isolation enforced |
| **R12** | User's existing handcrafted Anchor program adopted as workspace breaks marker scan | High | Low | `solis chain adopt` command (Phase 5+) wraps existing Anchor projects, asks user to confirm marker insertion |

---

## 12. Open Questions

| ID | Question | Required Resolution |
|---|---|---|
| **Q1** | Naming. Is `solis` available on GitHub + npm? Reserve before announce. | Before Phase 0 start |
| **Q2** | Should `solis chain new` default to single-program or multi-program-ready workspaces? Multi adds complexity early but matches real protocols. | Before Phase 1 |
| **Q3** | How should solis interact with `mucho`? Co-exist? Replace partial? Coordinate with Solana Foundation. | Before public release |
| **Q4** | Frontend hook generation: opinionated TanStack Query? Or just raw async functions? Survey real dApp codebases. | Phase 4 |
| **Q5** | Should verifiable builds be a flag on `chain deploy` (off by default) or part of CI generated by `chain new`? | Phase 1 |
| **Q6** | License: MIT (matches Anchor) or Apache-2.0 (matches Ignite)? | Phase 0 |
| **Q7** | Telemetry: opt-in usage statistics to track command popularity? If yes, design with strict privacy guarantees. | Phase 8 |
| **Q8** | Plugin marketplace governance: who curates `solis-apps`? Anyone can publish, or trusted set only? | Phase 6 |

---

## 13. Consequences

### 13.1 Positive

- **First-class scaffolding** for Solana, comparable to Ignite/Rails/Django ergonomics.
- **Workflow unification**: one tool replaces 4–5 manual steps for every new instruction.
- **Plugin ecosystem** unlocks community-driven recipes (Token-2022 variants, Metaplex updates, custom DEX patterns) without modifying core.
- **Single binary**: distribution-friendly, no Node version hell for the CLI itself.
- **Test-driven** from day one: every scaffolder ships with golden + compile coverage.
- **Composable**: plays well with Codama, Surfpool, Anchor — doesn't reinvent them.

### 13.2 Negative

- **Maintenance burden**: track Anchor + Codama + Surfpool releases. Mitigated by version-pinning and a release cadence aligned with Anchor minors.
- **Learning curve** for users coming from raw `anchor` — they need to understand the marker system. Mitigated by `solis chain doctor` and good docs.
- **Initial adoption friction**: new tool, ecosystem skepticism. Mitigated by clear differentiation from `anchor`/`mucho` and reference dApps.
- **Two plugin transports** add complexity. Acceptable trade-off for ecosystem reach.

### 13.3 Neutral

- Forces decisions about project layout that some teams may prefer to make themselves. Solis sticks to Anchor's `multiple` template, which is community-recommended, but is opinionated about `clients/`, `app/`, `tests/` placement.
- Codama becomes a hard runtime dependency. The Solana ecosystem is already moving this way.
- Surfpool's adoption is implicitly amplified by solis defaulting to it.

---

## 14. References

### 14.1 Primary Sources

- Ignite CLI: <https://github.com/ignite/cli>, <https://docs.ignite.com/>
- Ignite Apps (plugin system): <https://github.com/ignite/apps>
- Anchor 1.0 release notes: <https://www.anchor-lang.com/docs/updates/release-notes/1-0-0> (and 0.29 through 0.32)
- Anchor `--template multiple` PR: <https://github.com/solana-foundation/anchor/pull/2602>
- Codama: <https://github.com/codama-idl/codama>
- Codama renderers organization: <https://github.com/codama-idl>
- Surfpool: <https://www.surfpool.run>, <https://docs.surfpool.run>, <https://lib.rs/crates/surfpool-core>
- LiteSVM: <https://www.anchor-lang.com/docs/testing/litesvm>
- Pinocchio: <https://www.helius.dev/blog/pinocchio>, <https://learn.blueshift.gg/en/courses/pinocchio-for-dummies/pinocchio-101>
- `create-solana-program`: <https://github.com/solana-program/create-solana-program>
- `create-solana-dapp`: <https://github.com/solana-developers/create-solana-dapp>
- Mucho CLI: <https://github.com/solana-foundation/mucho>
- Solana Kit: <https://solana.com/docs/clients/javascript>

### 14.2 Market Research Sources

- "Deep Dive of the State of Developer Tooling on Solana" — superteam.fun, Aug 2025
- "Inside Solana's Developer Toolbox: A 2025 Deep Dive" — Medium, Jul 2025
- "Solana Developer Stack" — Substack (viveknakrani), 2025
- "How to Build Solana Programs with Pinocchio" — Helius, Jun 2025
- "How to Test Solana Programs with LiteSVM" — Quicknode, Nov 2025
- "Solana 2026 Outlook" — Blockdaemon, Feb 2026
- DeepWiki "Solana 2026 Task" (dylean) — framework comparison, Mar 2026

### 14.3 Tooling Libraries (Go side)

- cobra: <https://github.com/spf13/cobra>
- viper: <https://github.com/spf13/viper>
- bubble tea: <https://github.com/charmbracelet/bubbletea>
- lipgloss: <https://github.com/charmbracelet/lipgloss>
- fsnotify: <https://github.com/fsnotify/fsnotify>
- hashicorp/go-plugin: <https://github.com/hashicorp/go-plugin>
- sprig: <https://github.com/Masterminds/sprig>
- goldie: <https://github.com/sebdah/goldie>
- testify: <https://github.com/stretchr/testify>

### 14.4 AST/Rust Manipulation

- `ast-grep`: <https://github.com/ast-grep/ast-grep>
- `syn`/`quote` (reference only, not used directly): <https://github.com/dtolnay/syn>

---

## 15. Appendices

### Appendix A — Comparison Matrix (Detailed)

| Feature | `anchor init` | `create-solana-program` | `create-solana-dapp` | `mucho` | `solis` (this proposal) |
|---|:-:|:-:|:-:|:-:|:-:|
| Workspace scaffold | ✅ | ✅ | ✅ | ❌ | ✅ |
| Anchor program scaffold | ✅ | ✅ | ✅ | ❌ | ✅ |
| Multi-language clients | ❌ | ✅ (via Codama) | ❌ | ❌ | ✅ |
| Frontend scaffold | ❌ | ❌ | ✅ | ❌ | ✅ |
| Incremental `scaffold instruction` | ❌ | ❌ | ❌ | ❌ | ✅ |
| Incremental `scaffold account` | ❌ | ❌ | ❌ | ❌ | ✅ |
| CRUD recipe | ❌ | ❌ | ❌ | ❌ | ✅ |
| Frontend hook generation | ❌ | ❌ | ❌ | ❌ | ✅ |
| Local validator | ❌ | ❌ | ❌ | ✅ (wraps `solana-test-validator`) | ✅ (Surfpool primary) |
| Watch + auto-rebuild | ❌ | ❌ | ❌ | ❌ | ✅ |
| Plugin system | ❌ | ❌ | ❌ | ❌ | ✅ |
| Indexer scaffold | ❌ | ❌ | ❌ | ❌ | ✅ (via plugin) |
| Verifiable builds | ❌ | ❌ | ❌ | ✅ (delegates to `solana-verify`) | ✅ (integrates `solana-verify`) |
| Single binary distribution | ❌ (cargo install) | ❌ (npm) | ❌ (npm) | ❌ (npm) | ✅ |
| TUI dashboard | ❌ | ❌ | ❌ | ❌ | ✅ |

### Appendix B — Sample CLI Session

```bash
# Day 1: bootstrap a new protocol
$ solis chain new lending-protocol --frontend next --clients js,rust
✓ workspace created at ./lending-protocol
✓ Anchor 1.0.2 + Solana 2.1.0 + Rust 1.79.0
✓ frontend: Next.js 14 (app router) + Solana Kit + wallet-adapter
✓ pnpm install (12.3s)
✓ anchor build (8.7s)
✓ codama run (0.4s)

Next:
  cd lending-protocol
  solis chain serve

$ cd lending-protocol

# Day 1, an hour later: add a lending market
$ solis scaffold program market
✓ programs/market created
✓ updated Cargo.toml workspace members
✓ added market to solis.yml

$ solis scaffold account Market \
    --program market \
    --fields "authority:Pubkey,asset:Pubkey,total_deposits:u64,utilization_rate:u16,bump:u8" \
    --pda "market,asset"
✓ programs/market/src/state/market.rs (created)
✓ programs/market/src/state/mod.rs (patched: +market)

$ solis scaffold instruction initialize_market \
    --program market \
    --args "utilization_rate:u16" \
    --accounts "market:init:seeds=market,asset.key().as_ref():space=8+32+32+8+2+1:payer=authority,asset:mint,authority:signer:mut,system_program"
✓ programs/market/src/instructions/initialize_market.rs (created)
✓ programs/market/src/instructions/mod.rs (patched)
✓ programs/market/src/lib.rs (patched: dispatch)
✓ tests/market/initialize_market.test.ts (created)

$ solis scaffold instruction deposit \
    --program market \
    --args "amount:u64" \
    --accounts "market:mut:seeds=market,asset.key().as_ref(),asset:mint,user_ata:token:mut,vault:token:mut,user:signer:mut,token_program" \
    --emit "Deposited"
✓ programs/market/src/instructions/deposit.rs (created)
✓ programs/market/src/instructions/mod.rs (patched)
✓ programs/market/src/lib.rs (patched: dispatch)
✓ programs/market/src/events.rs (patched: +Deposited)
✓ tests/market/deposit.test.ts (created)

$ solis chain serve
[TUI launches]
[surfpool fork-mainnet starts on :8899, JIT account fetch]
[anchor build watches programs/**/*.rs]
[codama run re-renders clients/{js,rust}/src/generated on IDL change]
[next dev runs on :3000 with HMR]

# Edit programs/market/src/instructions/deposit.rs handler body...
# Save → 2.1s later: anchor build ✓ → codama run ✓ → frontend HMR refresh ✓
# Phantom-connected localhost frontend can call deposit() against forked mainnet SPL Token

# Day 2: deploy to devnet
^C  # stop serve
$ solis chain deploy --cluster devnet --verify
✓ idl diff: compatible (1 new instruction, 0 breaking)
✓ verifiable build (solana-verify)
✓ deployed: Market11111... at slot 264512098
✓ IDL uploaded on-chain
✓ Codama IDL pushed to clients/js (package version bumped)
```

### Appendix C — Glossary

| Term | Definition |
|---|---|
| **Marker / Marker region** | Structured comment pair (`// === solis:auto-generated:begin … ===` / `… :end …`) delimiting solis-managed code. |
| **User region** | Marker pair (`// === solis:user-region:begin … ===` / `… :end …`) delimiting human-owned code inside generated files. |
| **Scaffolder** | A type implementing the `Scaffolder` interface; one per generation noun. |
| **Recipe** | A composite scaffolder that internally calls multiple primitive scaffolders (e.g. CRUD). |
| **FileSet** | The unit of filesystem change inside a transaction; collection of `FileOp`s. |
| **Plan / Validate / Commit** | The three phases of the filesystem transaction lifecycle (§ 7.7). |
| **Runtime** | A local Solana VM implementation (Surfpool, test-validator). |
| **Fixture** | Pre-loaded program or account in the local runtime, sourced from mainnet/devnet. |
| **IDL** | Anchor's Interface Definition Language (JSON), generated by `anchor build`. |
| **Codama IDL** | Codama's normalized IDL format (superset of Anchor IDL). |
| **Renderer** | A Codama visitor that produces a client in a specific language. |
| **JIT mainnet fork** | Surfpool's strategy of fetching mainnet accounts on demand during transaction execution. |
| **Pinocchio** | Zero-dependency `no_std` Solana program library. Alternative to Anchor for perf-critical work. |

---

*End of ADR-0001.*
