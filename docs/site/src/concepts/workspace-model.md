# Workspace model

A sunscreen workspace is a directory containing four things:

1. A `sunscreen.yml` at the root.
2. A Rust workspace `Cargo.toml`.
3. An `Anchor.toml` (for Anchor) or nothing extra (for Pinocchio).
4. One or more programs under `programs/<name>/`.

Optional:

- A frontend under `app/` (React or Solid).
- A `plugins/` directory with local plugins.
- A `tests/` directory with TypeScript integration tests.

## Anchor layout

```text
my-app/
├── sunscreen.yml
├── Anchor.toml
├── Cargo.toml          ← workspace member list
├── package.json        ← TypeScript test dependencies
├── tsconfig.json
├── programs/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs        ← #[program], dispatch markers
│   │       ├── state/
│   │       ├── instructions/
│   │       ├── events.rs
│   │       └── errors.rs
│   └── governance/      ← multi-program workspaces are supported
├── app/                 ← frontend (when --frontend != none)
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── clients/     ← Codama-generated
│   │   ├── hooks/       ← from generate frontend-hooks
│   │   └── App.tsx
│   └── .sunscreen/
│       └── reload       ← touch trigger for frontend HMR
└── tests/               ← TypeScript integration tests
```

## Pinocchio layout

```text
bare/
├── sunscreen.yml
├── Cargo.toml
└── programs/
    └── bare/
        ├── Cargo.toml   ← `cargo build-sbf` target
        └── src/
            ├── lib.rs   ← entrypoint! macro, instruction dispatch
            └── instructions/
```

Pinocchio workspaces are minimal: no `Anchor.toml`, no built-in IDL. Codegen and recipes are Anchor-only today.

## Multi-program workspaces

A workspace can declare multiple programs. Sunscreen treats each one independently for `scaffold`, but bundles them for `build`, `deploy`, `serve`:

```yaml
programs:
  - name: core
    path: programs/core
  - name: governance
    path: programs/governance
  - name: treasury
    path: programs/treasury
```

`sunscreen chain build` compiles all three. `sunscreen scaffold instruction Foo --program governance` targets only the named one.

## Discovery

When you invoke any workspace-scoped command, sunscreen walks up the directory tree from cwd looking for `sunscreen.yml`. Found → that's your workspace root. Not found → exit `5` (`missing_workspace`).

You can override with `SUNSCREEN_CONFIG=/path/to/sunscreen.yml`.

## Lock files

Sunscreen does not write its own lock file. The Anchor/Cargo/pnpm lock files are the source of truth for dependency versions. Sunscreen's templates pin minor versions in `Cargo.toml`; you control the rest.

## See also

- [`sunscreen.yml` schema](../reference/config/schema.md)
- [Incremental scaffolding](./incremental-scaffolding.md)
