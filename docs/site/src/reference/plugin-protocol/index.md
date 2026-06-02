# Plugin protocol

Sunscreen plugins are external binaries that speak two transport layers:

- **stdio JSON-RPC** (default, used today).
- **gRPC** (defined in `proto/plugin.proto`, planned for richer plugins).

## Manifest

Each plugin ships a `plugin.toml` next to its binary:

```toml
[plugin]
name = "yellowstone-indexer"
version = "0.3.0"
description = "Scaffold a Yellowstone gRPC indexer slice."
entry = "./bin/plugin"

[permissions]
workspace_write = true       # default: false
network = false              # if true, prompts user at install time
declared_paths = []          # extra read-only paths outside workspace

[commands.scaffold]
indexer = "Scaffold a Yellowstone indexer slice"
listener = "Scaffold an event listener"

[commands.hook]
after_build = "Update indexer config when IDL changes"
```

## JSON-RPC methods

Sunscreen calls these on the plugin via stdin/stdout. One JSON object per line.

### `commands`

Request:
```json
{"jsonrpc":"2.0","id":1,"method":"commands"}
```

Response:
```json
{"jsonrpc":"2.0","id":1,"result":{
  "scaffold": {"indexer": "…", "listener": "…"},
  "hook": {"after_build": "…"}
}}
```

### `run`

Request:
```json
{"jsonrpc":"2.0","id":2,"method":"run","params":{
  "command": "scaffold:indexer",
  "args": ["Trades"],
  "flags": {"program": "app"},
  "workspace_root": "/abs/path/to/workspace",
  "config": {"version": 1, "name": "my-app", "..." : "..."}
}}
```

Response (success):
```json
{"jsonrpc":"2.0","id":2,"result":{
  "files_written": ["programs/app/src/instructions/index_trades.rs", "..."],
  "summary": "scaffolded indexer Trades"
}}
```

Response (error):
```json
{"jsonrpc":"2.0","id":2,"error":{
  "code": 4,
  "message": "instruction index_trades already exists",
  "data": {"next_step": "pass --force or pick a different name"}
}}
```

### `hook`

Request:
```json
{"jsonrpc":"2.0","id":3,"method":"hook","params":{
  "name": "after_build",
  "context": {"workspace_root": "...", "idl": {...}}
}}
```

Response: same shape as `run`.

## Sandbox

Sunscreen enforces:

| Capability | Default | Toggle |
|------------|---------|--------|
| Read workspace files | allowed | always on |
| Write workspace files | denied | `permissions.workspace_write = true` |
| Read outside workspace | denied | declare each path in `permissions.declared_paths` |
| Network access | denied | `permissions.network = true` (user-approved at install) |
| Spawn subprocesses | denied | not toggleable in current version |

Sandbox violations terminate the session and return exit `9` to the caller.

## gRPC contract

`proto/plugin.proto` (in the sunscreen repo) defines the streaming-friendly equivalent of the JSON-RPC surface. Use when:

- Plugin needs bidirectional streaming (live progress, log forwarding).
- Plugin is implemented in a non-stdio-friendly runtime (e.g. JVM).

The gRPC transport is wire-defined but not yet end-to-end implemented in the runtime.

## Conformance tests

Reference plugins under [`sunscreen-apps/`](https://github.com/Pantani/sunscreen-apps) exercise the full protocol surface and serve as templates. The `spl-token-2022` plugin is the canonical example.
