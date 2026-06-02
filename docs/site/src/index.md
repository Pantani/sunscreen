# Sunscreen

> A Rust CLI for scaffolding, repairing, and orchestrating Solana Anchor and Pinocchio workspaces.

Sunscreen takes you from an empty folder to a working Solana program — Anchor or Pinocchio — without hand-stitching `anchor`, `solana`, `cargo`, `codama`, `surfpool`, and frontend tooling. It is deterministic, marker-driven, and built for the long term: incremental edits, plugin extension, supervised dev loop.

<div class="hero-cta">

[Start in 10 minutes →](./learn/your-first-nft.md)
&nbsp;&nbsp;[Read the reference →](./reference/cli/index.md)

</div>

---

## Pick your path

::::{.cards}

:::{.card}
### 🌱 Learn

New to Solana, Rust, or both? Start here. We assume zero prior knowledge.

[Begin learning →](./learn/what-is-sunscreen.md)
:::

:::{.card}
### 🛠 Guides

Task-oriented walkthroughs. "How do I scaffold a CRUD?" "How do I deploy to devnet?"

[Browse guides →](./guides/scaffolding-crud.md)
:::

:::{.card}
### 📖 Reference

Every command, every flag, every exit code. For when you know what you want.

[Open reference →](./reference/cli/index.md)
:::

::::

---

## Why sunscreen?

- **Deterministic generation.** Same inputs, same files, byte-for-byte. Golden-tested.
- **Marker-based incremental edits.** Add a new instruction to an existing program without breaking what's there. `chain doctor --fix-markers` repairs safe drift.
- **Supervised dev loop.** `chain serve` runs Surfpool (or `solana-test-validator`), watches files, rebuilds, regenerates Codama clients, notifies the frontend — all in one process.
- **Plugins.** Local plugins extend `scaffold <noun>` via stdio JSON-RPC; gRPC contract for the future. Trust and sandboxing are explicit.
- **Two frameworks.** Anchor for the productive default; Pinocchio for the bare-metal path.

## Status

`v0.1.0` is the first preview release. The path to `v1.0` is tracked in the [Roadmap](./contributing/roadmap.md). Phase 8 (this site, completions, Homebrew/Windows distribution, release polish) is in progress.
