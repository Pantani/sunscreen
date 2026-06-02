# Developer setup

How to clone, build, and test sunscreen itself.

## Pre-requisites

- Rust stable (matches `rust-toolchain.toml` if present).
- For the integration tests that exercise Anchor: `anchor`, `solana`, `cargo-build-sbf`, `pnpm`, Node 18+.
- The CI runner does *not* need Anchor/Solana — Anchor-touching tests are `#[ignore]` by default and explicitly gated.

## Clone and build

```bash
git clone https://github.com/Pantani/sunscreen.git
cd sunscreen
cargo build --locked --release --all-features
```

The release binary lands at `target/release/sunscreen`. Symlink it onto your PATH if you want to use the locally-built version.

## Run the test suite

```bash
# Fast unit + golden + integration smoke (no Anchor toolchain required)
cargo test --locked --all --all-features --no-fail-fast

# Feature-gate check (`--no-default-features` must compile)
cargo check --locked --no-default-features --all-targets

# Lints
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## Anchor-touching tests

These are ignored by default. To run them you need the full Solana stack:

```bash
cargo test --locked --test integration_anchor -- --include-ignored
```

If your toolchain is missing parts, the tests skip with a clear message rather than fail.

## Editor

Any editor that speaks `rust-analyzer` works. The repo includes no editor-specific config — open the workspace root and rust-analyzer figures out the rest.

## Pre-commit

The repo expects `cargo fmt` + `cargo clippy --no-warnings` before commits. CI fails the PR otherwise.

## Where to start

- [`src/cli/`](https://github.com/Pantani/sunscreen/tree/main/src/cli) — to add a flag or subcommand.
- [`src/templates/`](https://github.com/Pantani/sunscreen/tree/main/src/templates) + [`templates/`](https://github.com/Pantani/sunscreen/tree/main/templates) — to change scaffold output.
- [`src/runtime/`](https://github.com/Pantani/sunscreen/tree/main/src/runtime) — for build/serve pipeline behavior.
- [`tests/golden/`](https://github.com/Pantani/sunscreen/tree/main/tests/golden) — snapshot tests for scaffolds.

## Docs

To preview this site locally:

```bash
cargo install mdbook mdbook-admonish mdbook-mermaid mdbook-linkcheck
mdbook serve docs/site --open
```

## Releases

Releases are tag-driven: push `vX.Y.Z`, the `release.yml` workflow builds and publishes binaries. The docs site auto-deploys from `main` via `.github/workflows/docs.yml`.
