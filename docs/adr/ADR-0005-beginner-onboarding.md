# ADR-0005 — Beginner Onboarding Surface for `sunscreen`

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-06-01 |
| **Authors** | Pantani |
| **Tags** | onboarding, ux, beginner, wizard, dx |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | ADR-0001 § 1.4 and § 10.7 (sunscreen CLI), ADR-0002 (CLI Design Conventions), ADR-0004 (Incremental Scaffolding), `IMPLEMENTATION-KICKOFF.md` |

---

## Variation Log

| Date | Author | Version | Summary |
|------|--------|---------|---------|
| 2026-06-01 | Pantani | 1.0.0 | Initial ADR — formalizes Phase 5.5 (Onboarding Layer) |

---

## TL;DR

`sunscreen` adds a **dedicated onboarding layer** (Phase 5.5) composed of six top-level commands — `init`, `examples`, `quickstart`, `wallet`, `deploy`, `learn` — and a formal contract for **actionable errors** with a `next_step` field. This layer is a thin interactive wrapper over the already-existing core (`chain new`, scaffolders, doctor): `init` is a `dialoguer` wizard that ends by calling the same loader/validator as `chain new`; `quickstart <recipe>` composes `chain new` + scaffolders + frontend bootstrap into a single one-shot command; `wallet` and `deploy` are friendly wrappers over `solana-keygen`/`solana airdrop`/`anchor deploy`; `examples` ships ready-made projects via `rust-embed`; `learn` renders embedded markdown tutorials via `termimad`. All commands respect DD2 (power-user non-blocking): TTY detection disables prompts and `--non-interactive` forces flag-based equivalence. DoD: a user with no Solana account runs `sunscreen init` → `sunscreen quickstart nft` → sees an NFT minted on devnet in **< 10 min**.

---

## 1. Context

### 1.1 Problem framing

The original plan in ADR-0001 (Phases 0–8) assumes an **intermediate Solana dev** — someone who already understands `Pubkey`, knows the difference between `Account` and `AccountInfo`, knows the `anchor build → anchor deploy` cycle, and is comfortable editing `Anchor.toml`. Phases 0–2 R3 (already delivered) reflect that audience: `chain new` requires explicit flags (`--framework anchor --frontend next --clients ts,rs`), scaffolders expect the user to know what an "instruction" is, and `doctor` reports toolchain status in jargon (`anchor-cli 0.30.x`, `solana-cli 2.0.x`).

The **product vision**, however, is more ambitious: `sunscreen` should be **the entry point for devs who don't know Rust or Solana deeply** — a TypeScript developer curious about NFTs, a student who has heard of SPL tokens, an indie who wants to prototype a DAO in an afternoon. This audience:

- Doesn't know they need a keypair before doing an `airdrop`.
- Doesn't know the difference between `localnet`, `devnet`, `testnet`, `mainnet`.
- Doesn't know what a PDA is, and therefore doesn't know *why* `scaffold account` asks about seeds.
- Will abandon the CLI in the first 5 minutes if the first screen is `error: missing required argument '--framework <FRAMEWORK>'`.

This ADR formalizes a **surface layer** that does not exist in ADR-0001 and that will be prioritized between Phase 5 (recipes) and v1.0 (release).

### 1.2 Constraints

- **Don't break the expert.** The entire Phase 0–5 surface must continue to work byte-for-byte. Onboarding is *additive*.
- **Offline-first.** Embedded examples and tutorials (no mandatory `git clone` on first use).
- **No parallel path.** The `init` wizard **cannot** duplicate the loader/validator of `chain new`; it must end by invoking the same code.
- **TTY-aware.** Detect `isatty(stdin)` and degrade to flag-based when running under a pipe/CI.
- **Zero cost by default.** No command should spend SOL without explicit confirmation (mainnet especially).
- **i18n-ready but en-US first.** Strings centralized in `src/strings/en_US.rs`; PT-BR remains a future skill.

---

## 2. Decision Drivers

- **DD1 — Learning curve.** Time to "hello world → NFT deployed on devnet" < 10 min for an absolute beginner with no prior account.
- **DD2 — Power-user non-blocking.** Every wizard has a flag-based equivalent; `--non-interactive` or TTY detection disables prompts; no new command appears in `chain new` or the existing scaffolders.
- **DD3 — No mandatory network.** Examples and `learn` embedded via `rust-embed`; cluster ops (`wallet airdrop`, `deploy devnet`) are opt-in.
- **DD4 — Reuse of existing infrastructure.** The wizard ends by calling the same workspace-construction entry point as `chain new` (today `Config::new_for_workspace(...)` in `src/cli/chain.rs`); Phase 5.5 may extract a shared `ChainNewArgs` struct so the wizard and the flag parser feed it identically; `quickstart` composes scaffolders via their Rust API, not via shellout.
- **DD5 — i18n-ready, en-US first.** Strings in a dedicated module; no literals in the control flow.
- **DD6 — Actionable errors.** Every variant of `SunscreenError` carries `next_step: Option<String>`; 100% coverage verified in CI.
- **DD7 — Discoverability.** Top-level commands (not flags), names oriented to the user's task (`wallet new`, not `keypair generate`).

---

## 3. Considered Options

| # | Option | Summary |
|---|---|---|
| (A) | **Separate layer** *(chosen)* | New top-level commands (`init`, `examples`, `quickstart`, `wallet`, `deploy`, `learn`) + delegation to the core |
| (B) | `--interactive` flags on existing commands | `chain new --interactive`, `scaffold instruction --interactive` |
| (C) | Separate sub-CLI | Ship `sunscreen-easy` as a parallel binary |
| (D) | Optional external plugin | Onboarding via `sunscreen plugin install onboarding` |

### 3.1 Option (A) — Separate layer

**Pros:**
- Keeps ADR-0001 intact: no new flag on `chain new` or scaffolders.
- Explicit and discoverable surface: `sunscreen --help` lists the friendly commands alongside the expert ones.
- Easy to evolve: each command is a `clap` subcommand with its own module.
- Clean composition: `quickstart nft` is literally `chain_new(...) + scaffold_instruction(...) + scaffold_account(...)` in sequence.

**Cons:**
- Increases the number of top-level commands (from ~8 to ~14). Mitigation: `Beginner` group in `--help` (cf. ADR-0002 § 3.2).
- Risk of divergence between what the wizard asks and what `chain new` accepts. Mitigation: single shared validator.

### 3.2 Option (B) — `--interactive` on existing commands

**Pros:** zero new commands; discovery via `--help` of the existing command.

**Cons:**
- Pollutes the surface: `chain new --interactive --framework anchor` is semantically confusing (why ask for a framework if it's interactive?).
- Does not cover the new cases (`wallet`, `deploy`, `learn`, `examples`) that have no current equivalent.
- Makes the beginner's "happy path" harder: they have to guess that `chain new` is the entry point.

**Rejected.**

### 3.3 Option (C) — Separate sub-CLI (`sunscreen-easy`)

**Pros:** completely isolates the beginner UX from the expert one.

**Cons:** fragments the brand; duplicates config loading; the user has to learn *when* to switch binaries; install doubles.

**Rejected.**

### 3.4 Option (D) — Optional external plugin

**Pros:** keeps the core lean; allows fast iteration without a core release.

**Cons:** onboarding has to be **default**, not opt-in — the person who most needs the plugin is precisely the one who won't discover they need to install it.

**Rejected.**

---

## 4. Decision

`sunscreen` adopts **Option (A) — Separate layer** with six new top-level commands and a formal contract for actionable errors.

### 4.1 New commands

| Signature | Flags | Output | Exit codes |
|-----------|-------|--------|-----------|
| `sunscreen init [name]` | `--non-interactive`, `--from-preset <name>`, `--json` | Creates workspace; emits a summary of choices + next step | 0 ok; 4 user_input (prompt aborted); 7 path_conflict (path exists) |
| `sunscreen examples list` | `--json`, `--tag <tag>` | Table: name, short description, tags, estimated time | 0 ok |
| `sunscreen examples describe <name>` | `--json` | Example README rendered via `termimad` | 0 ok; 4 user_input (unknown name) |
| `sunscreen examples use <name> [path]` | `--non-interactive`, `--json` | Copies embedded example to `path` (default: `./<name>`) | 0 ok; 4 user_input (unknown name); 7 path_conflict |
| `sunscreen quickstart <recipe>` | `--name <n>`, `--cluster <localnet\|devnet>`, `--non-interactive`, `--json` | Composes `chain new` + scaffolds + frontend bootstrap; opens `localhost:3000` if TTY | 0 ok; 2 toolchain; 4 user_input; 7 path_conflict |
| `sunscreen wallet new [name]` | `--out <path>`, `--no-bip39-passphrase`, `--json` | Creates keypair; reports pubkey + path | 0 ok; 7 path_conflict |
| `sunscreen wallet list` | `--json` | Lists known keypairs + which is default | 0 ok |
| `sunscreen wallet airdrop [amount]` | `--cluster <c>`, `--to <pubkey>`, `--json` | Requests airdrop; reports final balance | 0 ok; 8 network (includes rate-limited responses) |
| `sunscreen wallet balance` | `--cluster <c>`, `--json` | Balance of the default keypair | 0 ok; 8 network |
| `sunscreen wallet set-default <name>` | `--json` | Updates `sunscreen.yml` | 0 ok; 4 user_input (unknown name) |
| `sunscreen deploy <target>` | `--program <name>`, `--verify`, `--yes-i-understand-cost` (mainnet), `--json` | Wraps `anchor deploy`; shows estimated cost beforehand (mainnet) | 0 ok; 2 toolchain; 4 user_input; 8 network |
| `sunscreen learn` | — | Lists available topics | 0 ok |
| `sunscreen learn <topic>` | `--json` (emits frontmatter) | Renders the markdown tutorial via `termimad` | 0 ok; 4 user_input (unknown topic) |

`<recipe>` ∈ `{token, nft, dao, blog}` (extensible in a future ADR).
`<target>` ∈ `{localnet, devnet, mainnet}`.
`<topic>` MVP ∈ `{pda, cpi, token-2022, accounts-model, anchor-vs-native}`.

> **Exit code compatibility.** The canonical source of truth is `src/error.rs::SunscreenError::exit_code`, which already assigns `1`=Other, `2`=ToolchainMissing, `3`=ConfigInvalid, `4`=UserInput, `5`=WorkspaceMissing, `6`=InstructionDrift. ADR-0002 § 4.3 documents `1`–`4` but still describes `5`/`6` as "reserved" (loosely flagged for future network/conflict use); that section is **out of date** relative to the implementation and must be amended as part of Phase 5.5 to reflect the current `5`/`6` assignments and the additions below. Onboarding does **not** repurpose any existing code. It extends the canonical mapping with two new variants:
>
> - `7` — `PathConflict` (target directory or file already exists; raised by `init`, `examples use`, `wallet new`, `quickstart`).
> - `8` — `Network` (RPC, faucet, or airdrop failure — including rate-limited responses; raised by `wallet airdrop`, `wallet balance`, `deploy`).
>
> Phase 5.5 implementation MUST add `SunscreenError::PathConflict` and `SunscreenError::Network` variants, extend `exit_code()` to return `7` and `8`, and ship the ADR-0002 § 4.3 amendment that promotes `5`/`6`/`7`/`8` from "reserved" or undocumented to "assigned". Codes `5` and `6` MUST NOT be reused for any new meaning — they are already taken by `WorkspaceMissing`/`InstructionDrift`.

### 4.2 Actionable error contract

`SunscreenError` is and remains the existing **enum** in `src/error.rs` (do **not** redefine it as a struct). Phase 5.5 extends it with an optional `next_step` field carried alongside each variant — implemented either as an associated value per variant or via a parallel `fn next_step(&self) -> Option<&str>` method that pattern-matches on `self`. The chosen shape is left to the implementation PR; the contract below is what matters externally.

The JSON serialization extends the canonical schema documented in ADR-0002 § 4.4 (`{"error": "...", "kind": "..."}`, written to stderr by `src/cli/root.rs::execute`) by appending two optional fields. Existing field names are preserved verbatim — no renames, no removals — so parsers built against ADR-0002 keep working:

```json
{"error":"no default wallet configured","kind":"user_input","next_step":"sunscreen wallet new --out ~/.config/solana/id.json","exit_code":4}
```

- New optional fields: `next_step` (string, suggested remediation command) and `exit_code` (integer, mirrors the process exit code for callers that capture stderr without inspecting `$?`).
- CI test (`tests/errors_contract.rs`) covers every `SunscreenError` variant and asserts that each one returns `Some(next_step)`. Because `SunscreenError` has data-carrying variants (`ConfigInvalid(String)`, `InstructionDrift { .. }`, etc.), `strum::EnumIter` cannot be derived directly; the test uses an explicit hand-maintained list of constructor calls (one representative call per variant) or a parallel fieldless `SunscreenErrorKind` discriminant enum that `does` derive `EnumIter`. A compile-time exhaustiveness check (`match self { SunscreenError::Foo(..) => .. }` with no `_` arm) prevents the list from drifting out of sync. Variants with no meaningful remediation MAY return `Some("")` and the test SHALL allow that explicitly, with a code-comment justification per variant.
- TTY rendering: extra line `→ try: <next_step>` in cyan, emitted only when stderr is a TTY and the value is non-empty.

### 4.3 Asset distribution

- **Examples**: embedded via `rust-embed` in `assets/examples/<name>/**`. Target binary size: < 15 MB total. Large examples (>2 MB) marked with `remote=true` in the manifest; `examples use <name>` downloads via `gix` (pure Rust, no `git` CLI dependency).
- **Learn**: 100% embedded in `assets/learn/<topic>.md`. YAML frontmatter with `title`, `est_minutes`, `prereqs`.
- **Recipes** (`quickstart`): defined in code (`src/onboarding/recipes/<name>.rs`) — they are not templates, they are *programs* that orchestrate core calls.

### 4.4 TTY detection & --non-interactive

- Single helper `src/onboarding/tty.rs::is_interactive() -> bool` checks `IsTerminal::is_terminal(&io::stdin())` AND absence of `--non-interactive` AND absence of `SUNSCREEN_NON_INTERACTIVE=1`.
- Wizard prompts replaced by an `SunscreenError::UserInput` error with `next_step` listing the equivalent flag when `is_interactive() == false`.

---

## 5. Consequences

### 5.1 Positive

- Democratizes the CLI: a beginner reaches their first minted NFT in < 10 min.
- Boosts adoption and reduces friction in demos/workshops.
- Reduces support load: errors with `next_step` avoid half the issues opened today against similar CLIs.
- Reuses 100% of the existing core — the wizard is a thin layer.
- `learn` creates a searchable documentation reservoir inside the binary.

### 5.2 Negative

- **+2 sprints** of work (Block E of the roadmap; see § 6).
- **+5–8 MB on the binary** for the examples + learn embed. Mitigation: introduce a Cargo feature-gating strategy (no `[features]` section exists in `Cargo.toml` today) — Phase 5.5 will add an `onboarding` feature enabled by default and a `--no-default-features` build path for CI/production where the embedded assets are not wanted.
- Increases test surface: each wizard needs an interactive test (via `expectrl` or similar) + a `--non-interactive` test.
- Risk of divergence between wizard and flags. Mitigation: single validator; property-based test that randomizes wizard inputs and compares with the equivalent `chain new`.
- More top-level commands in `--help`. Mitigation: group via `clap` `help_heading`.

### 5.3 Neutral

- Requires 3 new deps: `dialoguer ^0.11`, `termimad ^0.31`, `indicatif ^0.17` — all already planned for the Phase 3 TUI (Runtime Orchestration, `chain serve`).
- Strings centralized in `src/strings/` enable future i18n with no additional refactor.

### 5.4 Risk mitigations

- Golden tests record complete transcripts of each wizard (via `insta` + `expectrl`).
- Property test: for every possible combination of `init` answers, verify that the resulting workspace is byte-identical to the one produced by `chain new` with equivalent flags.
- `quickstart` has an E2E test on `localnet` in CI (Surfpool starts in the background; teardown via `Drop`).

---

## 6. Implementation Plan (Phase 5.5)

Inserted between Phase 5 (recipes) and Phase 8 (release). 2 sprints (~4 weeks).

| Sprint | Deliverable | Tests |
|---|---|---|
| **S1** | `init` (wizard + validator share), `wallet *`, `next_step` contract in 100% of variants | unit + golden transcripts |
| **S2** | `examples` (list/describe/use), `quickstart {token, nft, dao, blog}`, `deploy`, `learn` (5 MVP topics) | E2E on localnet; embed integrity test |

### 6.1 Expected components

```text
src/
├── onboarding/
│   ├── mod.rs
│   ├── tty.rs              # is_interactive()
│   ├── wizard.rs           # init flow
│   ├── recipes/
│   │   ├── token.rs        # SPL fungible
│   │   ├── nft.rs          # Metaplex Token Metadata + Master Edition
│   │   ├── dao.rs          # voting program
│   │   └── blog.rs         # CRUD with PDAs
│   ├── wallet.rs           # solana-keygen wrapper
│   ├── deploy.rs           # anchor deploy wrapper + cost preview
│   ├── examples.rs         # rust-embed gallery
│   └── learn.rs            # termimad renderer
├── strings/
│   └── en_US.rs            # every user-facing string
└── error.rs                # next_step field
assets/
├── examples/
│   ├── token-faucet/
│   ├── nft-collection/
│   ├── escrow/
│   ├── voting-dao/
│   └── blog-crud/
└── learn/
    ├── pda.md
    ├── cpi.md
    ├── token-2022.md
    ├── accounts-model.md
    └── anchor-vs-native.md
```

---

## 7. UX Examples

### 7.1 `sunscreen init` (transcript)

```text
$ sunscreen init
✻ Welcome to sunscreen — let's build a Solana app.

? Project name › my-app
? What are you building?
  ❯ A token (SPL fungible)
    An NFT collection (Metaplex)
    A DAO / voting program
    A blog / CRUD app
    Something else (blank workspace)
? Frontend framework?
  ❯ Next.js (recommended)
    Vite + React
    None (CLI only)
? Generate client SDKs?
  ❯ TypeScript + Rust
    TypeScript only
    None
? Cluster for development? › devnet

✓ Workspace created at ./my-app
✓ Codama IDL bootstrapped
✓ Frontend scaffolded (Next.js)

→ next: cd my-app && sunscreen quickstart nft
```

### 7.2 `sunscreen quickstart nft` (output)

```text
$ sunscreen quickstart nft --name pixel-cats
[1/6] chain new pixel-cats --framework anchor --frontend next --clients ts,rs   ✓ (1.2s)
[2/6] scaffold account collection --seeds "collection,authority"                 ✓ (0.4s)
[3/6] scaffold instruction mint_nft --accounts collection,mint,metadata         ✓ (0.6s)
[4/6] scaffold instruction update_metadata                                       ✓ (0.3s)
[5/6] wallet airdrop 2 --cluster devnet                                          ✓ (3.1s)
[6/6] anchor build && anchor deploy --provider.cluster devnet                    ✓ (47s)

✓ Program deployed: 7xKXt...mJqP
✓ Frontend running at http://localhost:3000
✓ Mint your first NFT: http://localhost:3000/mint

→ next: open http://localhost:3000/mint in your browser
```

### 7.3 Actionable error with `next_step`

```text
$ sunscreen deploy devnet
error: no default wallet configured
  → try: sunscreen wallet new --out ~/.config/solana/id.json
```

JSON equivalent (extends the canonical schema from ADR-0002 § 4.4 — keeps `error` and `kind` verbatim, adds `next_step` and `exit_code`):

```json
{
  "error": "no default wallet configured",
  "kind": "user_input",
  "next_step": "sunscreen wallet new --out ~/.config/solana/id.json",
  "exit_code": 4
}
```

---

## 8. Open Questions

1. **Examples gallery: embed vs git clone on-demand?**
   - Inclination: **embed by default** (offline-first, DD3); `remote=true` flag in the manifest for large examples (> 2 MB) that are downloaded via `gix`.
2. **Wizard in PT-BR for MVP or en-US only?**
   - Inclination: **en-US first** (Solana is global); strings centralized in `src/strings/en_US.rs` to enable PT-BR via a future skill with no refactor.
3. **Does `sunscreen deploy mainnet` require `--yes-i-understand-cost` or just interactive confirmation?**
   - Inclination: **both** — interactive confirmation when TTY, mandatory flag when `--non-interactive` (covers CI accidents).
4. **`sunscreen learn` content managed in-repo or in a separate versioned repo?**
   - Inclination: **in-repo** for MVP (5 topics); migrate to a `sunscreen-learn` repo + `learn update` once it exceeds ~20 topics.
5. **Should `quickstart` open the browser automatically?**
   - Inclination: yes when TTY (via the `open` crate); silent when `--non-interactive`.
6. **Wallet storage location.**
   - Reuse `~/.config/solana/id.json` (compat with `solana-cli`) or use `~/.config/sunscreen/wallets/`? Inclination: reuse the canonical Solana path for interop.

---

## 9. Acceptance Criteria

- [ ] Six new commands implemented per the table in § 4.1 with `--json` and `--non-interactive` where applicable.
- [ ] `next_step` contract covers 100% of `SunscreenError` variants (verified by a CI test).
- [ ] `init` wizard produces a workspace **byte-identical** to `chain new` with equivalent flags (property test).
- [ ] All four `quickstart` recipes defined in § 4.1 (`token`, `nft`, `dao`, `blog`) execute on localnet in CI.
- [ ] `sunscreen examples list` returns ≥ 5 embedded entries; `examples use <name>` creates a usable project.
- [ ] `sunscreen learn` renders ≥ 5 MVP topics with no `termimad` warnings.
- [ ] Human DoD: a user with no Solana account runs `sunscreen init` → `sunscreen quickstart nft` → sees an NFT minted on devnet in **< 10 min** (measured in an internal workshop).
- [ ] Binary size with onboarding: < 25 MB (release, stripped); without onboarding (`--no-default-features`): < 12 MB.

---

## 10. References

- ADR-0001 § 1.4 (Target personas) and § 10.7 (Recipes & onboarding gaps)
- ADR-0002 (CLI Design Conventions — `--json`, exit codes, help grouping)
- ADR-0004 (Incremental Scaffolding — core reuse by the recipes)
- `IMPLEMENTATION-KICKOFF.md` (roadmap; Phase 5.5 to be inserted)
- Ignite CLI `scaffold chain` wizard (UX reference)
- `dialoguer` 0.11, `termimad` 0.31, `indicatif` 0.17, `rust-embed` 8.x, `gix` 0.66
