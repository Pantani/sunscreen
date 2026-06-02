# Pinocchio Workspaces

`sunscreen` supports a Phase 7 Pinocchio bootstrap MVP.

## Create a workspace

```bash
sunscreen chain new fast-program --framework pinocchio --frontend none
cd fast-program
```

The generated workspace contains:

- `programs/<name>/` — a minimal Pinocchio program crate
- `Cargo.toml` — workspace manifest
- `sunscreen.yml` — config with `project.framework: pinocchio`
- no `Anchor.toml`
- no `anchor-lang` dependency

Frontend variants can still be requested with `--frontend next` or
`--frontend vite`; those templates are framework-neutral shells.

## Build

```bash
sunscreen chain build --headless
```

For Pinocchio workspaces, the build pipeline emits `pinocchio_build` events and
runs:

```bash
cargo build-sbf
```

Codama is skipped by default because the current generator consumes Anchor IDLs.
The `--no-codama` flag is therefore redundant for Pinocchio, but remains
accepted for CLI consistency.

## Unsupported Anchor-only commands

The built-in scaffolders still target Anchor source layout and marker regions.
In a Pinocchio workspace, these commands fail before writing files:

```bash
sunscreen scaffold instruction deposit --program fast-program
sunscreen scaffold account Vault --program fast-program
sunscreen scaffold crud Post --program fast-program
sunscreen generate idl
sunscreen generate clients
sunscreen generate frontend-hooks
```

Use plugin-backed `sunscreen scaffold <noun>` commands for Pinocchio-specific
experiments until a first-party Pinocchio scaffolding ADR lands.

## Toolchain

`chain new --framework pinocchio` preflight checks Rust/Cargo/Solana and does
not require Anchor. A JavaScript frontend still requires Node and pnpm.

Real `cargo build-sbf` execution requires the Solana CLI toolchain to be
installed on the host.
