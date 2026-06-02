# Codegen Reference

Phase 4 adds the managed codegen surface for Anchor IDLs, Codama clients, and frontend hooks.

## Generated Paths

| Path | Owner | Command |
|---|---|---|
| `codama.json` | sunscreen | `sunscreen generate clients`, `chain build`, `chain serve` |
| `clients/idl/*.json` | sunscreen | `sunscreen generate idl`, `sunscreen generate frontend-hooks` |
| `clients/js/src/generated/` | Codama | `sunscreen generate clients` |
| `app/src/generated/sunscreen/idl.ts` | sunscreen | `sunscreen generate frontend-hooks` |
| `app/src/generated/sunscreen/core.ts` | sunscreen | `sunscreen generate frontend-hooks` |
| `app/src/generated/sunscreen/react.ts` | sunscreen | `sunscreen generate frontend-hooks` for React frontends, or `--target react|all` |
| `app/src/generated/sunscreen/solid.ts` | sunscreen | `sunscreen generate frontend-hooks --target solid|all` |
| `app/src/generated/sunscreen/index.ts` | sunscreen | `sunscreen generate frontend-hooks` |

These files are fully generated and are overwritten only when the rendered bytes change. They do not use Rust marker comments; markers remain a Rust scaffolding contract documented in [`markers.md`](markers.md).

## Commands

### `sunscreen generate idl`

Copies built Anchor IDLs from `target/idl/*.json` into `clients/idl/*.json` using deterministic pretty JSON. Run `sunscreen chain build` first when the Anchor IDL does not exist yet.

Options:

- `--program <NAME>` exports one program IDL.
- `--out-dir <DIR>` changes the workspace-relative output directory. The default is `clients/idl`.

### `sunscreen generate clients`

Writes `codama.json` and runs:

```text
pnpm exec codama run --all --config codama.json
```

The managed config currently defines the JavaScript renderer:

```json
{
  "idl": "target/idl/<program>.json",
  "before": [],
  "scripts": {
    "js": {
      "from": "@codama/renderers-js",
      "args": ["clients/js/src/generated"]
    }
  }
}
```

If `target/idl` already contains an IDL, sunscreen uses that file stem. Otherwise it falls back to the first program in `sunscreen.yml`, or to `--program <NAME>`.

### `sunscreen generate frontend-hooks`

Exports IDLs if needed, then emits framework-agnostic IDL/core TypeScript plus TanStack React Query and Solid Query wrappers.

Options:

- `--program <NAME>` generates hooks for one program.
- `--frontend-path <DIR>` writes hooks into an explicit frontend root. This is required for workspaces created with `--frontend none`.
- `--target all|react|solid` selects hook files. For scaffolded Next.js and Vite React frontends, the default is `react`; pass `--target solid` or `--target all` only when the app installs Solid Query dependencies.

The generated `core.ts` includes `createSurfpoolRpc()` with the local endpoint `http://127.0.0.1:8899`, plus `getProgramAccounts()` for local Surfpool/test-validator reads. Instruction mutations are generated from IDL instruction names and argument lists; callers provide the transaction executor so wallet/client policy remains application-owned.

## Idempotency

All sunscreen-owned codegen files use byte-for-byte write guards. A second run with the same IDL and options reports an empty `changed_files` list under `--json` and does not rewrite files.

## Verification

Normal CI covers:

- `tests/codegen_codama.rs` for stable `codama.json` rendering.
- `tests/generate.rs` for CLI idempotency, Codama subprocess arguments, frontend hook shape, and `--frontend none` error handling.
- Runtime pipeline tests for the shared Codama wrapper used by `chain build` and `chain serve`.

The real Next.js typecheck is available as an ignored/gated test:

```text
SUNSCREEN_FRONTEND_COMPILE_TESTS=1 cargo test --test generate generated_frontend_hooks_typecheck_vanilla_next_project_when_dependencies_are_installed -- --ignored
```
