# sunscreen — Roadmap

**Status:** Live tracker
**Last updated:** 2026-06-01
**Supersedes (as the live source of truth):** the roadmap section of [`docs/adr/ADR-0001-solis-cli.md`](docs/adr/ADR-0001-solis-cli.md) §10 and the week-by-week checklist in [`IMPLEMENTATION-KICKOFF.md`](IMPLEMENTATION-KICKOFF.md). Those documents remain as historical context for the original Go-based design; this file is what changes as work lands.

## Legend

| Symbol | Meaning |
|---|---|
| ✅ | Done — merged, tested, in `main` |
| 🚧 | In progress |
| ⏳ | Next up — concrete, scheduled, unblocked |
| 📋 | Planned — scoped but not yet started |
| 🔮 | Post-v1.0 — explicitly deferred |

---

> **Stack migration note.** ADR-0001 and IMPLEMENTATION-KICKOFF were drafted assuming a Go stack (`cobra`, `viper`, `goreleaser`, `sprig`, `bubble tea`, `lipgloss`, `goldie`, `go-plugin`). The project is implemented in **Rust**. Map mentally:
>
> | Original (Go) | Adopted (Rust) |
> |---|---|
> | cobra | clap (derive) |
> | viper + JSON Schema | serde + jsonschema |
> | goreleaser | cargo-dist |
> | sprig + text/template | minijinja |
> | go:embed | rust-embed |
> | bubble tea / lipgloss | ratatui / crossterm |
> | goldie | insta |
> | hashicorp/go-plugin | tonic (gRPC) + stdio |
> | fsnotify | notify |
>
> `IMPLEMENTATION-KICKOFF.md` still contains Go-flavoured snippets and is retained for historical reference only; do not treat it as actionable. Strategic decisions from ADR-0001 (Anchor IDL as source of truth, marker-based incremental editing, plugin protocol, scaffolder taxonomy) are preserved as-is.

---

## 1. Overall Timeline

Total to **v1.0**: ~21 weeks of focused work (vs. 16 weeks in the original ADR-0001 plan). The increase absorbs the +4-week Phase 5.5 (Onboarding) introduced by ADR-0005. Phase 6 (plugins) and Phase 7 (Pinocchio) are explicitly deferred post-v1.0 to hold the v1.0 line.

| Phase | Theme | Status | Duration | Cum. | Key deliverables | DoD source |
|---|---|---|---:|---:|---|---|
| **0** | Foundations | ✅ DONE | 2 wk | 2 wk | CLI skeleton, config v1, toolchain detect, doctor, CI, golden infra | ADR-0001 §10.2 |
| **1** | Workspace Bootstrap | ✅ DONE | 2 wk | 4 wk | `chain new` produces compilable Anchor workspace + frontend variants | ADR-0001 §10.3 |
| **2** | Incremental Scaffolding | 🚧 R5 polish in progress | 4 wk | 8 wk | `scaffold {instruction, account, event, error, program}` + `chain doctor --fix-markers` | ADR-0001 §10.4, ADR-0004 |
| **3** | Runtime Orchestration | 🚧 initial build slice | 3 wk | 11 wk | `chain serve` (Surfpool + watcher + codama + ratatui TUI), `chain build` | ADR-0001 §10.5 |
| **4** | Codegen & Frontend Hooks | 📋 | 2 wk | 13 wk | `generate {clients, idl, frontend-hooks}`, codama wrapper | ADR-0001 §10.6 |
| **5** | Recipes | 📋 | 3 wk | 16 wk | `scaffold {crud, spl-token, metaplex-nft}` | ADR-0001 §10.7 |
| **5.5** | Onboarding Layer | 📋 NEW | 4 wk | 20 wk | `init`, `quickstart`, `examples`, `wallet`, `deploy`, `learn`, `next_step` errors | ADR-0005 §6 |
| **6** | Plugin System | 🔮 post-v1.0 | 4 wk | — | gRPC + stdio plugins, 2 reference plugins | ADR-0001 §10.8 |
| **7** | Pinocchio support | 🔮 post-v1.0 | 3 wk | — | `--framework pinocchio` MVP | ADR-0001 §10.9 |
| **8** | Distribution & Docs (v1.0) | 📋 | 1 wk | 21 wk | cargo-dist multi-OS, mdBook/Starlight docs, shell completions | ADR-0001 §10 |

> Phases are listed in ascending numeric order. Phase 8 is what cuts v1.0 (~21 wk cumulative); Phases 6 and 7 are explicitly deferred post-v1.0 and do **not** gate the v1.0 release — they are listed above Phase 8 only to keep the numeric sequence readable.

---

## 2. Per-Phase Detail

### Phase 0 — Foundations ✅

**Status.** ✅ Complete (Weeks 1 + 2). 22/22 tests, cold-start 3.18 ms (target was <50 ms).

**Goal.** Repo skeleton, build/test infrastructure, CI green, no scaffolding logic yet.

**Deliverables (Week 1).**
- [x] `src/cli/` clap-based root + persistent flags (`--verbose`, `--workdir`, `--config`)
- [x] `sunscreen version` with build-injected semver
- [x] `sunscreen doctor` — detects `anchor`, `solana`, `cargo`, `rustc`, `pnpm`, `node`, `surfpool`, `codama`
- [x] `src/config/` — `sunscreen.yml` v1 parser, JSON Schema embedded & validated
- [x] `src/toolchain/` — version detection with cached results
- [x] `src/templates/` — rust-embed + minijinja render pipeline
- [x] `src/error.rs` — typed error taxonomy
- [x] CI workflow (`.github/workflows/ci.yml`) — lint + unit on push/PR

**Deliverables (Week 2).**
- [x] Config round-trip: load → validate → serialize → load, zero drift
- [x] Migration framework (`v0 → v1`)
- [x] Cold-start benchmark in `benches/` (criterion) — measured 3.18 ms
- [x] [`docs/adr/ADR-0002-cli-design-conventions.md`](docs/adr/ADR-0002-cli-design-conventions.md)
- [x] [`docs/adr/ADR-0003-documentation-strategy.md`](docs/adr/ADR-0003-documentation-strategy.md)

**DoD.** All checks from ADR-0001 §10.2 met; baseline `sunscreen --help` measurably under budget.

---

### Phase 1 — Workspace Bootstrap ✅

**Status.** ✅ Complete. 59/59 tests; `chain new` E2E functional.

**Goal.** `sunscreen chain new <name>` produces a workspace where `anchor build` succeeds, with optional frontend.

**Deliverables.**
- [x] `src/fsutil/` — transactional filesystem ops (atomic rename, rollback on failure)
- [x] `src/cli/chain.rs` — `chain new` subcommand with `--framework`, `--frontend` flags
- [x] `templates/workspace/anchor-multiple/` — Anchor workspace skeleton
- [x] `templates/workspace/frontend-next/` — Next.js variant
- [x] `templates/workspace/frontend-vite/` — Vite variant
- [x] `templates/workspace/frontend-none/` — headless variant
- [x] Preflight checks (toolchain + path collisions) before writing any file
- [x] Config v1 expanded with workspace metadata

**DoD.** ADR-0001 §10.3 satisfied: `chain new foo --framework anchor` yields a workspace where `anchor build` succeeds; compile tests cover all three frontend variants.

---

### Phase 2 — Incremental Scaffolding 🚧

**Status.** 🚧 R5 polish in progress. R1–R4 are done; R5 has marker-hardening, offline compile coverage, and golden coverage landed. Real integration coverage is the remaining R5 gate. Strategy ratified in [`docs/adr/ADR-0004-incremental-scaffolding.md`](docs/adr/ADR-0004-incremental-scaffolding.md).

**Goal.** Idempotent, marker-driven scaffolders that surgically edit Rust source without disturbing user code.

#### R1 — rustpatch + `scaffold instruction` ✅

- [x] `src/rustpatch/marker.rs` (+ `mod.rs`) — scan/apply behaviour, line-ending preservation, and R5 `rustfmt` roundtrip coverage are covered
- [x] `src/workspace/` — workspace discovery and program enumeration
- [x] `src/cli/scaffold.rs` — `scaffold instruction <name>` subcommand
- [x] `src/templates/instruction.rs`
- [x] `templates/scaffold/instruction/` (`.j2`)
- [x] [`docs/reference/markers.md`](docs/reference/markers.md) — marker contract
- [x] Idempotency + drift detection; conflicting re-run → exit code 4 (`user_input`)
- [x] 96/96 tests; fmt + clippy clean

#### R2 — Dispatch carry-over ✅

- [x] `templates/workspace/anchor-multiple/programs/__program__/src/lib.rs.j2` ships with `segment=dispatch`
- [x] `templates/workspace/anchor-multiple/programs/__program__/src/instructions/mod.rs` ships with `segment=instructions`
- [x] First `scaffold instruction` on a brand-new workspace patches cleanly (`lib_rs_patched=true`, no warning)
- [x] `tests/scaffold_instruction.rs` + updated `tests/golden/snapshots/*`
- [x] 103/103 tests

#### R3 — `account` / `event` / `error` scaffolders ✅

- [x] `src/templates/{account, event, error}.rs`
- [x] `templates/scaffold/{account, event, error}/*` (`.j2`)
- [x] `tests/scaffold_{account, event, error}.rs`
- [x] Generator tag (`account` / `event` / `error`) emitted in every marker (D2 fix)
- [x] Account conflict → explicit error, no silent overwrite (D3 fix)
- [x] `--dry-run` and `--json` honoured on all three
- [x] 115/115 tests; fmt + clippy clean

#### R4 — `program` scaffolder + `chain doctor --fix-markers` ✅

Shipped via #5 (`67b0338`).

- [x] `src/cli/scaffold.rs::run_program` — `scaffold program <name>` (adds a new program crate to `programs/`, registers in `Anchor.toml` and root `Cargo.toml`)
- [x] `templates/scaffold/program/` template tree
- [x] `sunscreen chain doctor --fix-markers` (`src/cli/chain.rs::run_doctor`) — scans the workspace for marker corruption, appends missing marker pairs for **appendable** host files (state/mod.rs, instructions/mod.rs, etc.), reconstructs the non-appendable `dispatch` segment inside `#[program]` when instruction files provide enough information, and rewraps `error_variants` inside `#[error_code]` enums while preserving existing variants.
- [x] Auto-injection of `pub mod events;` / `pub mod errors;` / `pub mod state;` in `lib.rs` on first relevant scaffold (closes a R3 gap where users had to add the line manually)
- [x] `tests/scaffold_program.rs` (also covers `chain doctor --fix-markers` paths)
- [x] `tests/rustfmt_roundtrip.rs` — golden test that runs `rustfmt --edition=2021` over a fixture containing every documented marker segment and re-scans the result; matches the invariant promised in `docs/reference/markers.md` §5 and ADR-0004 §4
- [x] Reconstruction of the non-appendable `dispatch` site when both `begin` and body are gone — `chain doctor --fix-markers` now inserts a fresh `dispatch` marker block inside `#[program]` and rebuilds wrappers from `instructions/*.rs`
- [x] Safe recovery for the remaining non-appendable `error_variants` site — `chain doctor --fix-markers` rewraps existing enum contents instead of appending invalid Rust at EOF

#### R5 — Polish target 📋

- [x] Marker hardening: rustfmt roundtrip coverage + safe `dispatch` and `error_variants` repair paths
- [x] Offline compile-test harness started and expanded: `tests/compile_generated_workspace.rs` patches generated workspaces to local `anchor-lang` / `anchor-spl` shims and runs `cargo check --workspace --all-targets --offline` across generated workspaces. This fixed empty-account instruction structs, made generated program crates declare `anchor-spl`, restored ADR-style account syntax compatibility (`payer:signer:mut`, `system_program`, `token_program`), and made `emit!` compile checks type-check fielded event literals instead of only token-stringifying them.
- [x] ≥ 75 golden tests across all five scaffolders; current landed coverage is 75 snapshots, including the `tests/golden/render_scaffolders_matrix.rs` matrix for account/event/error/instruction/program renderers plus the existing workspace/program/instruction golden tests
- [x] ≥ 25 compile tests (`cargo check` of generated workspaces); current landed coverage is 25 offline compile scenarios covering `chain new` frontends, account/event/error/instruction, token/associated-token accounts, PDA/generic accounts, ADR-style account syntax, fielded event emits, multiple instructions, dry-run then real scaffolds, cased identifiers, multiple events/errors, idempotent rescaffolds, empty scaffold payloads, varied argument types, global JSON, custom program IDs, normalized project names, and single/multiple program workspaces
- [ ] 5 integration tests: full `anchor build` + IDL inspection + codama regen. Local execution is currently blocked by missing required tools (`anchor`, `solana`, `pnpm`); the Phase 3 `chain build --headless` entry point is now available for those integration tests once the toolchain exists.
- [ ] Phase 2 DoD per ADR-0001 §10.4 satisfied end-to-end

**Carry-over.** R5 real integration coverage → close Phase 2; Phase 3 has an initial `chain build` slice, but full runtime orchestration remains gated by the real integrations.

---

### Phase 3 — Runtime Orchestration 🚧

**Status.** 🚧 Initial build slice started. `chain build --headless` now reuses workspace discovery and runs `anchor build` + optional Codama regeneration through a testable runtime pipeline with parseable line-delimited JSON events. The watcher debounce core, notify-event-to-pipeline bridge, and `chain serve --headless` watcher loop are also in place. The rest of Phase 3 remains blocked on Phase 2 real integration closure.

**Goal.** `sunscreen chain serve` orchestrates Surfpool, file-watcher, codama regen, and a ratatui TUI in one supervised process tree.

**Deliverables.**
- [ ] `src/runtime/surfpool.rs` + `testvalidator.rs` implementing a `Runtime` trait
- [x] `src/runtime/watcher.rs` debounce core — batches relevant Rust/config changes after a quiet period, dedupes/sorts paths, and ignores generated/unrelated paths
- [x] `notify::Event` adapter — feeds raw notify paths into the debouncer and relativizes absolute paths against the workspace before pipeline filtering
- [x] `chain build --headless` initial runner — discovers the workspace, invokes `anchor build` at the workspace root, emits parseable line-delimited JSON, returns exit 2 when `anchor` is missing, and preserves the Anchor exit code on build failure
- [x] `src/runtime/subprocess.rs` — testable `CommandSpec` / `ProcessRunner` / `SubprocessRunner` boundary for Phase 3 subprocess orchestration
- [x] `src/runtime/pipeline.rs` initial build pipeline — runs `anchor build`, then `pnpm exec codama run` unless `--no-codama` is set, stops before Codama when Anchor fails, and keeps the runner injectable for tests
- [x] Watch-triggered pipeline core — debounced file change batch → `BuildPipeline` with injectable subprocess runner
- [x] Long-running watcher source — `chain serve --headless` instantiates `notify`, receives filesystem events, ticks debounce deadlines, and emits parseable line-delimited JSON for watcher-triggered builds
- [ ] Frontend notify after Codama regeneration
- [ ] `src/tui/serve_model.rs` — ratatui panels (validator / build / faucet / frontend / logs), 80×24 minimum
- [ ] `chain serve` full runtime — Surfpool/test-validator supervisor + watcher + build/codama + frontend notify; `--headless` remains parseable line-delimited JSON
- [ ] Clean Ctrl-C teardown (process tree, no orphans)

**DoD.** ADR-0001 §10.5.

---

### Phase 4 — Codegen & Frontend Hooks 📋

**Goal.** Wrap codama; emit framework-agnostic IDL artifacts and TanStack-Query hooks.

**Deliverables.**
- [ ] `src/codegen/codama.rs` + `codama_config.rs`
- [ ] `src/codegen/frontend_hooks.rs` — TanStack / Solid Query wrappers derived from IDL
- [ ] `sunscreen generate clients`, `generate idl`, `generate frontend-hooks`
- [ ] Idempotent regeneration; generated hooks compile in a vanilla Next.js project against a local Surfpool

**DoD.** ADR-0001 §10.6.

---

### Phase 5 — Recipes 📋

**Goal.** Composite scaffolders that produce working dApp slices in one command.

**Deliverables.**
- [ ] `src/scaffold/crud.rs` — 4 instructions + state + events + errors + tests + hooks
- [ ] `src/scaffold/recipes/spl_token.rs`
- [ ] `src/scaffold/recipes/metaplex_nft.rs`
- [ ] `sunscreen scaffold {crud, spl-token, metaplex-nft}`
- [ ] E2E: `chain new` + `scaffold crud Post` + `chain serve` produces a working blog dApp in <5 min

**DoD.** ADR-0001 §10.7.

---

### Phase 5.5 — Onboarding Layer 📋 (NEW per ADR-0005)

**Status.** 📋 Planned. Cannot start until Phase 5 recipes exist (`quickstart nft` composes `scaffold metaplex-nft`).

**Goal.** A newcomer can go from "I just installed sunscreen" to "my NFT is minted on devnet" in **under 10 minutes**, without reading prose docs first. See [`docs/adr/ADR-0005-beginner-onboarding.md`](docs/adr/ADR-0005-beginner-onboarding.md).

**Sprint split (ADR-0005 §6).**

| Sprint | Deliverable | Tests |
|---|---|---|
| **S1** | `init` (wizard + validator share), `wallet *`, `next_step` contract on 100% of error variants | unit + golden transcripts |
| **S2** | `examples {list, describe, use}`, `quickstart {token, nft, dao, blog}`, `deploy`, `learn` (5 MVP topics) | E2E on localnet; embedded-asset integrity test |

**Components (per ADR-0005 §6.1).**
- [ ] `src/onboarding/{tty, wizard, wallet, deploy, examples, learn}.rs`
- [ ] `src/onboarding/recipes/{token, nft, dao, blog}.rs`
- [ ] `src/strings/en_US.rs` — every user-facing string centralised
- [ ] `src/error.rs` extended so every `SunscreenError` variant exposes a `next_step` — implementation choice (per-variant associated data or a `next_step()` method on the enum) is left to the PR; the external contract is what matters (see ADR-0005 §4.2)
- [ ] `assets/examples/{token-faucet, nft-collection, escrow, voting-dao, blog-crud}/`
- [ ] `assets/learn/{pda, cpi, token-2022, accounts-model, anchor-vs-native}.md`
- [ ] TTY detection + `--non-interactive` (CI path)

**DoD.** Newcomer → NFT on devnet in <10 min, measured on a fresh macOS + a fresh Ubuntu VM, no editor required.

---

### Phase 8 — Distribution & Docs (v1.0) 📋

**Goal.** Cut v1.0 with multi-OS prebuilt binaries and a published docs site.

**Deliverables.**
- [ ] `cargo-dist` config — linux/amd64, linux/arm64, darwin/amd64, darwin/arm64, windows/amd64
- [ ] Homebrew tap, optional `cargo binstall` path
- [ ] Docs site (mdBook or Starlight per ADR-0003)
- [ ] Shell completions (bash / zsh / fish / pwsh) emitted by `sunscreen completions`
- [ ] `CHANGELOG.md` populated; SemVer policy published

**DoD.** Published `v1.0.0` GitHub release; `sunscreen --help` reachable from a one-line install on all four primary platforms.

---

### Phase 6 — Plugin System 🔮 (post-v1.0)

Deferred to preserve a tight v1.0. Design intent unchanged from ADR-0001 §10.8: gRPC (tonic) and stdio transports, reference plugins for spl-token-2022 and a Yellowstone indexer. Will be revisited once the v1.0 surface stabilises.

---

### Phase 7 — Pinocchio Support 🔮 (post-v1.0)

Explicitly deferred per ADR-0001 §10.9. Requires its own ADR before scoping; not on the v1.0 critical path.

---

## 3. Critical Path & Dependencies

```text
Phase 2 R4 (program + doctor --fix-markers) ✅
   └─► Phase 2 R5 (polish, test counts)
         └─► Phase 3 (chain serve / build)
               └─► Phase 4 (codegen, frontend hooks)
                     └─► Phase 5 (recipes: crud, spl-token, metaplex-nft)
                           └─► Phase 5.5 (onboarding: quickstart wraps recipes)
                                 └─► Phase 8 (cargo-dist + docs site) ─► v1.0
```

- **R5 is the immediate unblock.** Marker hardening, compile coverage, and golden coverage are landed. The initial Phase 3 `chain build` runner exists to support the remaining real integration target, but full Phase 3 should wait until that target is either met or intentionally rescoped.
- **Phase 5.5 strictly follows Phase 5.** `quickstart nft` is a thin shell over `scaffold metaplex-nft`; without recipes there is nothing to wrap.
- **Phase 6 (plugins) and Phase 7 (Pinocchio) are parallelisable and post-v1.0.** They do not gate v1.0 and should not pull engineering attention until v1.0 ships.
- **Docs (Phase 8) can start drafting during Phase 5.5** — content for `learn/*.md` overlaps with the user-facing tutorial pages.

---

## 4. Maintaining This Document

- Update the per-phase checkboxes and the §1 status column on every merged change.
- Add a one-line entry to `CLAUDE.md` Variation log for any phase transition.
- When a phase reaches "done", reduce its section to a one-paragraph summary; keep deliverable lists for in-flight phases only.
- Major scope changes still warrant a new ADR; reference it from the affected phase.
