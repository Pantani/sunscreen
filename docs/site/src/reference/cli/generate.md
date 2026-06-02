# `generate`

Generate artifacts from the IDL.

```
sunscreen generate <ARTIFACT> [FLAGS]
```

`generate clients` is implicitly called by `chain build` and `chain serve`. Use these subcommands directly when you want to regenerate without rebuilding the program.

## `clients`

```
sunscreen generate clients [--program <NAME>]
```

Run Codama against the workspace IDL and write a JavaScript/TypeScript client.

| Flag | Default | Description |
|------|---------|-------------|
| `--program <NAME>` | first IDL | program to regenerate clients for |

**Requires:** `pnpm` on PATH (sunscreen drives Codama via `pnpm exec codama`).

## `idl`

```
sunscreen generate idl [--program <NAME>] [--out-dir <DIR>]
```

Export a deterministic IDL into the workspace.

| Flag | Default | Description |
|------|---------|-------------|
| `--program <NAME>` | all built IDLs in `target/idl` | program to export |
| `--out-dir <DIR>` | `clients/idl` | output directory relative to the workspace root |

The exported IDL is byte-stable between runs as long as the source hasn't changed.

## `frontend-hooks`

```
sunscreen generate frontend-hooks [FLAGS]
```

Generate TanStack Query hooks from exported IDLs.

| Flag | Default | Description |
|------|---------|-------------|
| `--program <NAME>` | all built IDLs | program to generate hooks for |
| `--frontend-path <DIR>` | from `sunscreen.yml` | required when the workspace was scaffolded with `--frontend none` |
| `--target <NAME>` | from project config | `react`, `solid`, or `all` (React Query + Solid Query wrappers) |

For each instruction in the IDL, generates a hook (`useCreatePost`, `useReadPost`, …) wrapping the Codama client. Hooks handle transaction building, signing, and refetching account queries on mutation success.

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `2` | `pnpm` or another required dependency missing |
| `3` | `sunscreen.yml` invalid |
| `5` | not in a workspace |

## Tips

- `chain build` calls `generate clients` automatically. Run `generate` directly when you've touched the IDL by hand or need clients without rebuilding the `.so`.
- Re-runs are idempotent. Codama overwrites only files it owns.
