# Recipes

Composite scaffolds built on top of [`scaffold` primitives](../cli/scaffold.md).

Each recipe runs as a single command and produces a complete, working slice — account, instructions, events, errors, tests, optional frontend hooks. All operations are idempotent and marker-aware.

## Available recipes

| Recipe | What it generates |
|--------|------------------|
| [CRUD](./crud.md) | An account + 4 instructions (`create`/`read`/`update`/`delete`) + 3 events + 2 errors + TS test |
| [SPL Token](./spl-token.md) | An SPL Token mint + transfer slice |
| [Metaplex NFT](./metaplex-nft.md) | A Metaplex NFT mint with metadata + master edition |

## Recipe contract

Every recipe guarantees:

- **Preflight dry-run** before any write. If any underlying primitive would conflict, the recipe bails with exit `4` and no partial writes.
- **Single JSON object** under `--json`. The summary contains every file touched and every symbol generated.
- **Idempotency**. Same inputs → same outputs. Re-running is a no-op.
- **Marker-safety**. All writes happen inside marker regions. Hand-edits outside markers survive.
- **Optional frontend coupling**. When the workspace declares a frontend, `--frontend-hook` regenerates the matching React/Solid hook.

## Writing a custom recipe

Sunscreen doesn't expose user-defined recipes natively. The supported way to extend the recipe surface is via a plugin that registers `scaffold <noun>` commands. See [Plugin protocol](../plugin-protocol/index.md).
