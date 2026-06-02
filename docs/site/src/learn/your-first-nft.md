# Your first NFT in 10 minutes

⏱ 10 min · 🎯 you'll have: a Metaplex NFT scaffold ready to mint on devnet.

We'll go from empty folder to a mintable NFT program using sunscreen's quickstart recipe.

## Pre-requisites

- `sunscreen` ([Installing](./installing.md)).
- `anchor` CLI, `solana` CLI, Node 18+ with `pnpm`.
- A Solana keypair (we'll create one if you don't have it).

Run `sunscreen doctor` to confirm. Anything missing will be flagged.

## Step 1 — Quickstart

```bash
sunscreen quickstart nft my-first-nft
cd my-first-nft
```

This single command:

1. Created an Anchor workspace with a Vite frontend.
2. Scaffolded a Metaplex NFT recipe slice (mint, metadata, master edition).
3. Added a sample frontend hook for minting (TanStack Query).

You'll see a summary table at the end listing files created.

```admonish tip
`quickstart` is a composition of more granular commands: `chain new`, `scaffold metaplex-nft`, `generate frontend-hooks`. You can run them individually for more control — quickstart is the beginner-friendly shortcut.
```
## Step 2 — Build

```bash
sunscreen chain build
```

Successful build produces `target/idl/my_first_nft.json` and regenerates Codama clients into `app/src/clients/` (auto-wired because we picked the React frontend).

## Step 3 — Configure a devnet wallet

If you don't have a Solana keypair:

```bash
sunscreen wallet new dev
sunscreen wallet set-default dev --cluster devnet
```

The first command creates a keypair under `.sunscreen/wallets/dev.json` and prints the public key. The second makes it sunscreen's default for devnet operations.

Get devnet SOL:

```bash
sunscreen wallet airdrop 2 --cluster devnet
```

If the airdrop is throttled (common), the error tells you the next step (use `solana airdrop` directly or a public faucet).

## Step 4 — Deploy plan

```bash
sunscreen deploy devnet --dry-run
```

Sunscreen prints a deploy plan: how much SOL you need, what's going to be uploaded. No on-chain action yet — it's a `--dry-run`.

When ready:

```bash
sunscreen deploy devnet
```

You'll get a program ID. Save it.

## Step 5 — Mint

The Vite frontend in `app/` is already wired with the Codama-generated client and the mint hook. Start it:

```bash
cd app
pnpm install
pnpm dev
```

Open <http://localhost:5173> (Vite's default). Connect your wallet (the same one you funded), click *Mint*, approve. After a few seconds you'll see the mint signature.

You just minted an NFT through a program *you* wrote — even though sunscreen wrote most of it for you.

## What happened end-to-end

```text
quickstart nft           → workspace + Metaplex recipe + frontend hooks
chain build             → anchor build → IDL → Codama clients
wallet new / airdrop    → fund a keypair on devnet
deploy devnet           → upload program, register program ID
frontend pnpm dev       → Vite app calls the generated client → mints
```

## Next

- [The dev loop with `chain serve`](../guides/dev-loop.md) — hot reload on save.
- [Deploying to mainnet](../guides/deploying-to-mainnet.md) — going from devnet to prod.
- [Metaplex NFT recipe reference](../reference/recipes/metaplex-nft.md) — what was generated, and why.
