# CRUD recipe

```
sunscreen scaffold crud <NAME> --program <PROGRAM> [FLAGS]
```

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--program <name>` | required | which program to scaffold into |
| `--fields <spec>` | `title:string,body:string` | account fields |
| `--frontend-hook` | off | also generate matching React/Solid hooks |
| `--dry-run` | off | print plan only |
| `--json` | off | machine-readable summary |
| `--force` | off | overwrite marker content even if a symbol exists |

## Field types

`--fields` accepts comma-separated `name:type` pairs:

| Spec | Anchor type | Notes |
|------|-------------|-------|
| `name:string` | `String` | UTF-8, max 256 bytes by default |
| `name:bool` | `bool` | |
| `name:u8` … `name:u128` | `u8` … `u128` | |
| `name:i8` … `name:i128` | `i8` … `i128` | |
| `name:pubkey` | `Pubkey` | |
| `name:vec<u8>` | `Vec<u8>` | max 256 bytes |
| `name:option<u64>` | `Option<u64>` | optional fields |

Types outside this set require hand-editing the generated account.

## Generated files

For `scaffold crud Post --program blog`:

| Path | Status |
|------|--------|
| `programs/blog/src/state/post.rs` | created |
| `programs/blog/src/state/mod.rs` | patched (marker) |
| `programs/blog/src/instructions/create_post.rs` | created |
| `programs/blog/src/instructions/read_post.rs` | created |
| `programs/blog/src/instructions/update_post.rs` | created |
| `programs/blog/src/instructions/delete_post.rs` | created |
| `programs/blog/src/instructions/mod.rs` | patched |
| `programs/blog/src/lib.rs` | patched (dispatch markers) |
| `programs/blog/src/events.rs` | patched (3 new variants) |
| `programs/blog/src/errors.rs` | patched (2 new variants) |
| `tests/post.spec.ts` | created |

## Generated instructions

| Instruction | Accounts | Effect |
|-------------|----------|--------|
| `create_post` | `[authority: Signer, post: Account<Post> (init), system_program]` | initializes a `Post` PDA seeded by authority + name |
| `read_post` | `[post: Account<Post>]` | view-only; emits `PostRead` if the read fee model is enabled (off by default) |
| `update_post` | `[authority: Signer, post: Account<Post> (mut, has_one = authority)]` | updates fields, emits `PostUpdated` |
| `delete_post` | `[authority: Signer, post: Account<Post> (close = authority, has_one = authority)]` | closes the account, emits `PostDeleted` |

PDA seeds: `["post", authority.key, name.as_bytes()]`.

## Generated events

- `PostCreated { post: Pubkey, author: Pubkey, timestamp: i64 }`
- `PostUpdated { post: Pubkey, timestamp: i64 }`
- `PostDeleted { post: Pubkey, timestamp: i64 }`

## Generated errors

- `PostNotFound` — account discriminator mismatch.
- `PostUnauthorized` — signer is not the `authority` on the account.

## Frontend hook (`--frontend-hook`)

For React + React Query:

```ts
useCreatePost({ authority, name, fields });
useReadPost({ post });
useUpdatePost({ authority, post, fields });
useDeletePost({ authority, post });
```

Hooks invalidate the relevant queries on mutation success. Generated in `app/src/hooks/post.ts`.

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `4` | preflight conflict (symbol already exists; pass `--force` to overwrite marker content) |
| `5` | not in a workspace |
