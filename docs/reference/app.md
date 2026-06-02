# `sunscreen app` — declarative plugin lifecycle

The `app` command group manages **declarations** of application plugins in
`sunscreen.yml` under the `plugins[]` array. It is a deliberately small,
offline MVP carved out of the deferred Phase 6 plugin runtime.

## Scope

`app` only edits the manifest. It does **not**:

- execute any external plugin process or entrypoint;
- expose gRPC or stdio JSON-RPC transports;
- contact a registry / marketplace;
- download remote artifacts;
- sandbox or supervise running plugins;
- register dynamic commands.

Every JSON success payload that describes a plugin therefore includes
`"status": "declared"` so callers can tell the difference between
"this plugin is recorded in the workspace manifest" and "this plugin is
running". The plugin runtime, registry, transports, sandbox, and
marketplace remain part of the deferred Phase 6 surface and have no code
landed yet.

## Subcommands

| Command | Effect |
|---|---|
| `app install <source>[@<version>] [--version V] [--dry-run]` | Add or update an entry. Idempotent — re-running with the same source/version is a no-op. |
| `app uninstall <name-or-source> [--dry-run]` | Remove an existing entry. |
| `app list` | Print all declared plugins. |
| `app describe <name-or-source>` | Print one declared plugin. |
| `app update <name-or-source> --version V [--dry-run]` | Change only the pinned version of an existing entry. `--version` is required — "latest registry lookup" is out of scope. |

### Resolving `<name-or-source>`

`uninstall`, `describe`, and `update` accept either:

- the exact `source` string as written in `sunscreen.yml`, or
- a normalized **basename**: the last `/`-separated segment with any
  trailing `.git` stripped. `github.com/org/foo.git` → `foo`.

If a basename matches multiple declared sources, the command exits **4
(`user_input`)** with a message naming both candidates — pass the exact
source to disambiguate.

### Version syntax

`--version` and the `@<version>` shorthand accept any value that
`semver::Version::parse` accepts, optionally prefixed with `v`. Examples:
`1.2.3`, `v0.1.0`, `1.0.0-alpha.1+meta`.

## JSON output shape

Every subcommand emits JSON on stdout when `--json` is passed at the root.
Success payloads share a common envelope:

```json
{
  "ok": true,
  "command": "app install",
  "config": "sunscreen.yml",
  "app":  { "name": "foo", "source": "github.com/org/foo.git", "version": "1.2.3", "status": "declared" },
  "apps": [ /* only present for `list` */ ],
  "changed": true,
  "dry_run": false
}
```

- `app` is present for `install`, `uninstall`, `describe`, `update`.
- `apps` is present for `list`.
- `changed` is `true` only when the manifest was actually rewritten —
  idempotent re-runs and `--dry-run` set it to `false`.
- `dry_run` mirrors the flag.

## Exit codes

`app` reuses the existing CLI taxonomy — there is **no** new
plugin-specific exit code.

| Exit | Kind | When |
|---|---|---|
| 0 | success | requested mutation applied, or `--dry-run` printed plan |
| 3 | `config_invalid` | `sunscreen.yml` parses but fails `Config::validate` (empty source, malformed version, duplicate normalized source) |
| 4 | `user_input` | missing `--version` on `update`; basename collision on `install` with a different source; ambiguous `<name-or-source>`; no match |
| 5 | `workspace_missing` | not inside a sunscreen workspace |

## Config-level validation

`Config::validate()` enforces:

- every plugin `source` is non-empty after trimming;
- each plugin `version` (when present) is valid semver with an optional
  leading `v`;
- no two entries share the same normalized (`trim` + `to_ascii_lowercase`)
  source.

These rules apply both when loading the manifest at startup and when
writing it back from an `app` subcommand, so an `install` that would
introduce drift fails before disk is touched.

## Out of scope

The following remain part of the deferred Phase 6 plugin work and are not
addressed by this MVP:

- gRPC / stdio JSON-RPC transports
- plugin entrypoint execution and dynamic command registration
- a plugin registry or marketplace
- mandatory remote artifact download
- sandboxing or supervisor lifecycle
- per-plugin permission scopes beyond the declared `source` + `version`

When Phase 6 lands, these capabilities are expected to layer on top of
the same `plugins[]` array — the declaration format and JSON shape
exposed today are intended to forward-compatibly host them.
