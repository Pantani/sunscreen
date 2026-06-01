# Marker Format Reference

> **Status:** canonical spec for marker syntax used by `sunscreen` scaffolders.
> **Related:** ADR-0001 § 7.1 (Rust Code Mutation Strategy), § 8.4 (Sample Generated File), ADR-0004 (Incremental Scaffolding).

`sunscreen` modifies existing Rust files (`lib.rs`, `instructions/mod.rs`, `errors.rs`, etc.) through **regions delimited by structured comments** — called *markers*. This page is the source of truth for the format. Any divergence between an implementation and this document is a bug.

---

## 1. Philosophy

- Markers are **line comments** (`//`), never block comments. This guarantees survival under `rustfmt` (invariant item, see § 5).
- Markers never appear **inside** an expression, `match`, `if`, or arbitrary `{}` block; always at item scope (top-level, inside `mod {}`, inside the `#[program]` block).
- There are **two kinds** of region:
  - `auto-generated` — `sunscreen`'s territory. Will be **overwritten** on every `sunscreen scaffold`.
  - `user-region` — the human's territory. `sunscreen` **never touches** it after initial creation.
- Markers work in pairs (`begin` / `end`) and are matched by **line-by-line parsing**: `sunscreen` recognises a line only when its trimmed prefix is `// ===` followed by a `sunscreen:` namespace (see `src/rustpatch/marker.rs::strip_marker_prefix`). Leading indentation is stripped and attributes are tokenised by whitespace as `key=value`. No regex; mid-line occurrences are NOT treated as markers.

---

## 2. Formal Syntax

### 2.1 Auto-managed region

```rust
// === sunscreen:auto-generated:begin segment=<name> version=<n> [generator=<g>] ===
// DO NOT EDIT THIS REGION. Manual changes will be overwritten by `sunscreen scaffold`.
//
// <managed content>
//
// === sunscreen:auto-generated:end segment=<name> ===
```

### 2.2 User region

```rust
// === sunscreen:user-region:begin segment=<name> ===
// You can freely edit anything inside this region.
//
// <user content — sunscreen never overwrites>
//
// === sunscreen:user-region:end segment=<name> ===
```

### 2.3 Grammar

```text
MARKER       := AG_BEGIN | AG_END | UR_BEGIN | UR_END

AG_BEGIN := "// === sunscreen:auto-generated:begin"
            " segment=" NAME " version=" INT
            ( " generator=" IDENT )?
            " ==="                              // version= required; user-region does not version


AG_END   := "// === sunscreen:auto-generated:end segment=" NAME " ==="

UR_BEGIN := "// === sunscreen:user-region:begin segment=" NAME " ==="
UR_END   := "// === sunscreen:user-region:end segment=" NAME " ==="

NAME  := [a-z][a-z0-9_-]*
INT   := [0-9]+        // any u32; leading zeros and 0 are accepted by the scanner
IDENT := [a-z][a-z0-9_-]*
```

Additional rules:

- `===` at the start of the line (after `//`) is required. The closing `===` is conventional (preserved during generation), but the current scanner tolerates its absence — future versions may make it strict.
- The scanner ignores **leading indentation** (trim_start) and tokenizes attributes by whitespace; the order of extra attributes is tolerated. Generation always produces the canonical form (`segment=` before `version=`).
- `version` only appears in `auto-generated`. `user-region` does not version (sunscreen never migrates user content).
- `generator` is diagnostic (which scaffolder produced the segment).

---

## 3. Marker Kinds

| Kind | `sunscreen` writes | `sunscreen` reads | User edits | Survives re-scaffold |
|---|---|---|---|---|
| `auto-generated` | yes, on every scaffold | yes | **no** (will be overwritten) | content is regenerated |
| `user-region` | only on initial creation | yes (to preserve offsets) | **yes, freely** | yes, preserved byte-for-byte |

> Mental summary: `auto-generated` = "sunscreen writes, human reads"; `user-region` = "human writes, sunscreen avoids".

---

## 4. Known Segments

| Segment | Default kind | Location | Content |
|---|---|---|---|
| `instructions` | `auto-generated` | `programs/<prog>/src/instructions/mod.rs` | `pub mod <ix>;` per instruction + re-exports |
| `dispatch` | `auto-generated` | `programs/<prog>/src/lib.rs` inside `#[program] pub mod <prog> { … }` | one `pub fn` per instruction, e.g. `pub fn deposit(ctx: Context<…>, …) -> Result<()> { instructions::deposit::handler(ctx, …) }` (the instruction name `<ix>` is substituted directly into both positions) |
| `file` | `auto-generated` | `programs/<prog>/src/instructions/<ix>.rs` | imports, `#[derive(Accounts)] struct <Ix>`, auxiliary structs |
| `handler` | `user-region` | same file as `file` | body of `pub fn handler(...) -> Result<()> { … }` |
| `accounts` *(R3)* | `auto-generated` | `programs/<prog>/src/state/mod.rs` | `pub mod <acc>;` |
| `state` *(R3)* | `auto-generated` | `programs/<prog>/src/state/<acc>.rs` | `#[account] pub struct <Acc> { … }` |
| `events` *(R3)* | `auto-generated` | `programs/<prog>/src/events.rs` | `#[event]` declarations |
| `error_variants` *(R3)* | `auto-generated` | `programs/<prog>/src/errors.rs` | variants of the `#[error_code]` enum |

Future segments are added to this table and introduced with `version=1`; subsequent bumps (`version=2`, …) trigger automatic migrators.

---

## 5. Invariants

1. **Survive `rustfmt`.** Because they are line comments outside any expression, `rustfmt --edition=2021` is expected to preserve them. CI currently runs `cargo fmt -- --check` over the workspace itself; a dedicated golden test that formats a fixture file with `rustfmt` and re-scans markers is planned for Phase 2 R5 (cf. ADR-0001 § 9.5.1).
2. **Never inside `match`, `if`, `for`, `while`, `loop`, or arbitrary `{ … }` block.** Markers live only at item scope.
3. **Line-grained.** Markers occupy entire lines; no inline marker alongside code.
4. **Paired and ordered.** For each `begin segment=X` there is exactly one later `end segment=X` in the same file. No nesting.
5. **Deterministic.** Same scaffold invocation with the same args ⇒ same content between the markers, byte-for-byte.

---

## 6. Common Errors and Recovery

| Symptom | Cause | Recovery |
|---|---|---|
| `error: marker pair mismatch: begin segment=dispatch without matching end` | user deleted the `end` line | `sunscreen chain doctor --fix-markers` reports the mismatch; for **appendable** host files (e.g. `instructions/mod.rs`, `state/mod.rs`) it appends a missing pair. `dispatch` lives inside `#[program]` and is **not** appendable today — the user must restore it manually or re-run `scaffold instruction` for each affected instruction |
| `error: duplicate begin segment=instructions in src/instructions/mod.rs` | unresolved merge conflict | resolve the conflict; keep only one pair |
| `error: marker drift: version=1 expected, found version=2` | CLI downgrade | upgrade `sunscreen` to a version that understands `version=2` (no reverse migrator exists yet — see § 7) |
| `error: marker inside expression` | user moved the marker inside a `match` | move it back to item scope |

> The scanner currently tolerates extra attributes on `user-region` markers silently (including a stray `version=`); enforcement is deferred until a real version bump exists for user regions. Treat extra attributes as a spec violation even though no warning is emitted today.

`sunscreen chain doctor` (shipped in Phase 2 R4, `src/cli/chain.rs::run_doctor`) validates markers across the entire workspace and offers `--fix-markers` for **appendable** recoverable cases. Note: the top-level `sunscreen doctor` is the **toolchain** diagnostic (verifies `anchor`/`solana`/`cargo`/etc.); workspace-level marker auditing lives under `chain doctor` per ADR-0001 §6.1.

---

## 7. Versioning

- Every `auto-generated` region carries `version=<n>`.
- A `version` bump indicates an **incompatible change** in the format of the content generated within that segment.
- When `sunscreen` encounters `version=N` but the current scaffolder emits `version=N+1`, it will run the corresponding **migrator** before rewriting the region. *No segment has bumped past `version=1` yet, so the migrator machinery in `src/rustpatch/` has not been built; it will be introduced the first time a real `version=2` lands.*
- `version=1` is the starting point for every segment listed in § 4.

---

## 8. Full Example

Workspace generated by `sunscreen chain new escrow` followed by:

```bash
sunscreen scaffold instruction deposit \
  --program escrow \
  --args "amount:u64" \
  --accounts "vault:mut|seeds=b\"vault\",depositor:signer|mut,system_program:system"
```

### 8.1 `programs/escrow/src/instructions/mod.rs`

```rust
// === sunscreen:auto-generated:begin segment=instructions version=1 ===
// DO NOT EDIT. Use `sunscreen scaffold instruction` to extend.
pub mod deposit;
pub mod initialize;
pub use deposit::*;
pub use initialize::*;
// === sunscreen:auto-generated:end segment=instructions ===
```

### 8.2 `programs/escrow/src/lib.rs`

```rust
use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

declare_id!("Esc11111111111111111111111111111111111111");

#[program]
pub mod escrow {
    use super::*;

    // === sunscreen:auto-generated:begin segment=dispatch version=1 ===
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::initialize::handler(ctx, fee_bps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }
    // === sunscreen:auto-generated:end segment=dispatch ===
}
```

### 8.3 `programs/escrow/src/instructions/deposit.rs`

```rust
// === sunscreen:auto-generated:begin segment=file version=1 generator=instruction ===
// This file is initial scaffolding. The handler body below is a user-region.
// Re-running `sunscreen scaffold instruction deposit` with the same args is a no-op.

use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}
// === sunscreen:auto-generated:end segment=file ===

// === sunscreen:user-region:begin segment=handler ===
// You can freely edit anything inside this region.
pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.total = vault.total.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(())
}
// === sunscreen:user-region:end segment=handler ===
```

The key point of this example: re-running `sunscreen scaffold instruction deposit` with the same args **does not touch** the `handler` body — it only validates that the `file` segment remains consistent with the provided args. Changing the args (e.g., adding `--accounts ",fee_receiver:mut"`) regenerates the `file` segment, leaving `handler` intact.

---

## 9. Conformance

Scaffolder implementations **must**:

1. Emit markers exactly as specified in § 2.
2. Validate pairing before applying any patch (fail-fast).
3. Treat `user-region` regions as read-only after creation.
4. Version every `auto-generated` segment with a numeric `version=`.
5. Fail with an actionable message pointing to `sunscreen chain doctor --fix-markers` when corruption is encountered.
