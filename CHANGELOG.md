# Changelog

All notable changes to `sunscreen` are documented here.

The project follows Semantic Versioning. During the `0.x` preview line, CLI flags,
generated file layouts, and plugin protocol details may still change before
`1.0.0`; breaking changes will be called out in this file.

## [0.1.0] - 2026-06-02

### Added

- First public preview release of the Rust `sunscreen` CLI.
- Anchor workspace bootstrap via `sunscreen chain new`, including multi-program
  templates and optional Next.js/Vite/headless frontend variants.
- Pinocchio workspace bootstrap via `sunscreen chain new --framework pinocchio`,
  with `cargo build-sbf` build routing and clear guards for Anchor-only commands.
- Marker-based scaffolders for programs, instructions, accounts, events, errors,
  and safe marker repair through `sunscreen chain doctor --fix-markers`.
- Composite recipes for CRUD, SPL token, and Metaplex NFT slices.
- Runtime orchestration through `sunscreen chain build --headless` and
  `sunscreen chain serve --headless`, including watcher debounce, Surfpool or
  `solana-test-validator` supervision, Codama regeneration, and frontend reload
  notification.
- Code generation commands for deterministic IDL export, Codama client
  generation, and React/Solid Query frontend hooks.
- Beginner onboarding commands: `init`, `quickstart`, `examples`, `wallet`,
  `deploy`, and `learn`, with actionable `next_step` errors.
- Local plugin runtime MVP: plugin manifests, stdio JSON-RPC transport, gRPC
  proto contract, sandbox/trust boundaries, marketplace listing, lifecycle hooks,
  and plugin-backed `scaffold <noun>` routing.
- GitHub Actions release pipeline using `cargo-dist` to publish Linux x64,
  Linux ARM64, macOS x64, macOS ARM64 archives, checksums, and a shell installer
  as GitHub Release assets.

### Known limitations

- `sunscreen` is distributed through GitHub Releases only; crates.io, Homebrew,
  `cargo-binstall`, and Windows installers remain future Phase 8 work.
- Real Anchor/Codama/Solana integration tests are still gated by the host
  toolchain and skipped by default on generic CI runners.
- Pinocchio support is currently a bootstrap/build MVP; native Pinocchio
  scaffold/codegen flows are intentionally deferred.
