# What is sunscreen?

⏱ 5 min read · 🎯 you'll understand: what sunscreen does, when to use it, when not to.

Sunscreen is a command-line tool for Solana developers. It scaffolds programs, manages a local dev loop, generates clients, and is extensible by plugins.

If you've ever written a Solana program, you know the friction: `cargo`, `anchor`, `solana-cli`, IDL generation, a frontend, a local validator, hot reload — five tools, five mental models. Sunscreen is one CLI that orchestrates them, with one config file (`sunscreen.yml`).

## What sunscreen does

- **Creates** a new Anchor or Pinocchio workspace from a single command.
- **Adds** instructions, accounts, events, errors, or full recipes (CRUD, SPL Token, Metaplex NFT) to an existing program — without breaking what's there. It uses **markers** in your code to edit the same files safely on every run.
- **Builds and watches** in one supervised process: `chain serve` runs Surfpool (or `solana-test-validator`), watches files, rebuilds with Anchor, regenerates Codama clients, and tells your frontend to reload.
- **Generates clients**: IDL, JavaScript via Codama, React/Solid Query hooks — all deterministic, all idempotent.
- **Onboards beginners**: `init`, `quickstart token|nft|dao|blog`, embedded examples, wallet helpers, deploy plans, and actionable errors that tell you the next step.

## When to use sunscreen

- You're starting a new Solana program and want a clean workspace in 30 seconds.
- You want a single tool that scaffolds, builds, runs a local validator, and regenerates clients on file changes.
- You like declarative configs (`sunscreen.yml`) over running 6 separate commands.
- You want the option to extend the CLI with plugins for your team.

## When *not* to use sunscreen

- Your program is already deeply customized in a non-Anchor, non-Pinocchio workflow. Sunscreen targets these two frameworks.
- You need Windows distribution today — that's planned for v1.0 (see [Roadmap](../contributing/roadmap.md)). macOS and Linux work now.
- You want to manage validator clusters in production — sunscreen targets local dev. Use [Helius](https://www.helius.dev/) or similar for prod RPC.

## How it compares to Anchor CLI alone

Anchor CLI gives you `anchor init`, `anchor build`, `anchor deploy`. Great primitives. Sunscreen sits one level above:

| Task | Anchor CLI | Sunscreen |
|------|-----------|-----------|
| Create workspace | `anchor init` | `sunscreen chain new --framework anchor --frontend react` |
| Add instruction | hand-edit lib.rs | `sunscreen scaffold instruction Create --program app` |
| Add CRUD slice | manual (~200 lines) | `sunscreen scaffold crud Post --program app` |
| Watch + rebuild + regenerate clients | run 3 terminals | `sunscreen chain serve` |
| Diagnose toolchain | trial and error | `sunscreen doctor` |

Anchor stays under the hood. Sunscreen does not replace it; it composes with it.

## Next

- [Install it](./installing.md).
- [Make your first workspace](./first-workspace.md).
- Or jump straight to [your first NFT in 10 minutes](./your-first-nft.md).
