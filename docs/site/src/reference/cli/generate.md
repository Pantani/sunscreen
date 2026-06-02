# `generate`

Generate artifacts from the IDL.

```
sunscreen generate <ARTIFACT> [FLAGS]
```

`generate` is implicitly called by `chain build` and `chain serve`. Use it directly when you want to regenerate without rebuilding the program.

## `clients`

```
sunscreen generate clients [FLAGS]
```

Run Codama against the workspace IDL and write a JavaScript/TypeScript client into `app/src/clients/` (or the path configured in `sunscreen.yml`).

| Flag | Default | Description |
|------|---------|-------------|
| `--rebuild-config` | off | rewrite `codama.config.mjs` from scratch (use when IDL shape changes drastically) |
| `--out <path>` | from config | client output directory |
| `--json` | off | summary on stdout |

**Requires:** `pnpm` on PATH (sunscreen uses `pnpm exec codama`).

## `idl`

```
sunscreen generate idl [FLAGS]
```

Export a deterministic IDL into `idl/`. Useful for CI artifacts and clients consumed outside Codama.

| Flag | Default | Description |
|------|---------|-------------|
| `--out <path>` | `idl/` | output directory |
| `--pretty` | on | format JSON with 2-space indent |

The exported IDL is byte-identical between runs as long as the source hasn't changed (sorted fields, normalized numeric types).

## `frontend-hooks`

```
sunscreen generate frontend-hooks [FLAGS]
```

Generate React Query or Solid Query hooks from the IDL.

| Flag | Default | Description |
|------|---------|-------------|
| `--framework <name>` | from `sunscreen.yml` | `react` or `solid` |
| `--out <path>` | `app/src/hooks/` | output directory |
| `--json` | off | summary on stdout |

For each instruction in the IDL, generates a hook (`useCreatePost`, `useReadPost`, …) wrapping the Codama client. The hook handles transaction building, signing, and refetching account queries.

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `2` | `pnpm` or required dependency missing |
| `3` | `sunscreen.yml` does not declare a frontend |
| `5` | not in a workspace |

## Tips

- `chain build` calls `generate clients` automatically. Run `generate` directly when you've edited the IDL by hand or need clients without rebuilding the `.so`.
- Re-runs are idempotent. Codama overwrites only files it owns; your hand-written code in `app/src/` is untouched.
