# Your first workspace

⏱ 8 min · 🎯 you'll have: a freshly scaffolded Anchor workspace, building locally.

We'll create an empty workspace, look at what was generated, and run our first build.

## Pre-requisites

- `sunscreen` installed ([Installing](./installing.md)).
- `anchor` CLI on your PATH (`anchor --version` should print a version). If missing, install via [AVM](https://www.anchor-lang.com/docs/installation).
- `cargo` (comes with Rust).

Don't have everything? Run `sunscreen doctor` first — it tells you exactly what's missing.

## Step 1 — Create

```bash
sunscreen chain new my-app --framework anchor --frontend none
```

You'll see progress output, then a summary similar to:

```
✓ workspace my-app/ created (anchor, no frontend)
```

```admonish tip`--frontend` accepts `none`, `vite`, or `next`. Pick `vite` if you want React + Vite scaffolded under `app/`; pick `next` for a Next.js scaffold. `none` keeps the workspace lean.
```
## Step 2 — Look around

```bash
cd my-app
tree -L 2 -I node_modules
```

```
my-app/
├── Anchor.toml
├── Cargo.toml
├── package.json
├── programs/
│   └── my_app/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── sunscreen.yml      ← sunscreen's config (the source of truth)
├── tests/
│   └── my_app.ts
└── target/
```

The interesting files:

- `sunscreen.yml` — your project config. Programs, plugins, runtime settings.
- `programs/my_app/src/lib.rs` — the Anchor program. Has `// sunscreen:begin ... end` markers that sunscreen can edit on later commands without breaking what you wrote in between.
- `Anchor.toml` — standard Anchor config, points at the program.

```admonish tipThe markers in `lib.rs` are how sunscreen does *incremental* edits. When you run `sunscreen scaffold instruction Foo` later, it injects code only into marked regions. Your hand-written code is left alone. Read more in [Incremental scaffolding](../concepts/incremental-scaffolding.md).
```
## Step 3 — Build

```bash
sunscreen chain build
```

This runs `anchor build` under the hood and reports progress as NDJSON lines (one event per line, useful for editor integrations). On success:

```
✓ anchor build
✓ codama clients regenerated (skipped: no frontend)
```

Behind the scenes:

1. `anchor build` produced `target/idl/my_app.json` and the program's `.so`.
2. Sunscreen skipped Codama client generation because we picked `--frontend none`.

## Step 4 — Add an instruction

```bash
sunscreen scaffold instruction Greet --program my_app
```

Open `programs/my_app/src/lib.rs` — you'll see a new `pub fn greet(...)` handler inside the program module, registered in the dispatch. The marker regions made the edit safe.

Re-run the same command:

```bash
sunscreen scaffold instruction Greet --program my_app
# error: instruction "Greet" already exists in program "my_app" (exit code 4)
```

Idempotent: sunscreen refuses to clobber. Use `--force` if you really want to regenerate.

## What just happened

- One command (`chain new`) gave you a buildable Anchor workspace plus sunscreen's own config.
- The generated code has marker comments that let sunscreen safely edit the same files in later runs.
- `chain build` is a thin orchestration over `anchor build` + Codama generation.
- `scaffold instruction` mutated marked regions only, leaving your code untouched.

## Next

- [Your first NFT in 10 minutes](./your-first-nft.md) — chain `init` + scaffold + deploy.
- [Scaffolding a CRUD resource](../guides/scaffolding-crud.md) — composite recipes.
- [Incremental scaffolding](../concepts/incremental-scaffolding.md) — the marker model in depth.
