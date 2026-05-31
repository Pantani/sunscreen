# sunscreen template assets

All files in this directory are embedded into the `sunscreen` binary at
compile time via `rust-embed` and rendered through `minijinja`.

## Naming convention

`<output_path>.jinja` — the `.jinja` suffix is stripped when materialised on
disk. The remainder of the path (including subdirectories) becomes the output
location relative to the rendering root.

Examples:

| Template | Output |
|---|---|
| `version.txt.jinja` | `version.txt` |
| `src/lib.rs.jinja` | `src/lib.rs` |
| `README.md.jinja` | `README.md` |

## Context

Templates receive a `serde_json::Value` context (typically an object). Field
access uses standard Jinja syntax: `{{ name }}`, `{{ deps.tokio }}`.

For deterministic output of mappings, callers should use `IndexMap` (or
construct JSON with ordered keys) — minijinja preserves insertion order.

## Available filters

| Filter | Example input | Output |
|---|---|---|
| `pascal_case` | `my project` | `MyProject` |
| `camel_case` | `my project` | `myProject` |
| `snake_case` | `MyProject` | `my_project` |
| `kebab_case` | `MyProject` | `my-project` |
| `screaming_snake` | `MyProject` | `MY_PROJECT` |

Implementations are backed by the `heck` crate.

## Determinism guarantees

- No timestamps, hostnames, or other ambient state.
- Map iteration order is insertion order (use `IndexMap` upstream).
- All tests are snapshot-locked via `insta` under `tests/golden/`.
