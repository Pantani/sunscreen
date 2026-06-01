# Sunscreen — Tooling CLI for Solana (Rust)

Greenfield Rust CLI inspired by Ignite CLI. Scope: incremental scaffolding of Anchor 1.0 programs, dev loop orchestration (Surfpool + Codama + frontend), and plugin system. Design source of truth: `docs/adr/ADR-0001-solis-cli.md` (with the name adapted from "solis" → "sunscreen" and the language from Go → Rust). Tactical roadmap: `IMPLEMENTATION-KICKOFF.md`.

## Harness: sunscreen

**Goal:** Implement and evolve the sunscreen CLI using a parallel team of specialized agents.

**Trigger:** Any request to implement, expand, fix, or refactor the sunscreen CLI → invoke the `sunscreen-orchestrator` skill. Conceptual questions about Solana/Anchor can be answered directly.

**Key variation from the ADR:** the ADR refers to Go/solis; this project is Rust/sunscreen. Preserve the strategic decisions (Anchor IDL as the source of truth, marker-based editing, plugin protocol, etc.) but switch every stack reference to Rust (clap, serde, minijinja, rust-embed, tokio, insta).

**Variation log:**
| Date | Change | Target | Reason |
|------|--------|--------|--------|
| 2026-05-31 | Initial harness configuration | agents/* + skills/sunscreen-orchestrator | bootstrap |
| 2026-05-31 | Phase 0 Week 1 implemented | src/{cli,config,toolchain,templates,error} | 16/16 tests green |
| 2026-05-31 | docs-writer agent added | agents/docs-writer.md | ADRs Week 2 |
| 2026-05-31 | Phase 0 Week 2 implemented | .github/, docs/adr/0002-0003, benches/, migration v0→v1 | 22/22 tests, cold-start 3.18ms |
| 2026-05-31 | Phase 1 (Workspace Bootstrap) implemented | src/fsutil/, src/cli/chain.rs, templates/workspace/, expanded Config v1, preflight | 59/59 tests, `chain new` E2E functional |
| 2026-05-31 | Phase 2 R1 (rustpatch + scaffold instruction) implemented | src/rustpatch/, src/workspace/, src/cli/scaffold.rs, src/templates/instruction.rs, templates/scaffold/instruction/, docs/reference/markers.md, docs/adr/ADR-0004 | 96/96 tests, idempotency + drift detection, fmt/clippy clean. Carry-over R2: `dispatch` segment in `lib.rs.j2` |
| 2026-05-31 | Phase 2 R2 dispatch carry-over | templates/workspace/anchor-multiple/programs/__program__/src/lib.rs.j2, templates/workspace/anchor-multiple/programs/__program__/src/instructions/mod.rs, tests/scaffold_instruction.rs, tests/golden/snapshots/* | A freshly created workspace already ships with `segment=dispatch` in `lib.rs` and `segment=instructions` in `instructions/mod.rs`; the first `scaffold instruction` patches without warning (`lib_rs_patched=true`). 103/103 tests, golden snapshots updated. |
| 2026-05-31 | Phase 2 R3 (account/event/error scaffolders) | src/cli/scaffold.rs, src/templates/{account,event,error}.rs, templates/scaffold/{account,event,error}/*, tests/scaffold_{account,event,error}.rs | The 3 remaining Phase 2 scaffolders delivered with the same architecture as instruction (idempotency, --dry-run, --json, conflict → exit 4 user_input). Generator tag `account`/`event`/`error` on every marker (D2 fix). Account conflict now returns a clear error (D3 fix). 115/115 tests, fmt+clippy clean. **Carry-over R4:** `program` scaffolder + `chain doctor --fix-markers` + auto-inclusion of `pub mod events/errors/state` in `lib.rs`. |
| 2026-06-01 | ADR-0005 (Beginner Onboarding Surface) proposed | docs/adr/ADR-0005-beginner-onboarding.md | Formalizes Phase 5.5 — Onboarding Layer (init wizard, examples, quickstart, wallet, deploy, learn, actionable errors). Status: Proposed. DoD: newcomer → NFT on devnet in < 10 min. PR #6. |
| 2026-06-01 | Consolidated ROADMAP.md added | ROADMAP.md | Unifies ADR-0001 §10 + IMPLEMENTATION-KICKOFF + ADR-0005 into a single live tracker with per-phase status. Supersedes the roadmap sections of those docs (originals kept as historical context). Total to v1.0 recomputed: ~21 weeks. |
