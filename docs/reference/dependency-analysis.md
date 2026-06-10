# Dependency analysis

Sunscreen validates architectural dependencies with
[Dependency Cruiser](https://github.com/sverweij/dependency-cruiser). The check
is intentionally limited to dependency analysis:

- circular dependencies between Rust modules and top-level layers;
- forbidden imports between discovered project layers;
- layer boundary violations.

It does not calculate coupling metrics, run SonarQube, run mutation testing, or
modify production Rust code.

## Local command

```bash
npm install
npm run arch:deps
```

`dependency-cruiser` analyzes JavaScript/TypeScript projects natively, not Rust.
For this Rust crate, `scripts/dependency-cruiser-rust-adapter.mjs` generates a
temporary JavaScript shadow graph under `.dependency-cruiser-rust/` from
`crate::...` imports. Dependency Cruiser then validates that graph with
`.dependency-cruiser.cjs`.

## Discovered layers

The rules are based on the current source tree and
`docs/site/src/concepts/architecture.md`, not on a generic Clean Architecture
template.

| Layer | Paths |
|-------|-------|
| Interface/entry | `src/main.rs`, `src/cli/`, `src/onboarding/`, `src/tui/` |
| Application capabilities | `src/bootstrap.rs`, `src/runtime/`, `src/plugin/`, `src/codegen/`, `src/scaffold/`, `src/toolchain/`, `src/workspace/` |
| Shared foundation | `src/config/`, `src/templates/`, `src/rustpatch/`, `src/fsutil/`, `src/process.rs`, `src/strings/`, `src/error.rs` |

## Rules

### `no-circular-rust-modules`

Fails on circular dependency chains between Rust source modules in the generated
module graph.

Example detected pattern:

```text
src/cli/onboarding.rs -> src/onboarding/wizard.rs -> src/cli/onboarding.rs
```

### `no-circular-rust-layers`

Fails on circular dependency chains between top-level Rust modules/layers.

Example detected pattern:

```text
src/codegen/* -> src/runtime/* -> src/codegen/*
```

### `lower-layers-must-not-import-interface`

Forbids non-interface modules from importing `cli`, `onboarding`, or `tui`.
The command surface may orchestrate lower layers, but lower layers should not
reach back into command parsing or UI modules.

Example detected pattern:

```text
src/runtime/pipeline.rs -> src/cli/chain.rs
```

### `foundational-modules-stay-foundational`

Forbids shared foundation modules from depending on application capability or
interface modules. Foundation modules should stay reusable and low-level.

Example detected pattern:

```text
src/config/schema.rs -> src/cli/root.rs
```

### `codegen-must-not-import-runtime`

Allows runtime orchestration to call code generation, but forbids codegen from
depending back on runtime. This protects the build pipeline from becoming a
bidirectional dependency.

Example detected pattern:

```text
src/codegen/codama.rs -> src/runtime/subprocess.rs
```

### `plugin-runtime-boundary`

Keeps the plugin runtime independent from CLI, runtime orchestration,
generators, scaffolders, templates, toolchain logic, TUI, rustpatch, and fsutil.
The current intended dependencies are plugin internals plus config, workspace,
and error types.

Example detected pattern:

```text
src/plugin/manager.rs -> src/cli/app.rs
```

### `workspace-boundary`

Keeps workspace discovery/model code below application capabilities. It may use
low-level shared modules, but it should not depend on command, runtime,
generator, plugin, scaffold, template, toolchain, or UI modules.

Example detected pattern:

```text
src/workspace/mod.rs -> src/runtime/pipeline.rs
```

### `toolchain-runtime-boundary`

Allows `toolchain` to use the shared `process` boundary for testable command
execution, but forbids coupling toolchain detection or repair code to runtime
orchestration modules.

Example detected pattern:

```text
src/toolchain/fix.rs -> src/runtime/supervisor.rs
```
