# `scaffold`

Add code to an existing workspace. Primitives + recipes.

```
sunscreen scaffold <NOUN> <NAME> [FLAGS]
```

All scaffold operations are **idempotent** (re-running with the same args is a no-op) and **marker-aware** (writes happen inside marker regions; hand-edits outside markers survive). See [Marker protocol](../markers.md).

## Common flags

| Flag | Default | Description |
|------|---------|-------------|
| `--program <name>` | required for most | which program to scaffold into |
| `--dry-run` | off | print planned diffs without writing |
| `--json` | off | machine-readable summary |
| `--force` | off | overwrite marker content even if a symbol exists |

## Primitives

### `instruction <Name>`

Add an instruction handler.

```bash
sunscreen scaffold instruction CreatePost --program blog
```

Creates `programs/blog/src/instructions/create_post.rs`, registers it in `lib.rs` dispatch, and updates `instructions/mod.rs`.

### `account <Name>`

Add an account struct.

```bash
sunscreen scaffold account Post --program blog \
  --fields "title:string,body:string,author:pubkey"
```

`--fields` syntax: `name:type[,name:type…]`. See [CRUD recipe](../recipes/crud.md#field-types) for accepted types.

### `event <Name>`

Add an event.

```bash
sunscreen scaffold event PostCreated --program blog \
  --fields "post:pubkey,author:pubkey,timestamp:i64"
```

### `error <Name>`

Add an error variant.

```bash
sunscreen scaffold error PostNotFound --program blog \
  --message "Post not found"
```

### `program <Name>`

Add a *new program* to an existing workspace.

```bash
sunscreen scaffold program governance
```

Creates `programs/governance/`, updates `Cargo.toml` workspace members, `Anchor.toml` `[programs.localnet]`, and `sunscreen.yml`.

## Recipes

Composite scaffolds that combine primitives:

| Noun | What it scaffolds |
|------|-------------------|
| [`crud <Name>`](../recipes/crud.md) | account + 4 instructions + 3 events + 2 errors + TS test |
| [`spl-token <Name>`](../recipes/spl-token.md) | SPL Token mint/transfer slice |
| [`metaplex-nft <Name>`](../recipes/metaplex-nft.md) | Metaplex NFT mint with metadata + master edition |

Each recipe runs a dry-run preflight: if any of its constituent primitives would conflict, it bails before writing.

## Plugin-provided nouns

When a plugin declares `scaffold <noun>` commands, sunscreen routes `sunscreen scaffold <noun>` through that plugin. List available plugin scaffolds:

```bash
sunscreen app commands --filter scaffold
```

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `2` | toolchain missing (rare; only for recipes that compile-check) |
| `3` | invalid config |
| `4` | symbol already exists, ambiguous flag, or recipe preflight failure |
| `5` | not in a workspace |
| `9` | plugin runtime failure (plugin-routed nouns) |
