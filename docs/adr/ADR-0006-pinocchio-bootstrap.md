# ADR-0006 — Pinocchio Bootstrap Support

| Field | Value |
|---|---|
| **Status** | Implemented |
| **Date** | 2026-06-02 |
| **Authors** | Pantani |
| **Tags** | pinocchio, framework, workspace, build, phase-7 |
| **Supersedes** | ADR-0001 §10.9 post-v1.0 deferral for the bootstrap MVP |
| **Superseded by** | — |
| **Related** | ADR-0001, ADR-0002, ADR-0003, ADR-0004, ROADMAP.md |

---

## TL;DR

Phase 7 is closed as a **Pinocchio bootstrap MVP**. `sunscreen chain new --framework pinocchio` now generates a minimal Pinocchio workspace, `chain build` routes it through `cargo build-sbf`, and Anchor-only scaffold/codegen commands fail early with explicit guidance instead of trying to mutate incompatible source.

This deliberately does not claim full Pinocchio parity with Anchor. Native Pinocchio instruction scaffolding, Shank/IDL generation, and Pinocchio-specific recipes should land through a later ADR or plugin surface once their contracts are proven.

---

## 1. Context

ADR-0001 originally marked Pinocchio support as post-v1.0 because the main value stream was Anchor-first: marker-based Anchor scaffolders, Anchor IDLs, Codama, and recipes derived from Anchor program shape.

After Phase 6, the project already had:

- `Framework::Pinocchio` in the config schema.
- A plugin runtime for ecosystem-specific scaffolders.
- A supervised build pipeline with injectable subprocess boundaries.
- A roadmap request to close Phase 7 in the active PR.

The missing slice was the safe first step: create and build Pinocchio workspaces without pretending Anchor-specific codegen can operate on them.

## 2. Decision

Implement Pinocchio as a first-class framework option for workspace bootstrap:

- Add CLI `--framework pinocchio`.
- Add `templates/workspace/pinocchio-minimal`.
- Emit `project.framework: pinocchio` in `sunscreen.yml`.
- Make preflight framework-aware: Pinocchio requires Rust/Cargo/Solana, not Anchor.
- Make the build pipeline framework-aware: Anchor uses `anchor build` plus optional Codama; Pinocchio uses `cargo build-sbf` and skips Codama.
- Preserve frontend template support because frontend shell files are not Anchor-specific.
- Guard built-in scaffolders and `generate` with clear Anchor-only errors.

## 3. Rationale

This gives users a working Pinocchio starting point while preserving the safety model of previous phases. The existing scaffolders patch Anchor-specific constructs such as `#[program]`, `Context`, `Accounts`, `#[event]`, and `#[error_code]`; applying those to Pinocchio would be worse than not supporting Pinocchio at all.

The plugin runtime creates an escape hatch for early Pinocchio ecosystem experiments without forcing the core CLI to bless a premature Shank/IDL or native scaffolding contract.

## 4. Consequences

Positive:

- Pinocchio becomes visible and testable through the same `chain new` command family.
- `chain build` works for both framework families with parseable NDJSON events.
- Existing Anchor workflows remain unchanged.
- Unsupported commands fail before mutating files.

Negative:

- Pinocchio does not yet have first-party instruction/account/event/error scaffolders.
- `generate {idl,clients,frontend-hooks}` remains Anchor-IDL-based.
- Real `cargo build-sbf` validation still requires a Solana toolchain on the host.

## 5. Follow-ups

- Decide whether Shank is the canonical IDL path for Pinocchio.
- Add Pinocchio-native scaffolders or ship them as reference plugins.
- Add real gated integration coverage for `cargo build-sbf` on machines with Solana installed.
- Revisit frontend hook generation once Pinocchio IDL artifacts exist.
