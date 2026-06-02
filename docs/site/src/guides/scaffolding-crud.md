# Scaffolding a CRUD resource

⏱ 6 min · 🎯 you'll have: a complete `Post` resource with create/read/update/delete instructions, events, errors, and TS test.

The CRUD recipe is the fastest way to add a new resource to an existing program. It composes the primitive scaffolders (`account`, `instruction`, `event`, `error`) into a coherent slice.

## Pre-requisites

- A workspace already created with `sunscreen chain new`.
- At least one program in `programs/`.

## Run

```bash
sunscreen scaffold crud Post --program my_app
```

Output (truncated):

```
✓ scaffolded account: Post
✓ scaffolded instructions: create_post, read_post, update_post, delete_post
✓ scaffolded events: PostCreated, PostUpdated, PostDeleted
✓ scaffolded errors: PostNotFound, PostUnauthorized
✓ tests/post.spec.ts
```

## What was generated

| File | Purpose |
|------|---------|
| `programs/my_app/src/state/post.rs` | `Post` account struct with `#[account]` |
| `programs/my_app/src/instructions/create_post.rs` | `create_post` handler |
| `programs/my_app/src/instructions/read_post.rs` | `read_post` handler |
| `programs/my_app/src/instructions/update_post.rs` | `update_post` handler |
| `programs/my_app/src/instructions/delete_post.rs` | `delete_post` handler |
| `programs/my_app/src/events.rs` (patched) | `PostCreated`, `PostUpdated`, `PostDeleted` events |
| `programs/my_app/src/errors.rs` (patched) | `PostNotFound`, `PostUnauthorized` variants |
| `tests/post.spec.ts` | TypeScript test scaffolding the four ops |

All writes happen inside marker regions. Hand-edit anywhere outside the markers and your changes survive future scaffolds.

## Options

```bash
sunscreen scaffold crud Post \
  --program my_app \
  --fields "title:string,body:string,author:pubkey,created_at:i64" \
  --frontend-hook \
  --json
```

| Flag | Default | What it does |
|------|---------|--------------|
| `--program <name>` | required | program to scaffold into |
| `--fields "<spec>"` | `title:string,body:string` | comma-separated `name:type` pairs for the account struct |
| `--frontend-hook` | off | also generate React Query hooks (requires `frontend: react` in `sunscreen.yml`) |
| `--dry-run` | off | print what would change without writing |
| `--json` | off | machine-readable summary |
| `--force` | off | overwrite existing conflicting symbols |

## Idempotency

Re-running the same command is a no-op:

```bash
sunscreen scaffold crud Post --program my_app
# error: account "Post" already exists in program "my_app" (exit 4)
```

Add `--force` if you really want to regenerate (will *not* clobber hand-edits outside markers).

## What "fields" supports

| Spec syntax | Anchor type |
|-------------|------------|
| `name:string` | `String` |
| `name:bool` | `bool` |
| `name:u64` (also `u8`, `u16`, `u32`, `u128`) | matching unsigned int |
| `name:i64` (also `i8`, `i16`, `i32`, `i128`) | matching signed int |
| `name:pubkey` | `Pubkey` |
| `name:vec<u8>` | `Vec<u8>` (limited to 256 bytes) |

Complex types (nested structs, large vectors) need manual editing.

## Build and test

```bash
sunscreen chain build
anchor test
```

The generated `tests/post.spec.ts` exercises each of the four operations end-to-end against a local validator.

## Going further

- [Recipe reference: CRUD](../reference/recipes/crud.md) — every flag, every generated file.
- [SPL Token recipe](../reference/recipes/spl-token.md) and [Metaplex NFT recipe](../reference/recipes/metaplex-nft.md) — other composite recipes.
- [Marker protocol](../reference/markers.md) — understand how regenerations stay safe.
