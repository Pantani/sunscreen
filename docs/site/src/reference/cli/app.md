# `app`

Manage plugins.

```
sunscreen app <SUBCOMMAND> [ARGS]
```

Plugins are declared in `sunscreen.yml` under `plugins:`. Most `app` subcommands edit that file declaratively.

## `install`

```
sunscreen app install <SOURCE> [--version <SEMVER>] [--dry-run] [--json]
```

Add a plugin entry to `sunscreen.yml`. **Idempotent**: re-running with the same source is a no-op.

| Arg / flag | Description |
|------------|-------------|
| `<SOURCE>` | local path (`./plugins/foo`) or git URL (`github.com/org/foo.git`) |
| `--version` | semver string; with or without `v` prefix |
| `--dry-run` | print planned change without writing |
| `--json` | `{"status": "declared"|"already_declared", ...}` |

Source normalization: `github.com/org/foo.git` → plugin name `foo`. Conflicting basenames → exit `4`.

## `uninstall`

```
sunscreen app uninstall <NAME>
```

Remove a plugin entry. Does not delete files from disk.

## `list`

```
sunscreen app list [--json]
```

Print declared plugins:

```
NAME       VERSION  SOURCE                STATUS
my-plugin  0.2.0    ./plugins/my-plugin   declared
```

## `describe`

```
sunscreen app describe <NAME> [--json]
```

Read the plugin manifest and report:

- name, version
- declared commands (with `scaffold:`, `hook:` namespacing)
- declared permissions (`needs_network`, `needs_workspace_write`, declared file paths)

## `update`

```
sunscreen app update <NAME> --version <SEMVER>
```

Bump the `version` field in `sunscreen.yml`.

## `commands`

```
sunscreen app commands [<NAME>] [--filter <scaffold|hook>] [--json]
```

List commands offered by one or all plugins.

## `run`

```
sunscreen app run <NAME> -- <PLUGIN_ARGS>...
```

Invoke the plugin's `run` JSON-RPC method with the trailing args. Sunscreen sets up the sandbox, opens stdio JSON-RPC, and proxies stdout/stderr.

## `hook`

```
sunscreen app hook <NAME> <HOOK_NAME> [--json]
```

Manually trigger a registered hook. Normally hooks fire automatically (e.g. `after_build` on `chain build`).

## `marketplace`

```
sunscreen app marketplace [--json]
```

List reference plugins maintained under `sunscreen-apps/`. Currently:

- `sunscreen-apps/spl-token-2022`
- `sunscreen-apps/yellowstone-indexer`

This is a static, embedded list. Remote registry support is planned.

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `3` | `sunscreen.yml` invalid |
| `4` | name conflict, missing source, version not semver |
| `5` | not in a workspace |
| `9` | plugin binary crashed, sandbox violation, or JSON-RPC protocol error |

See [Plugin protocol](../plugin-protocol/index.md) for the JSON-RPC wire format.
