# `doctor`

Detect installed toolchain versions.

```
sunscreen doctor [--json] [--component <NAME>] [--fix]
```

Outputs a table of tools sunscreen knows how to detect, with their installed version and availability flag. Use `--component <NAME>` to inspect a single tool.

`--fix` attempts automatic repairs for unavailable tools with known recipes, then runs detection again. Without `--component`, it repairs required tools only and skips optional tools. With `--component`, it targets that tool even when the tool is optional. Progress logs are printed to stderr, including each command sunscreen runs and the final re-check result.

## Detected tools

| Tool | Detected via |
|------|--------------|
| `anchor` | `anchor --version` |
| `solana` | `solana --version` |
| `rustc` | `rustc --version` |
| `cargo` | `cargo --version` |
| `node` | `node --version` |
| `pnpm` | `pnpm --version` |
| `codama` | `codama --version` |
| `surfpool` | `surfpool --version` |
| `rustfmt` | `rustfmt --version` |

If a tool is missing or below its minimum version, sunscreen reports `available: false`. Missing optional tools do not fail the plain diagnostic. Missing required tools make `doctor` exit `2`.

## Human output

```
TOOL              VERSION         STATUS
rustc             1.79.0          ok
cargo             1.79.0          ok
anchor            0.30.1          ok
solana            1.18.18         ok
cargo-build-sbf   1.18.18         ok
pnpm              9.4.0           ok
node              20.13.1         ok
codama            (not found)     missing
surfpool          (not found)     missing
```

With `--fix`, sunscreen prints a before table, progress logs, a fix summary, then an after table. Some upstream installers update shell profiles instead of the current process environment; in that case the fix result is `reload-shell` and sunscreen prints the exact PATH reload command to try next. If the installer ran but the binary still reports an unparsable or stale version, the result is `inspect`. If a downloader command fails, the result is `failed` and the curl/agave error is preserved in the logs.

The Solana/Agave repair recipe downloads the official installer with curl retries and HTTP/1.1 forced, which avoids a common transient `HTTP/2 stream ... INTERNAL_ERROR` failure mode while still surfacing a real download failure when the CDN or network is unavailable.

## `--json` output

A flat array of `ToolReport` objects:

```json
[
  {"tool":"rustc","version":"1.79.0","available":true,"next_step":null},
  {"tool":"anchor","version":"0.30.1","available":true,"next_step":null},
  {"tool":"codama","version":null,"available":false,"next_step":"pnpm add -D codama in your frontend, or sunscreen will install on demand"}
]
```

Use this in CI to assert your runner has the expected toolchain.

With `--fix`, `--json` emits an object so callers can compare before/after state. Progress logs still go to stderr so stdout remains parseable JSON:

```json
{
  "ok_before": false,
  "ok_after": true,
  "reports_before": [],
  "fixes": [
    {
      "name": "anchor",
      "required": true,
      "status": "fixed",
      "message": "tool is available after repair",
      "commands": [["cargo", "install", "--git", "https://github.com/solana-foundation/anchor", "avm", "--force"]]
    }
  ],
  "reports_after": []
}
```

## Exit codes

| Code | When |
|------|------|
| `0` | all required tools are available; for `--fix`, requested repairs succeeded |
| `2` | a required tool remains unavailable, or a requested repair failed |

For workspace-marker diagnostics, see [`chain doctor`](./chain.md#doctor).
