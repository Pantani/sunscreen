# Working with plugins

⏱ 7 min · 🎯 you'll: install a local plugin, list its commands, run one, and understand the trust model.

Plugins extend sunscreen's `scaffold <noun>` surface. They are external binaries speaking sunscreen's JSON-RPC protocol over stdio. Use them to add team-specific scaffolds (custom CRUD shapes, internal protocol patterns, indexer hooks).

## Plugin model in one paragraph

A plugin is a binary that, when invoked, reads JSON-RPC requests from stdin and writes JSON-RPC responses to stdout. Sunscreen calls the plugin's `commands` method to learn what it offers, then `run` when the user invokes one. Plugins are sandboxed: they can only write within the workspace and cannot reach the network unless declared.

## Discover

```bash
sunscreen app marketplace
```

Lists reference plugins (`sunscreen-apps/spl-token-2022`, `sunscreen-apps/yellowstone-indexer`) and any plugins you've already installed.

## Install a local plugin

Sunscreen reads plugins declared in `sunscreen.yml`:

```yaml
plugins:
  - source: ./plugins/my-plugin
    version: "0.2.0"
```

Or install via CLI (idempotent, writes to `sunscreen.yml`):

```bash
sunscreen app install ./plugins/my-plugin --version 0.2.0
```

After install:

```bash
sunscreen app list
# my-plugin  0.2.0  ./plugins/my-plugin  (status: declared)
```

`status: declared` means sunscreen knows about it. The first time you `run` a plugin command, sunscreen verifies the binary, reads its manifest, and starts a JSON-RPC session.

## List a plugin's commands

```bash
sunscreen app commands my-plugin
# scaffold:
#   indexer <Name>      Scaffold a Yellowstone indexer slice
#   listener <Name>     Scaffold an event listener
```

## Run a plugin command

```bash
sunscreen app run my-plugin -- scaffold indexer Trades
# or, when the plugin registers a top-level scaffold noun:
sunscreen scaffold indexer Trades --program my_app
```

## Trust and sandbox

Sunscreen's plugin runtime enforces:

- **Filesystem**: writes restricted to the workspace root and below. Reads outside the workspace are allowed only for plugin-declared paths in its manifest.
- **Network**: blocked by default. Plugins must declare network needs in their manifest (`needs_network: true`) and the user must approve at install time.
- **Subprocess**: blocked. Plugins cannot spawn child processes.

If a plugin violates the sandbox, sunscreen kills the session and exits with code `9` (`plugin_runtime`). See [Errors & exit codes](../reference/errors.md).

## Uninstall

```bash
sunscreen app uninstall my-plugin
```

Removes the entry from `sunscreen.yml`. Does not delete the plugin binary from disk (you do that manually).

## Update

```bash
sunscreen app update my-plugin --version 0.3.0
```

Bumps the `version` in `sunscreen.yml`.

## Hooks

Plugins can register hooks for build/serve events. Configure:

```yaml
plugins:
  - source: ./plugins/coverage
    version: "0.1.0"
    hooks:
      - after_build
```

After every `chain build`, sunscreen calls the plugin's `hook` method with the build context. Use this for coverage reports, telemetry, custom artifacts.

## Writing your own plugin

See the [Plugin protocol reference](../reference/plugin-protocol/index.md) for the JSON-RPC schema, manifest format, and gRPC contract.

## Going further

- [`app` reference](../reference/cli/app.md) — all subcommands.
- [Plugin protocol](../reference/plugin-protocol/index.md) — wire format.
- [Plugin runtime concepts](../concepts/plugin-runtime.md) — sandbox model.
