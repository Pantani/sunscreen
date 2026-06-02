# `doctor`

Detect installed toolchain versions.

```
sunscreen doctor [--json]
```

Outputs a table of tools sunscreen knows how to detect, with their installed version and availability flag.

## Detected tools

| Tool | Detected via |
|------|--------------|
| `rustc` | `rustc --version` |
| `cargo` | `cargo --version` |
| `anchor` | `anchor --version` |
| `solana` | `solana --version` |
| `cargo-build-sbf` | `cargo build-sbf --help` |
| `pnpm` | `pnpm --version` |
| `node` | `node --version` |
| `codama` | from local `node_modules/.bin/codama` |
| `surfpool` | `surfpool --version` |

If a tool is missing, sunscreen reports `available: false` and a `next_step` hinting installation. Missing tools are only blocking when you actually run a command that needs them.

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

## Exit codes

| Code | When |
|------|------|
| `0` | always — `doctor` reports, it does not fail on missing tools. Inspect `available` per row. |

For workspace-marker diagnostics, see [`chain doctor`](./chain.md#doctor).
