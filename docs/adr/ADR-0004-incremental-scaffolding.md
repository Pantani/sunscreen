# ADR-0004 — Incremental Scaffolding Strategy for `sunscreen`

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-05-31 |
| **Authors** | Danilo Lacombe |
| **Tags** | scaffolding, codegen, markers, anchor, rust-mutation |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | ADR-0001 § 7.1 and § 8.4 (sunscreen CLI), ADR-0002 (CLI Design Conventions), ADR-0003 (Documentation Strategy), `docs/reference/markers.md` |

---

## Variation Log

| Date | Author | Version | Summary |
|------|--------|---------|---------|
| 2026-05-31 | Pantani | 1.0.0 | Initial ADR |

---

## TL;DR

`sunscreen` adopts **incremental scaffolding via marker-based editing** as the primary strategy for mutations in Rust files of existing Anchor 1.0 workspaces (`lib.rs`, `instructions/mod.rs`, `state/mod.rs`, `errors.rs`, `events.rs`). Each scaffolder (`instruction`, `account`, `event`, `error`, `program`) operates exclusively within regions delimited by `// === sunscreen:auto-generated:begin segment=… ===` comments (cf. `docs/reference/markers.md`). User code lives in `user-region` regions and is treated as immutable. For the rare cases where insertion into unmarked territory is unavoidable (referencing a user-named struct), `sunscreen` falls back to **`ast-grep` as a subprocess** — without any runtime dependency on the Rust toolchain. This decision follows Sub-ADR-001 of ADR-0001 § 7.1 and formalizes the R1→R5 implementation roadmap for the scaffolders in Phase 2.

---

## 1. Context

### 1.1 Problem framing

The differentiator of `sunscreen` versus a simple template renderer (à la `cargo generate`) is the ability to **keep generating code after bootstrap**. A user runs `sunscreen chain new escrow`, edits the handlers, and then runs `sunscreen scaffold instruction deposit` — the tool must:

1. Add `pub mod deposit;` to `instructions/mod.rs`.
2. Add an arm in `#[program] pub mod escrow { … }` inside `lib.rs`.
3. Create `instructions/deposit.rs` with a `#[derive(Accounts)]` struct + `handler` skeleton.
4. **Not touch** a single byte of logic that the user already wrote in the other instructions.

This constraint (preserving user code) is the central tension. Anchor 1.0 + IDL as the source of truth reduces the problem (most of the generated code is mechanically derivable from the CLI args), but does not eliminate it: `lib.rs` mixes generated code (dispatch arms) with code that may have been touched by the user (imports, `#[program]` attributes).

### 1.2 Constraints

- **Reentrancy.** Running the same scaffold command twice with the same args ⇒ binary no-op (no diff).
- **Safety.** Do not silently corrupt the workspace. Failures must be detected before any disk write.
- **No runtime dependency on the Rust toolchain for editing.** `sunscreen` is a standalone binary; it cannot depend on `cargo` or on a dynamic `tree-sitter` C library.
- **Anchor 1.0 + IDL first.** Most mutations are mechanically derivable from args + IDL — we do not need a full AST to insert a `pub mod` in a known location.

---

## 2. Decision Drivers

- **DD1 — Idempotency.** Re-running the same scaffold is a no-op.
- **DD2 — Preservation of user code.** Never overwrite bytes the user wrote.
- **DD3 — No heavy runtime dependency.** No linking `libclang`, dynamic `tree-sitter`, or requiring `cargo` on the user's machine just for `scaffold`.
- **DD4 — Speed.** `scaffold instruction X` must complete in < 100 ms on a 50-file project. Cold-start is already at 3.18 ms (Phase 0 W2).
- **DD5 — Predictability / debuggability.** The user must be able to open the file and literally see which region belongs to the tool and which is theirs. A `// === sunscreen:auto-generated:begin … ===` comment is self-documenting.
- **DD6 — Robustness against `rustfmt`.** Any strategy that uses textual anchors must survive automatic formatting.

---

## 3. Considered Options

| # | Option | Summary |
|---|---|---|
| (a) | **Starter-only** | `sunscreen chain new` creates the project and that's it; all subsequent mutation is manual |
| (b) | **Marker-based editing** *(chosen)* | Regions delimited by structured comments; sunscreen only edits inside them |
| (c) | **AST via linked `tree-sitter-rust`** | Parse the file's CST, query the AST, emit modifications in Rust code |
| (d) | **`ast-grep` CLI as subprocess** | Same power as (c), but via an external binary with YAML rules |

### 3.1 Option (a) — Starter-only

**Pros:** trivial to implement; zero risk of corrupting user code.
**Cons:** eliminates the competitive differentiator. The value of Ignite/Cosmos comes from incremental scaffolding — `sunscreen scaffold instruction` must work on day 30 of the project, not just day 1.

**Rejected.** Reduces the product to a `cargo generate` with an Anchor theme.

### 3.2 Option (b) — Marker-based editing

**Pros:**
- Simple implementation: scan lines to find `begin`/`end`, replace the body, write.
- No runtime dependency on the Rust toolchain.
- Self-documenting: the user **sees** what is managed.
- Deterministic, fast, easy to test (golden tests).
- Survives `rustfmt` (line comments outside expressions — ADR-0001 § 9.5.1).
- Composes naturally with `user-region` to preserve handlers.

**Cons:**
- Only works in files that `sunscreen` generates. Mutation of user-authored code is out of reach.
- User renames/moves of files can "lose" markers.
- Match is line-oriented (`key=value` attribute parsing per `docs/reference/markers.md`): leading indentation is ignored and trailing `===` is tolerated, but free-form rewrites of the marker line will desync the scanner.

**Mitigation:** the set of files `sunscreen` actually needs to edit is small and canonical (sub-module `mod.rs`, `lib.rs` dispatch, `errors.rs`, `events.rs`). Everything else is one-file-per-instruction, created once and protected by `user-region` in the handler.

### 3.3 Option (c) — Linked `tree-sitter-rust`

**Pros:** structurally "correct"; understands real Rust syntax; does not depend on comments surviving.
**Cons:**
- Native binding (`tree-sitter` C lib) adds cross-platform build complications.
- Parse + query + emit is ≥10× slower than line-scan for a trivial operation (adding `pub mod X;`).
- Does not solve the problem of **choosing where** to insert — we still need a convention (the string `// instructions go here` or an equivalent marker comment). We end up reinventing markers, only without the self-documentation benefit.
- Hard to test — golden tests become CST comparisons, not text comparisons.

**Rejected as primary.** AST is overkill for 95% of the operations `sunscreen` needs to perform.

### 3.4 Option (d) — `ast-grep` CLI as subprocess

**Pros:** tree-sitter under the hood, but distributed as a standalone binary; rules in YAML; covers the 5% of cases where we need to reference user-authored identifiers.
**Cons:** external dependency that must be installed (mitigated: can be downloaded by `sunscreen doctor`); YAML rules are one more DSL for the contributor to learn.

**Accepted as an escape hatch.** Not primary, but available when needed.

---

## 4. Decision

`sunscreen` adopts:

1. **Marker-based editing as the primary strategy** for any mutation of Rust files generated by `sunscreen` itself. Canonical format in `docs/reference/markers.md`.
2. **`ast-grep` as an escape-hatch subprocess** for the rare case of insertion into user-authored territory (e.g., adding `#[event]` that references a struct in a user-named module).
3. **Three-phase pipeline** for every scaffold operation (aligned with ADR-0001 § 8.x):
   - **Plan** — computes a `FileSetPlan` in memory (creates, updates, marker-region edits) without touching disk.
   - **Validate** — dry-run: schema-check, marker lint, paths within the workspace, conflicts.
   - **Commit** — atomic per-file write (`<path>.sunscreen-tmp.<pid>` + rename); undo log for rollback.
4. Optional **post-commit hooks**: `cargo fmt --files-with-diff <changed>`, `cargo check` (gated by flag).
5. **Versioned markers** (`version=<n>`); bumps trigger automatic migrators.

This decision is consistent with Sub-ADR-001 (ADR-0001 § 7.1).

---

## 5. Consequences

### 5.1 Positive

- Simple and auditable implementation.
- Zero runtime dependency on the Rust toolchain for the primary path.
- Deterministic and fast (O(n) line-scan).
- Self-documenting: the user reads the file and literally sees the contract.
- Tests become trivial (golden + snapshot via `insta`).
- Naturally supports the `user-region` concept that enables handler preservation.

### 5.2 Negative

- Mutation of user-authored code is out of scope (except via `ast-grep`).
- Markers visually "pollute" the files. Mitigation: the visual convention `=== … ===` makes them readable and visually segregable.
- The user may accidentally delete a marker. Mitigation: `sunscreen doctor --fix-markers` (R4).
- User rename/move of files can decouple markers from the scaffolder's expectation. See § 7 Open Questions.

### 5.3 Mitigations

- Marker validation runs on **every** invocation of `sunscreen scaffold` before any write.
- Planned: a dedicated golden test for "markers survive `rustfmt --edition=2021`" (ADR-0001 § 9.5.1). CI currently runs `cargo fmt -- --check` over the workspace but not a fixture-format-then-re-scan loop; that test lands in Phase 2 R4.
- Migrators ensure that `version=` bumps do not break existing workspaces.

---

## 6. Implementation Plan (Phase 2)

Order of scaffolder implementation during Phase 2:

| Round | Scaffolder | Segments touched | Notes |
|---|---|---|---|
| **R1** | `instruction` | `instructions` (mod.rs), `dispatch` (lib.rs), `file` + `handler` (instruction.rs) | bootstraps the mechanism; covers all marker types |
| **R2** | dispatch carry-over | `dispatch` (lib.rs), `instructions` (mod.rs) | freshly-created workspace already ships markers so the first `scaffold instruction` patches cleanly |
| **R3** | `account` + `event` + `error` | `accounts`/`state` (state/<acc>.rs), `events` (events.rs), `error_variants` (errors.rs) | three scaffolders delivered together with the same architecture as `instruction` |
| **R4** | `program` + `doctor --fix-markers` | new sub-program + workspace-wide marker validation | composes R1–R3 over a new program inside an existing workspace; adds the recovery command |
| **R5** | polish | golden + compile + integration coverage | reach the ≥75 golden / ≥25 compile / ≥5 integration targets from ADR-0001 §10.4 |

Each round delivers: scaffolder + golden tests + entry in the `docs/reference/markers.md` table if it introduces a new segment.

### 6.1 Expected components

```text
src/
├── rustpatch/
│   ├── marker.rs       # scan / validate / apply
│   ├── segment.rs      # registry of segments + versions
│   ├── migrate/        # migrators version=N -> version=N+1
│   ├── astgrep.rs      # subprocess wrapper (escape hatch)
│   └── fmt.rs          # rustfmt invocation
├── scaffold/
│   ├── plan.rs         # FileSetPlan
│   ├── instruction.rs  # R1
│   ├── account.rs      # R2
│   ├── event.rs        # R3
│   ├── error.rs        # R4
│   └── program.rs      # R5
```

---

## 7. Open Questions

1. **User renames/moves.** If the user moves `instructions/deposit.rs` → `instructions/transfers/deposit.rs`, the scaffolder loses the reference. Options:
   - (i) Detect via `git mv` in history (fragile — user may not use git).
   - (ii) Maintain a `_workspace/.sunscreen/manifest.json` index with known paths and resolved segments.
   - (iii) Full re-scan of `src/` looking for all markers on every invocation (O(n) — likely choice).
   - **Current attempt:** (iii) + warning if an expected segment disappears.
2. **Drift between IDL and code.** The user may edit the `Deposit<'info>` struct manually, diverging from what `sunscreen` would generate. How to detect?
   - Option: re-running `scaffold instruction <name>` always regenerates the `file` segment (which is `auto-generated`) — so divergence is the user's *intent* by **not** running the command. `doctor` can compare the IDL produced by Anchor against the IDL inferred from the original args persisted in `.sunscreen/manifest.json` and warn.
3. **Multiple programs in the workspace.** R5 needs to decide whether markers carry a program qualifier or whether the file path is enough as context. Inclination: the path is enough; markers stay local to the file.
4. **Support for future Rust editions.** If Rust 2027 changes the behavior of line comments inside `mod`, the CI golden test will break first — reactive policy, not preventive.

---

## 8. Acceptance Criteria

- [ ] `docs/reference/markers.md` is the source of truth for the format and is linked from mdBook (ADR-0003).
- [ ] Scaffolders R1–R5 implemented with golden tests.
- [ ] Specific golden test "markers survive `rustfmt --edition=2021`" passes in CI (planned for Phase 2 R4; not yet implemented).
- [ ] Re-running any scaffold with the same args produces an empty diff.
- [ ] `sunscreen doctor --fix-markers` recovers from a corrupted marker in at least the scenarios listed in `docs/reference/markers.md` § 6.
