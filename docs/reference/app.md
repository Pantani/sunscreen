# `sunscreen app` — plugin lifecycle and runtime contract

The `app` command group manages application plugins declared in `sunscreen.yml`
under the `plugins[]` array and, in Phase 6, connects those declarations to a
supervised plugin runtime. The design follows ADR-0001 §7.5 and §10.8 with the
project stack adapted from `solis`/Go to `sunscreen`/Rust: a finalized gRPC
proto contract for compiled plugins, JSON-RPC over stdio for scripting
ecosystems, and the existing Rust config and subprocess boundaries for lifecycle
management.

## Workspace declarations

`plugins[]` is the source of truth for which plugins a workspace wants enabled:

```yaml
plugins:
  - source: github.com/sunscreen-apps/spl-token-2022
    version: v0.4.1
  - source: github.com/sunscreen-apps/yellowstone-indexer
    version: v0.2.0
  - source: ./plugins/local-transfer-hook
    version: 0.1.0
    manifest: ./plugins/local-transfer-hook/sunscreen-plugin.json
```

The existing lifecycle commands keep managing this list:

| Command | Effect |
|---|---|
| `app install <source>[@<version>] [--version V] [--dry-run]` | Add or update a plugin declaration. Local path manifests become available immediately; remote sources stay pinned declarations until resolved by marketplace/distribution tooling. |
| `app uninstall <name-or-source> [--dry-run]` | Remove a declaration and detach the runtime entry. |
| `app list` | Print declared plugins with lifecycle status. |
| `app describe <name-or-source>` | Print one plugin, including manifest, transport, capabilities, and install location when resolved. |
| `app update <name-or-source> --version V [--dry-run]` | Change the pinned version of a declaration. |
| `app commands` | List dynamic commands exported by available plugin manifests. |
| `app run <plugin> <command> -- [args...]` | Execute an `app`-kind plugin command through the selected transport. |
| `app hook <hook> -- [args...]` | Execute a lifecycle hook on every available plugin that declares it. |
| `app marketplace` | List built-in/reference marketplace entries and their declared transports/capabilities. |

`<name-or-source>` resolves first by exact `source`, then by normalized basename:
the last `/`-separated segment with trailing `.git` stripped
(`github.com/org/foo.git` -> `foo`). Ambiguous basename matches are a user input
error; pass the exact source.

Version values use `semver::Version::parse`, optionally prefixed with `v`.

## Runtime states

JSON success payloads keep the existing envelope and report the plugin state:

```json
{
  "ok": true,
  "command": "app describe",
  "config": "sunscreen.yml",
  "app": {
    "name": "spl-token-2022",
    "source": "github.com/sunscreen-apps/spl-token-2022",
    "version": "v0.4.1",
    "status": "available",
    "transport": "grpc",
    "capabilities": {
      "commands": ["scaffold transfer-hook"],
      "hooks": ["pre-build", "post-codama"]
    }
  },
  "changed": false,
  "dry_run": false
}
```

Status values are intentionally narrow:

| Status | Meaning |
|---|---|
| `declared` | Present in `sunscreen.yml`; no artifact has been resolved yet. |
| `installed` | Artifact is present in the local plugin cache or points to a local path. |
| `available` | Manifest is valid, engine range matches this `sunscreen`, and capabilities are accepted. |
| `running` | A supervised process is currently serving a command or hook. |
| `failed` | Manifest validation, transport handshake, sandbox setup, or plugin execution failed. |

`changed` is `true` only when `sunscreen.yml` or the resolved local artifact set
changed. `--dry-run` never writes the manifest or plugin cache.

## Plugin manifest

Each plugin root contains `sunscreen-plugin.json`. Remote marketplace plugins
must include it at the artifact root; local path plugins are validated in place.

```json
{
  "name": "spl-token-2022",
  "version": "0.4.1",
  "description": "Token-2022 extension recipes for sunscreen",
  "transport": "grpc",
  "entrypoint": ["spl-token-2022-plugin"],
  "engines": {
    "sunscreen": ">=0.6.0"
  },
  "commands": [
    {
      "name": "transfer-hook",
      "kind": "scaffold",
      "summary": "Scaffold a Token-2022 transfer hook"
    }
  ],
  "hooks": ["pre-build", "post-codama"],
  "capabilities": {
    "filesystem": ["workspace", "scratch"],
    "network": true
  }
}
```

Required fields: `name`, `version`, and `transport`. `entrypoint` is required
before a command or hook can run, and marketplace entries must declare
`engines.sunscreen` so incompatible artifacts can be rejected before install.
Capability declarations are also part of the trust model: the runtime rejects
undeclared command, hook, filesystem, and network requests; signing requests are
rejected in Phase 6. Dynamic command descriptors live in the top-level
`commands` array so `app commands` and plugin-backed `scaffold <noun>` routing
can inspect them without starting long-running hooks.

## Transport contract

Both transports expose the same logical operations through a single internal
adapter:

| Operation | Purpose |
|---|---|
| `initialize` | Negotiate protocol version, engine compatibility, workspace metadata, and accepted capabilities. |
| `capabilities` | Return command and hook descriptors used for dynamic registration. |
| `run_command` | Execute a plugin command such as `scaffold transfer-hook`. |
| `run_hook` | Execute lifecycle hooks such as `pre-build` or `post-codama`. |
| `shutdown` | Allow graceful process teardown before the supervisor kills the process group. |

### gRPC

The gRPC transport is defined by [`proto/plugin.proto`](../../proto/plugin.proto)
and targets a local supervisor-owned `tonic` endpoint. It is the default for
compiled plugins and performance-sensitive hooks because request and response
shapes are schema-checked. The endpoint is local-only; plugins do not open
public network listeners as part of the contract.

### stdio JSON-RPC

The stdio transport uses LSP-style framing:
`Content-Length: N\r\n\r\n{json}`. It is the default for TypeScript, Node,
Python, and other scripting ecosystems. Version negotiation happens during
`initialize`; a plugin that cannot satisfy the current protocol must fail before
registering commands.

## Sandbox and trust model

Plugins are executable code. `sunscreen` can supervise and constrain them, but
installation is still a trust decision by the user or workspace owner.

The Phase 6 sandbox rules are:

- Filesystem access is restricted to the workspace root and a per-plugin scratch
  directory under the local plugin cache.
- Writes outside marker-managed scaffold targets still go through the same
  transactional planning rules used by core scaffolders.
- Network access is denied unless the manifest sets `capabilities.network:
  true`; in the current Phase 6 host-process transports, `sunscreen` enforces
  that by refusing to spawn plugins that omit the capability, because local
  stdio/gRPC processes cannot have host networking disabled portably yet.
- Signing is denied in Phase 6. Plugins must not read wallet files such as
  `~/.config/solana/id.json` directly; flows that require signatures must route
  through core `sunscreen wallet` / `sunscreen deploy` surfaces or a later ADR.
- The plugin environment is sanitized. Secrets, SSH agents, Solana key paths,
  and user shell startup state are not inherited unless a future capability
  explicitly allows them.
- Every plugin process is launched and stopped through the runtime supervisor so
  Ctrl-C and failure paths tear down the full process group.

Rejected by design: unbounded filesystem access, implicit private-key access,
background daemons that survive the parent `sunscreen` process, and dynamic
commands that mutate core CLI state without a declared manifest capability.

## Marketplace and reference plugins

`app install` accepts three source classes:

| Source | Resolution |
|---|---|
| Local path, for example `./plugins/foo` | Validate `sunscreen-plugin.json` in place; no network or copy required. |
| Repository URL, for example `github.com/org/foo.git` | Record a pinned declaration. Remote resolution/download is handled by distribution tooling outside the offline runtime tests. |
| Marketplace shorthand, for example `sunscreen-apps/spl-token-2022` | Record a pinned declaration against the documented marketplace index. |

The Phase 6 reference set is:

| Plugin | Transport | Capability |
|---|---|---|
| `sunscreen-apps/spl-token-2022` | gRPC | Token-2022 recipes including transfer-hook and confidential-transfer scaffolding. |
| `sunscreen-apps/yellowstone-indexer` | stdio JSON-RPC | Yellowstone/Vixen indexer scaffolding derived from Anchor IDLs. |

Marketplace entries must publish source, version, checksum, manifest summary,
transport, and declared capabilities. Local path plugins are never treated as
marketplace plugins and are not downloaded.

## Exit codes

`app` reuses the existing CLI taxonomy.

| Exit | Kind | When |
|---|---|---|
| 0 | success | requested lifecycle operation succeeded, or `--dry-run` printed a plan |
| 3 | `config_invalid` | `sunscreen.yml` or `sunscreen-plugin.json` is syntactically valid but semantically invalid |
| 4 | `user_input` | missing required version, ambiguous plugin target, refused capability, or incompatible engine range |
| 5 | `workspace_missing` | not inside a sunscreen workspace |
| 7 | `path_conflict` | plugin or lifecycle command would overwrite an existing path that is not safely owned |
| 9 | `plugin_runtime` | plugin process crashed, failed handshake, violated sandbox, or returned a transport error |

Plugin runtime failures use a dedicated exit code so they do not collide with
the existing `path_conflict` contract.

## Out of scope

Phase 6 does not make arbitrary plugin code trusted. It also does not allow
plugins to bypass marker ownership, mutate generated Codama artifacts directly,
or replace the core Anchor/Codama/Surfpool execution model. Plugins extend the
same scaffold, generate, and runtime hooks that core uses; they do not become a
second unsupervised CLI.
