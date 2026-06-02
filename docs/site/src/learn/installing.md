# Installing

⏱ 3 min · 🎯 you'll have: `sunscreen --version` working in your terminal.

Sunscreen ships as a single binary. Three install paths, in order of recommendation.

## Pre-requisites

You need a recent stable Rust toolchain on your PATH. If you don't have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The other Solana tools (`anchor`, `solana`, `cargo-build-sbf`, `pnpm`, …) are checked at runtime by `sunscreen doctor`. You only need them when you actually run the corresponding command.

## Option 1 — installer script (recommended)

The fastest path. Works on macOS (x86_64, aarch64) and Linux (x86_64, aarch64).

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Pantani/sunscreen/releases/download/v0.1.0/sunscreen-installer.sh \
  | sh
```

The script installs to `~/.cargo/bin/sunscreen`. Restart your shell or `source ~/.cargo/env` so the binary is on PATH.

## Option 2 — `cargo install`

If you already have a Rust toolchain and prefer to build locally:

```bash
cargo install --git https://github.com/Pantani/sunscreen --tag v0.1.0 sunscreen
```

This compiles from source (~2 minutes on a modern laptop) and places the binary in `~/.cargo/bin`.

## Option 3 — download the archive

Releases include pre-built tarballs and zips:

1. Open <https://github.com/Pantani/sunscreen/releases/latest>.
2. Download the archive matching your platform (`sunscreen-aarch64-apple-darwin.tar.xz`, `sunscreen-x86_64-unknown-linux-gnu.tar.xz`, …).
3. Extract and move `sunscreen` into a directory on your `$PATH`.

## Verify

```bash
sunscreen --version
# sunscreen 0.1.0
```

Run the doctor to see which Solana tools sunscreen detected:

```bash
sunscreen doctor
```

You'll see a table. Anything reported `missing` will be required *only* when you run a command that needs it. For example, you can `chain new` without `anchor` installed, but you'll need `anchor` to `chain build`.

## Updating

- Installer script: re-run with the new version URL.
- `cargo install`: re-run with `--force` and the new `--tag`.
- Archive: download a new tarball.

## Uninstall

```bash
rm "$(which sunscreen)"
```

Sunscreen does not write to global directories outside the binary itself. Per-workspace state lives in your project folder.

## Next

- [Create your first workspace](./first-workspace.md).
