# Marker protocol

Markers are how sunscreen edits files it has generated, without clobbering your hand-edits.

## Anatomy

```rust
// sunscreen:begin {generator=instructions}
pub use create_post::*;
pub use read_post::*;
// sunscreen:end
```

A marker is a *region* delimited by two comment lines:

- `// sunscreen:begin {generator=<tag>}` — opens the region; carries the generator tag (`dispatch`, `instructions`, `account`, `event`, `error`, `error_variants`).
- `// sunscreen:end` — closes the region.

Sunscreen edits **only the content between** the markers. Everything outside is yours.

## Why markers exist

When you re-run `sunscreen scaffold instruction Foo`, sunscreen needs to:

1. Create a new file (`foo.rs`).
2. Edit `instructions/mod.rs` to re-export from it.
3. Edit `lib.rs` to register the dispatcher.

Marker regions tell sunscreen *where* to inject in step 2 and 3, without re-parsing your whole file with `syn`.

## Generator tags

| Tag | Lives in | Content |
|-----|----------|---------|
| `dispatch` | `lib.rs` | `pub fn <ix>(...) -> Result<()> { ... }` wrappers |
| `instructions` | `instructions/mod.rs` | `pub use <module>::*;` re-exports |
| `state` | `state/mod.rs` | `pub mod <account>;` declarations |
| `events` | `events.rs` | `pub use <event>::*;` re-exports |
| `errors` | `errors.rs` | `pub use <error>::*;` re-exports |
| `error_variants` | inside an error enum | enum variants like `#[msg("…")] Foo,` |

## Editing rules

You **may** edit outside markers. Sunscreen will not touch your changes.

You **may not** rely on hand-edits *inside* markers surviving a regeneration. The content of markers is regenerated from sunscreen's templates.

If you absolutely need a hand-customized version of generated code, copy it out of the marker into the unmanaged area below it.

## Drift and repair

Sometimes a file gets into a state sunscreen can't safely edit (you deleted a marker; the file was scaffolded by an older sunscreen version; merge conflict left stray markers). Detect and repair:

```bash
sunscreen chain doctor
sunscreen chain doctor --fix-markers
```

`--fix-markers` only rebuilds markers when it can do so *safely*:

| Site | Repair behaviour |
|------|------------------|
| `instructions/mod.rs` `instructions` marker missing | reconstruct, listing files present in `instructions/` |
| `lib.rs` `dispatch` marker missing | reconstruct **only** if all generated instruction files define `pub fn handler` (else refuse) |
| `errors.rs` `error_variants` marker missing | reconstruct **only** if the enum body is empty or clearly delimited (else refuse) |

When `--fix-markers` refuses, you get an exit `4` with a description of the ambiguity and a manual fix path.

## Implementation notes (for the curious)

Sunscreen's marker engine lives in [`src/rustpatch/`](https://github.com/Pantani/sunscreen/tree/main/src/rustpatch). It uses line-based parsing (not AST) for resilience — even malformed Rust around the markers is fine as long as the marker pair is intact.

## See also

- [Incremental scaffolding](../concepts/incremental-scaffolding.md) — the mental model.
- [`chain doctor`](./cli/chain.md#doctor) — the repair command.
