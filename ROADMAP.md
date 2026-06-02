# sunscreen — Roadmap

**Status:** Live tracker
**Last updated:** 2026-06-02 (v0.1.0 preview release pipeline)
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

Total to **v1.0**: ~28 weeks of focused work (vs. 16 weeks in the original ADR-0001 plan). The increase absorbs the +4-week Phase 5.5 (Onboarding) introduced by ADR-0005, brings Phase 6 (plugins) into the v1.0 line, and promotes the Phase 7 Pinocchio bootstrap MVP from post-v1.0 into this PR.

| Phase | Theme | Status | Duration | Cum. | Key deliverables | DoD source |
|---|---|---|---:|---:|---|---|
| **0** | Foundations | ✅ DONE | 2 wk | 2 wk | CLI skeleton, config v1, toolchain detect, doctor, CI, golden infra | ADR-0001 §10.2 |
| **1** | Workspace Bootstrap | ✅ DONE | 2 wk | 4 wk | `chain new` produces compilable Anchor workspace + frontend variants | ADR-0001 §10.3 |
| **2** | Incremental Scaffolding | ✅ DONE | 4 wk | 8 wk | `scaffold {instruction, account, event, error, program}` + `chain doctor --fix-markers` | ADR-0001 §10.4, ADR-0004 |
| **3** | Runtime Orchestration | ✅ DONE | 3 wk | 11 wk | `chain serve` (Surfpool/test-validator + watcher + codama + frontend notify + serve model), `chain build` | ADR-0001 §10.5 |
| **4** | Codegen & Frontend Hooks | ✅ DONE | 2 wk | 13 wk | `generate {clients, idl, frontend-hooks}`, Codama wrapper, IDL artifacts, React/Solid Query hooks | ADR-0001 §10.6 |
| **5** | Recipes | ✅ DONE | 3 wk | 16 wk | `scaffold {crud, spl-token, metaplex-nft}` | ADR-0001 §10.7 |
| **5.5** | Onboarding Layer | ✅ DONE | 4 wk | 20 wk | `init`, `quickstart`, `examples`, `wallet`, `deploy`, `learn`, `next_step` errors | ADR-0005 §6 |
| **6** | Plugin System | ✅ DONE | 4 wk | 24 wk | runtime manager, gRPC proto contract + stdio plugins, sandbox/trust model, marketplace/local plugins, 2 reference plugins | ADR-0001 §10.8 |
| **7** | Pinocchio support | ✅ DONE | 3 wk | 27 wk | `--framework pinocchio` MVP, Pinocchio template, Cargo/Solana toolchain config, `cargo build-sbf` pipeline, Anchor-only guards | ADR-0006 |
| **8** | Distribution & Docs (v1.0) | 🚧 IN PROGRESS | 1 wk | 28 wk | cargo-dist preview release, mdBook/Starlight docs, shell completions | ADR-0001 §10 |

> Phases are listed in ascending numeric order. Phase 6 and Phase 7 are now closed in the v1.0 line; Phase 8 cuts v1.0 after plugin and Pinocchio bootstrap closure.

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

**Goal.** `sunscreen chain new <name>` produces a workspace where `anchor build` succeeds for Anchor, with optional frontend.

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

**Phase 7 extension.** `chain new foo --framework pinocchio` now yields a Pinocchio workspace without `Anchor.toml` or `anchor-lang`; see Phase 7 and [`docs/reference/pinocchio.md`](docs/reference/pinocchio.md).

---

### Phase 2 — Incremental Scaffolding ✅

**Status.** ✅ DONE. R1–R5 plus follow-up hardening shipped. Phase 2 has no remaining known carry-overs: non-appendable `dispatch` / `error_variants` repair is hardened, and the stale no-accounts instruction compile test is active again. Strategy ratified in [`docs/adr/ADR-0004-incremental-scaffolding.md`](docs/adr/ADR-0004-incremental-scaffolding.md).

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
- [x] `sunscreen chain doctor --fix-markers` (`src/cli/chain.rs::run_doctor`) — scans the workspace for marker corruption, appends missing marker pairs for **appendable** host files (state/mod.rs, instructions/mod.rs, etc.), reconstructs the non-appendable `dispatch` segment inside `#[program]` when the generated body is gone and instruction files provide enough information, and inserts `error_variants` markers only for safe empty enums or existing marked regions.
- [x] Auto-injection of `pub mod events;` / `pub mod errors;` / `pub mod state;` in `lib.rs` on first relevant scaffold (closes a R3 gap where users had to add the line manually)
- [x] `tests/scaffold_program.rs` (also covers `chain doctor --fix-markers` paths)
- [x] `tests/rustfmt_roundtrip.rs` — golden test that runs `rustfmt --edition=2021` over fixture files containing every documented marker segment and re-scans the result; matches the invariant promised in `docs/reference/markers.md` §5 and ADR-0004 §4 (shipped in R5)
- [x] Reconstruction of the non-appendable `dispatch` site when both markers and generated body are gone — `chain doctor --fix-markers` inserts a fresh `dispatch` marker block inside `#[program]`, rebuilds wrappers from instruction files that define `pub fn handler`, and refuses ambiguous cases where wrappers already remain.
- [x] Safe recovery for the remaining non-appendable `error_variants` site — `chain doctor --fix-markers` inserts markers for empty multi-line `#[error_code]` enums and refuses ambiguous existing enum contents instead of wrapping user variants or appending invalid Rust at EOF.

#### R5 — Polish ✅

Shipped via PR #7.

- [x] ≥ 75 golden tests across all five scaffolders (37 render snapshots + 25 compile tests + existing scaffold suites)
- [x] ≥ 25 compile tests (`cargo check` of generated workspaces) — `tests/compile_generated.rs`, 25 tests, shared `CARGO_TARGET_DIR` cache (~4s warm)
- [x] 5 integration tests scaffolded — full `anchor build` + IDL inspection + codama regen (`tests/integration_anchor.rs`, `#[ignore]`d + skip-when-toolchain-missing)
- [x] `doctor --json` emits a flat `[ToolReport, …]` array where each report carries an `available` boolean (covers anchor, codama, solana, surfpool, rustfmt, …) + public `toolchain::detect_*` helpers for in-process callers
- [x] Phase 2 DoD per ADR-0001 §10.4 satisfied end-to-end (202 passing, fmt + clippy clean)

**Phase 2 follow-up status.** ✅ Closed. The stale ignored compile test for `scaffold instruction` without `--accounts` is active again; the current template emits an empty `Accounts` struct without an unused lifetime.

---

### Phase 3 — Runtime Orchestration ✅

**Status.** ✅ DONE in this PR. `chain build --headless` reuses workspace discovery and runs `anchor build` + optional Codama regeneration through a testable runtime pipeline with parseable line-delimited JSON events. `chain serve` now launches a managed Surfpool or `solana-test-validator` runtime, falls back automatically from implicit Surfpool to test-validator when Surfpool is missing, starts the watcher/build/Codama/frontend notification loop, and tears down the runtime process group on Ctrl-C.

**Goal.** `sunscreen chain serve` orchestrates Surfpool, file-watcher, codama regen, and a ratatui TUI in one supervised process tree.

**Deliverables.**
- [x] `src/runtime/surfpool.rs` + `testvalidator.rs` implementing a `Runtime` trait, shared endpoint contract, and minimal `RuntimeSupervisor` start/stop boundary
- [x] `src/runtime/watcher.rs` debounce core — batches relevant Rust/config changes after a quiet period, dedupes/sorts paths, and ignores generated/unrelated paths
- [x] `notify::Event` adapter — feeds raw notify paths into the debouncer and relativizes absolute paths against the workspace before pipeline filtering
- [x] `chain build --headless` initial runner — discovers the workspace, invokes `anchor build` at the workspace root, emits parseable line-delimited JSON, returns exit 2 when `anchor` is missing, and preserves the Anchor exit code on build failure
- [x] `src/runtime/subprocess.rs` — testable `CommandSpec` / `ProcessRunner` / `SubprocessRunner` boundary for Phase 3 subprocess orchestration
- [x] `src/runtime/pipeline.rs` initial build pipeline — runs `anchor build`, then `pnpm exec codama run` unless `--no-codama` is set, stops before Codama when Anchor fails, and keeps the runner injectable for tests
- [x] Watch-triggered pipeline core — debounced file change batch → `BuildPipeline` with injectable subprocess runner
- [x] Long-running watcher source — `chain serve --headless` instantiates `notify`, receives filesystem events, ticks debounce deadlines, and emits parseable line-delimited JSON for watcher-triggered builds
- [x] Frontend notify after Codama regeneration — successful Codama runs touch `app/.sunscreen/reload` when a scaffolded frontend exists and emit a `frontend_notified` JSON event
- [x] `src/tui/serve_model.rs` — serve model with validator / build / faucet / frontend / logs panels and 80×24 minimum guard
- [x] Integrate runtime supervisor into `chain serve --headless` — runtime selection/fallback, start event, stop event, and build watcher loop in one supervised path
- [x] `chain serve` full runtime — Surfpool/test-validator supervisor + watcher + build/codama + frontend notify; `--headless` remains parseable line-delimited JSON
- [x] Clean Ctrl-C teardown — runtime subprocesses are spawned in a Unix process group and stopped as a group, with SIGKILL fallback after SIGTERM

**DoD.** ADR-0001 §10.5.

---

### Phase 4 — Codegen & Frontend Hooks ✅

**Status.** ✅ DONE in this PR. `sunscreen generate` is no longer a stub: it exports deterministic IDL artifacts, writes a managed `codama.json`, runs Codama through the shared subprocess boundary, and generates React Query + Solid Query frontend hooks from Anchor IDLs. `chain build` and `chain serve` now reuse the same Codama wrapper.

**Goal.** Wrap codama; emit framework-agnostic IDL artifacts and TanStack-Query hooks.

**Deliverables.**
- [x] `src/codegen/codama.rs` + `codama_config.rs` — writes stable `codama.json` and runs `pnpm exec codama run --all --config codama.json`
- [x] `src/codegen/idl.rs` — exports `target/idl/*.json` into deterministic `clients/idl/*.json`
- [x] `src/codegen/frontend_hooks.rs` — framework-agnostic IDL/core TypeScript plus TanStack React Query and Solid Query wrappers derived from IDL instructions
- [x] `sunscreen generate clients`, `generate idl`, `generate frontend-hooks`
- [x] Idempotent regeneration covered by CLI tests; generated Next.js hook typecheck is available as a gated ignored test for machines with the JS toolchain installed

**DoD.** ADR-0001 §10.6. Operational details are documented in [`docs/reference/codegen.md`](docs/reference/codegen.md).

---

### Phase 5 — Recipes ✅

**Status.** ✅ DONE in this PR. `sunscreen scaffold` now has composite recipes for CRUD, SPL token, and Metaplex NFT slices. Recipes preflight their primitive steps before writing, reuse the Phase 2 marker-based scaffolders, and keep Phase 4 generated paths owned by `generate`. Operational details are documented in [`docs/reference/recipes.md`](docs/reference/recipes.md).

**Goal.** Composite scaffolders that produce working dApp slices in one command.

**Deliverables.**
- [x] `src/scaffold/crud.rs` — `create`, `read`, `update`, `delete` instructions + state + events + errors + recipe test + optional frontend hook
- [x] `src/scaffold/recipes/spl_token.rs`
- [x] `src/scaffold/recipes/metaplex_nft.rs`
- [x] `sunscreen scaffold {crud, spl-token, metaplex-nft}`
- [x] E2E smoke: `chain new` + `scaffold crud Post` covered by CLI tests; gated compile and real Anchor IDL coverage are available for toolchain-equipped machines.

**DoD.** ADR-0001 §10.7.

---

### Phase 5.5 — Onboarding Layer ✅

**Status.** ✅ DONE in this PR. The beginner surface is now present as top-level commands: `init`, `examples`, `quickstart`, `wallet`, `deploy`, and `learn`. `init` reuses the `chain new` workspace construction path; `quickstart {token,nft,dao,blog}` composes Phase 5 scaffolders; `wallet` and `deploy` use the shared subprocess boundary; examples and learn topics are embedded and offline; `SunscreenError` now exposes `next_step`, `PathConflict` (exit 7), and `Network` (exit 8). Operational details are documented in [`docs/reference/onboarding.md`](docs/reference/onboarding.md).

**Goal.** A newcomer can go from "I just installed sunscreen" to "my NFT is minted on devnet" in **under 10 minutes**, without reading prose docs first. See [`docs/adr/ADR-0005-beginner-onboarding.md`](docs/adr/ADR-0005-beginner-onboarding.md).

**Sprint split (ADR-0005 §6).**

| Sprint | Deliverable | Tests |
|---|---|---|
| **S1** | `init` (wizard + validator share), `wallet *`, `next_step` contract on 100% of error variants | `tests/errors_contract.rs`, `tests/onboarding_init_quickstart.rs`, `tests/onboarding_wallet_deploy.rs` |
| **S2** | `examples {list, describe, use}`, `quickstart {token, nft, dao, blog}`, `deploy`, `learn` (5 MVP topics) | `tests/onboarding_examples_learn.rs`, `tests/onboarding_init_quickstart.rs`, fake subprocess boundary tests |

**Components (per ADR-0005 §6.1).**
- [x] `src/onboarding/{tty, wizard, wallet, deploy, examples, learn}.rs`
- [x] `src/onboarding/recipes/{token, nft, dao, blog}.rs`
- [x] `src/strings/en_US.rs` — every user-facing string centralised
- [x] `src/error.rs` extended so every `SunscreenError` variant exposes a `next_step`; JSON errors now include `next_step` and `exit_code`
- [x] `assets/examples/{token-faucet, nft-collection, escrow, voting-dao, blog-crud}/`
- [x] `assets/learn/{pda, cpi, token-2022, accounts-model, anchor-vs-native}.md`
- [x] TTY detection + `--non-interactive` (CI path)
- [x] Default `onboarding` Cargo feature gates the command modules and embedded assets for `--no-default-features` builds

**DoD.** Offline implementation is complete and covered by deterministic tests. The newcomer → NFT on devnet <10 min stopwatch remains a gated manual validation for a machine with Anchor/Solana/pnpm installed.

---

### Phase 6 — Plugin System ✅

**Status.** ✅ DONE in this PR. The Phase 6 documentation and runtime contract are captured in [`docs/reference/app.md`](docs/reference/app.md). The implementation closes local plugin discovery, manifest validation, stdio JSON-RPC execution, gRPC proto contract, sandbox/trust boundaries, dynamic scaffold routing, lifecycle hook execution, and the offline reference marketplace.

**Goal.** Turn the existing declarative `sunscreen app` lifecycle into a supervised plugin runtime that can register commands and hooks without modifying core.

**Already shipped.**
- [x] `sunscreen app {install,uninstall,list,describe,update}` manages declarative plugin entries in `plugins[]` of `sunscreen.yml`
- [x] Idempotent install, `--dry-run`, basename normalization (`github.com/org/foo.git` → `foo`), semver validation, and stable JSON envelope

**Deliverables.**
- [x] Runtime manager that resolves declared local-path plugins and reports available dynamic commands from their manifests
- [x] gRPC contract finalized in `proto/plugin.proto` with `initialize`, `capabilities`, `run_command`, `run_hook`, and `shutdown`
- [x] stdio JSON-RPC transport with `Content-Length: N\r\n\r\n{json}` framing, dynamic command dispatch, and hook execution
- [x] Plugin manifest validation for `sunscreen-plugin.json` (`name`, `version`, `transport`, `entrypoint`, commands, hooks, capabilities)
- [x] Sandbox/trust model: workspace + scratch filesystem boundary, path-traversal rejection, sanitized environment, and Unix process-group launch
- [x] Marketplace conventions and offline reference index for `sunscreen-apps/spl-token-2022` and `sunscreen-apps/yellowstone-indexer`
- [x] `app commands`, `app run`, `app hook`, and plugin-backed `scaffold <noun>` command routing
- [x] CI smoke now includes `tests/app_lifecycle.rs` explicitly

**DoD.** A third-party local plugin can register a new `sunscreen scaffold <noun>` command and a lifecycle hook without modifying `sunscreen` core; stdio runtime and gRPC proto-contract tests pass; sandbox/runtime violations fail with exit 9 (`plugin_runtime`).

---

### Phase 7 — Pinocchio Support ✅

**Status.** ✅ DONE in this PR. Phase 7 was promoted from ADR-0001's original post-v1.0 bucket by [`ADR-0006`](docs/adr/ADR-0006-pinocchio-bootstrap.md) and closed as a bootstrap MVP: users can create a Pinocchio workspace, build it through the supervised pipeline, and receive explicit guardrails where the existing Anchor-only scaffold/codegen surfaces do not apply.

**Goal.** Add first-class `--framework pinocchio` workspace bootstrap without pretending the Anchor IDL/scaffolder stack already works for native Pinocchio programs.

**Deliverables.**
- [x] `sunscreen chain new <name> --framework pinocchio` with `--dry-run`, `--json`, preflight, path conflict handling, frontend variants, and Pinocchio-specific next steps
- [x] `templates/workspace/pinocchio-minimal/` — Cargo workspace + no_std/Solana-SBF-aware Pinocchio program crate, stable runtime/entrypoint markers, no `Anchor.toml`, no `anchor-lang`
- [x] `sunscreen.yml` emits `project.framework: pinocchio`, `scaffolding.default_template: workspace/pinocchio-minimal`, and Rust/Cargo/Solana toolchain requirements
- [x] `chain build --headless` and watcher/serve pipeline route Pinocchio workspaces through `cargo build-sbf` and skip Codama by default
- [x] Preflight for Pinocchio requires Rust/Cargo/Solana but not Anchor; JS frontends still require Node/pnpm
- [x] Built-in scaffolders and `generate` reject Pinocchio workspaces with clear user-input errors; plugin-backed `scaffold <noun>` remains available for ecosystem-specific Pinocchio extensions
- [x] Golden snapshot, binary smoke tests, and offline generated-workspace compile coverage

**DoD.** `chain new --framework pinocchio` produces a cargo-checkable workspace, `chain build --headless` emits `pinocchio_build` events and calls `cargo build-sbf`, and unsupported Anchor-only commands fail before mutating user files.

Operational details are documented in [`docs/reference/pinocchio.md`](docs/reference/pinocchio.md).

---

### Phase 8 — Distribution & Docs (v1.0) 🚧

**Status.** 🚧 In progress. The `v0.1.0` preview release is the first distribution slice: GitHub Actions builds and publishes Linux/macOS `cargo-dist` artifacts, `CHANGELOG.md` now carries SemVer/release notes, and the remaining v1.0 work stays focused on docs/completions/additional distribution channels.

**Goal.** Cut v1.0 with multi-OS prebuilt binaries and a published docs site.

**Deliverables.**
- [x] Ignite-style Rust CLI integration harness — builds/uses the real `sunscreen` binary, isolates HOME/PATH, provides fake Solana/Anchor/Codama toolchain scripts, and runs command-group smoke suites for `chain`, `scaffold`, `generate`, onboarding commands, plugin runtime, and Pinocchio guardrails.
- [x] CI hardening for v1.0 QA — explicit command-group integration smoke job, locked Cargo commands, no-default-features build check, workflow concurrency, permissions, and timeouts.
- [x] `cargo-dist` baseline for Linux/macOS — `Cargo.toml` metadata + release workflow cover linux/amd64, linux/arm64, darwin/amd64, darwin/arm64.
- [x] `v0.1.0` preview release pipeline — tag-driven `cargo dist plan`, per-target Linux/macOS archives, global shell installer/checksum artifacts, and GitHub Release publishing from versioned notes.
- [x] `CHANGELOG.md` populated with a preview-line SemVer policy and `v0.1.0` release notes.
- [ ] Complete distribution matrix — add/validate windows/amd64 and final v1.0 artifact publishing polish.
- [ ] Homebrew tap, optional `cargo binstall` path
- [ ] Docs site (mdBook or Starlight per ADR-0003)
- [ ] Shell completions (bash / zsh / fish / pwsh) emitted by `sunscreen completions`

**DoD.** Published `v1.0.0` GitHub release; `sunscreen --help` reachable from a one-line install on all four primary platforms.

---

## 3. Critical Path & Dependencies

```text
Phase 2 R4 (program + doctor --fix-markers) ✅
   └─► Phase 2 R5 (polish, test counts)
         └─► Phase 3 (chain serve / build)
              └─► Phase 4 (codegen, frontend hooks) ✅
                    └─► Phase 5 (recipes: crud, spl-token, metaplex-nft) ✅
                           └─► Phase 5.5 (onboarding: quickstart wraps recipes) ✅
                                 └─► Phase 6 (plugin runtime + reference plugins) ✅
                                       └─► Phase 7 (Pinocchio bootstrap) ✅
                                             └─► Phase 8 (cargo-dist + docs site) 🚧 ─► v1.0
```

- **Phase 5 is closed.** Composite recipe scaffolding can now consume the generated hooks/client surface without owning Phase 4 generated paths.
- **Phase 5.5 is closed.** The top-level beginner commands now wrap the core scaffolding/runtime/deploy surfaces without duplicating their internals.
- **Binary-level CLI integration smoke now exists.** The Ignite-inspired Rust harness exercises the command groups through the compiled binary with isolated temp workspaces and fake external tools, giving Phase 8 a release-oriented QA layer without requiring network or a real Solana toolchain.
- **CI now runs that smoke layer explicitly.** The main pipeline also enforces lockfile use and keeps the optional onboarding feature boundary building via a no-default-features target check.
- **Phase 6 (plugins) is closed.** The declarative `app` MVP now has local manifest discovery, stdio execution, gRPC proto contract, sandbox/runtime failure handling, reference marketplace entries, dynamic scaffold routing, and lifecycle hook execution.
- **Phase 7 (Pinocchio) is closed.** The CLI can now bootstrap Pinocchio workspaces, route builds through `cargo build-sbf`, and guard Anchor-only scaffold/codegen paths before mutation.
- **Phase 8 is in progress.** The `v0.1.0` preview release covers Linux/macOS binary distribution and release notes; published docs, shell completions, Windows/Homebrew channels, and release polish still gate v1.0.

---

## 4. Maintaining This Document

- Update the per-phase checkboxes and the §1 status column on every merged change.
- Add a one-line entry to `CLAUDE.md` Variation log for any phase transition.
- When a phase reaches "done", reduce its section to a one-paragraph summary; keep deliverable lists for in-flight phases only.
- Major scope changes still warrant a new ADR; reference it from the affected phase.
