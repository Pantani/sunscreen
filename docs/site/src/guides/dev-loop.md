# The dev loop with `chain serve`

⏱ 6 min · 🎯 you'll have: a single-command dev loop that rebuilds, regenerates clients, and notifies your frontend on every save.

`chain serve` is sunscreen's supervised dev process. It runs a local validator, watches your files, and orchestrates everything else.

## What it runs

```text
┌─────────────────────────────────────────┐
│  chain serve                             │
│                                          │
│  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ validator│  │  watcher │  │pipeline│ │
│  │ (Surfpool│  │  (notify)│→ │anchor  │ │
│  │  or t-v) │  │  debounce│  │ build  │ │
│  └──────────┘  └──────────┘  │ ↓      │ │
│                              │codama  │ │
│                              │ ↓      │ │
│                              │frontend│ │
│                              │notify  │ │
│                              └────────┘ │
└─────────────────────────────────────────┘
```

## Start

From your workspace root:

```bash
sunscreen chain serve
```

The TUI shows four panels: validator status, build log, faucet, frontend status. Press `?` for keybindings, `q` to quit.

Headless (CI / editor integration):

```bash
sunscreen chain serve --headless
```

Headless mode emits one NDJSON event per line on stdout. See [NDJSON events](../reference/events.md).

## What "watching" means

Sunscreen watches your workspace tree minus a list of ignored paths (`target/`, `node_modules/`, `app/.sunscreen/`, hidden dirs). When you save a Rust file, sunscreen:

1. Debounces (waits ~200ms for related saves).
2. Runs `anchor build`.
3. If build succeeded *and* a frontend is configured, runs Codama to regenerate clients.
4. Touches `app/.sunscreen/reload` so your frontend dev server (Vite, Next, …) hot-reloads.

If any step fails, the TUI shows the error and the validator stays up — fix and save again.

## Choose your validator

Sunscreen prefers Surfpool if found on `$PATH`. Override:

```bash
sunscreen chain serve --validator test-validator
sunscreen chain serve --validator surfpool
```

If Surfpool is the default but missing, sunscreen falls back to `solana-test-validator` automatically and logs the fallback.

## Skip Codama on rebuild

If you don't have a frontend and don't need clients:

```bash
sunscreen chain serve --no-codama
```

## A typical session

1. `sunscreen chain serve` — TUI appears, validator boots in ~2s.
2. Open `programs/my_app/src/instructions/create_post.rs`, edit a handler.
3. Save. Within ~3s: anchor builds, Codama regenerates `app/src/clients/`, your Vite dev server reloads.
4. Test in browser. Fix bugs. Repeat.
5. `q` to quit. Sunscreen tears down the validator, kills the process group cleanly.

## Ctrl-C

`Ctrl-C` shuts everything down. Sunscreen stops the Unix process group of the validator (and any subprocess it spawned). If something doesn't exit within a grace period, sunscreen sends `SIGKILL`.

## When it doesn't help

- You don't have an Anchor or Pinocchio workspace. `chain serve` requires one.
- You're on Windows. Process-group teardown isn't yet implemented for Windows; Phase 8 work.
- You want to manage a remote validator. `chain serve` is for local dev only.

## Going further

- [`chain serve` reference](../reference/cli/chain.md#serve) — every flag.
- [NDJSON events](../reference/events.md) — for editor integrations.
- [Build pipeline](../concepts/build-pipeline.md) — what runs and in what order.
