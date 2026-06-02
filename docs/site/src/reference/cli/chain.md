# `chain`

Workspace lifecycle: create, build, serve, doctor.

## `new`

```
sunscreen chain new <NAME> [FLAGS]
```

Create a new workspace at `./<NAME>/`.

| Flag | Default | Description |
|------|---------|-------------|
| `--framework <name>` | `anchor` | `anchor` or `pinocchio` |
| `--frontend <name>` | `none` | `none`, `react`, `solid` |
| `--programs <list>` | `<name>` | comma-separated list of program names |
| `--license <spdx>` | `MIT OR Apache-2.0` | SPDX expression for generated `Cargo.toml` |
| `--git/--no-git` | `--git` | init a git repo with first commit |
| `--dry-run` | off | print planned files without writing |
| `--json` | off | machine-readable summary |

**Exit codes:** `0` ok · `2` toolchain · `4` directory exists.

**Examples:**

```bash
sunscreen chain new my-app
sunscreen chain new my-app --framework anchor --frontend react
sunscreen chain new bare --framework pinocchio
sunscreen chain new multi --programs "core,governance,treasury"
```

## `build`

```
sunscreen chain build [FLAGS]
```

Run the build pipeline: `anchor build` (or `cargo build-sbf` for Pinocchio), then Codama client regeneration (if frontend is configured).

| Flag | Default | Description |
|------|---------|-------------|
| `--no-codama` | off | skip Codama regeneration |
| `--headless` | off | NDJSON events on stdout, no TUI |
| `--release` | off | release profile build |
| `--json` | off | one summary object at the end |

**Exit codes:** `0` ok · `2` toolchain missing · `5` no workspace · build-tool exit codes preserved on failure.

**NDJSON events:**

```json
{"event":"build_start","framework":"anchor","programs":["my_app"]}
{"event":"build_progress","step":"anchor_build"}
{"event":"build_ok","programs":["my_app"],"duration_ms":4200}
{"event":"codama_start","frontend":"react"}
{"event":"codama_ok","files_written":12}
{"event":"frontend_notified","path":"app/.sunscreen/reload"}
```

Full event list in [NDJSON events](../events.md).

## `serve`

```
sunscreen chain serve [FLAGS]
```

Long-running supervised dev loop: validator + watcher + build pipeline + frontend notify.

| Flag | Default | Description |
|------|---------|-------------|
| `--validator <name>` | auto | `surfpool`, `test-validator`, or omit for auto-detect with fallback |
| `--no-codama` | off | skip Codama on rebuild |
| `--headless` | off | NDJSON stream, no TUI |
| `--rpc-port <n>` | `8899` | bind validator RPC port |
| `--ws-port <n>` | `8900` | bind validator WS port |
| `--quiet` | off | suppress validator stdout in TUI |

**Exit codes:** `0` ok (Ctrl-C) · `2` toolchain · `5` no workspace · `1` unexpected.

**Termination:** Ctrl-C sends SIGTERM to the validator's process group, waits up to 5s, then SIGKILL.

## `doctor`

```
sunscreen chain doctor [FLAGS]
```

Diagnose toolchain *and* workspace markers.

| Flag | Default | Description |
|------|---------|-------------|
| `--fix-markers` | off | reconstruct safe non-appendable markers (see [Marker protocol](../markers.md)) |
| `--json` | off | flat array of `ToolReport` objects |

Calls the same toolchain detectors as the top-level `sunscreen doctor`, plus marker integrity over your workspace.

**Exit codes:** `0` ok · `2` something critical missing · `4` non-fixable marker drift.

See also: [`doctor`](./doctor.md) for the toolchain-only command.
