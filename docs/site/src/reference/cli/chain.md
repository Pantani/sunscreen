# `chain`

Workspace lifecycle: create, build, serve, doctor.

`sunscreen chain deploy` is a stub today (Phase 2+). For real deploys use the top-level [`sunscreen deploy`](./onboarding.md#deploy).

## `new`

```
sunscreen chain new <NAME> [FLAGS]
```

Create a new workspace at `./<NAME>/` (or at `--path` if provided).

| Flag | Default | Description |
|------|---------|-------------|
| `--framework <name>` | `anchor` | `anchor` or `pinocchio` |
| `--frontend <name>` | `none` | `none`, `vite`, `next` |
| `--path <DIR>` | `./<NAME>` | output directory |
| `--dry-run` | off | print the planned file list without writing |

Global flags (apply to every subcommand): `--json`, `--verbose`/`-v`, `--workdir <DIR>`, `--config <FILE>`.

**Exit codes:** `0` ok · `2` toolchain missing · `4` directory exists / invalid argument.

**Examples:**

```bash
sunscreen chain new my-app
sunscreen chain new my-app --framework anchor --frontend vite
sunscreen chain new bare --framework pinocchio
sunscreen chain new my-app --frontend next --path packages/program
```

## `build`

```
sunscreen chain build [FLAGS]
```

Run the build pipeline: `anchor build` (or `cargo build-sbf` for Pinocchio), then Codama client regeneration when the workspace has a frontend.

| Flag | Default | Description |
|------|---------|-------------|
| `--headless` | off | NDJSON events on stdout suitable for CI logs |
| `--no-codama` | off | skip Codama regeneration after a successful build |

**Exit codes:** `0` ok · `2` toolchain missing · `5` no workspace · build-tool exit codes preserved on failure.

**NDJSON events** (selection — full list in [NDJSON events](../events.md)):

```json
{"event":"build_start","framework":"anchor","programs":["my_app"]}
{"event":"build_progress","step":"anchor_build"}
{"event":"build_ok","programs":["my_app"],"duration_ms":4200}
```

## `serve`

```
sunscreen chain serve [FLAGS]
```

Long-running supervised dev loop: validator + watcher + build pipeline + frontend notify.

| Flag | Default | Description |
|------|---------|-------------|
| `--headless` | off | NDJSON stream, no TUI |
| `--no-codama` | off | skip Codama on rebuild |
| `--no-frontend` | off | skip frontend reload notifications |
| `--runtime <name>` | from `sunscreen.yml` | `surfpool` or `test-validator`; defaults to the workspace's `runtime.engine`, falling back to `solana-test-validator` when Surfpool is requested but missing |
| `--debounce-ms <N>` | `150` | watcher debounce window |

**Exit codes:** `0` ok (Ctrl-C) · `2` toolchain · `5` no workspace · `4` invalid arg (e.g. `--debounce-ms 0`).

**Termination:** Ctrl-C sends SIGTERM to the validator's process group, waits, then SIGKILL.

## `doctor`

```
sunscreen chain doctor [--fix-markers]
```

Workspace-level diagnostic: marker integrity across scaffolded files.

| Flag | Default | Description |
|------|---------|-------------|
| `--fix-markers` | off | reconstruct safe non-appendable markers (see [Marker protocol](../markers.md)) |

For toolchain diagnostics see the top-level [`doctor`](./doctor.md) command.

**Exit codes:** `0` ok · `4` non-fixable marker drift detected.
