# NDJSON events

When you pass `--headless` (or `--json` to streaming commands), sunscreen emits **one JSON object per line** on stdout. Use this for editor integrations, CI parsers, and dashboards.

## Event envelope

Every event has at least:

```json
{
  "event": "<name>",
  "ts": "2026-06-02T14:33:12.412Z"
}
```

Plus event-specific fields.

## Build pipeline events

Emitted by `chain build` and the build phase of `chain serve`.

| Event | When | Fields |
|-------|------|--------|
| `build_start` | pipeline starts | `framework`, `programs[]` |
| `build_progress` | step transition | `step` (`anchor_build` / `cargo_build_sbf` / `codama`) |
| `build_ok` | pipeline succeeded | `programs[]`, `duration_ms` |
| `build_fail` | pipeline failed | `step`, `exit_code`, `stderr_tail` |

## Codama events

| Event | When | Fields |
|-------|------|--------|
| `codama_start` | Codama invocation begins | `frontend` |
| `codama_ok` | clients written | `files_written` (int), `duration_ms` |
| `codama_fail` | Codama failed | `exit_code`, `stderr_tail` |

## Frontend notify

| Event | When | Fields |
|-------|------|--------|
| `frontend_notified` | reload file touched | `path` |

## Serve / watcher events

Emitted by `chain serve --headless`.

| Event | When | Fields |
|-------|------|--------|
| `serve_start` | server ready | `rpc_url`, `ws_url`, `validator` |
| `validator_log` | validator wrote a line | `line` (string) |
| `watcher_batch` | file changes debounced | `paths[]` (relative), `kind` (`save`/`delete`) |
| `pipeline_triggered` | watcher kicked a build | `paths[]` |
| `serve_stop` | shutdown complete | `reason` (`ctrl_c`/`error`) |

## Toolchain events

Emitted by any command that reaches a missing toolchain.

| Event | Fields |
|-------|--------|
| `toolchain_missing` | `tool`, `next_step` |

## Stability

The set of events listed here is part of sunscreen's stable API. Adding new events is non-breaking. Removing or renaming events is a breaking change handled via the SemVer policy in [Roadmap](../contributing/roadmap.md).

## Parsing tips

- Treat unknown events as informational. Do not fail on new event names.
- `event` is always a stable string. Use it as the discriminator.
- Stdout is line-buffered; partial lines mean the process is still writing — wait for `\n`.
- Human messages may still hit stderr — don't merge stderr into your parser.
