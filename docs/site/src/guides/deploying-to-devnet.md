# Deploying to devnet

⏱ 6 min · 🎯 you'll have: your program deployed on Solana devnet, with the program ID wired into your config.

## Pre-requisites

- Built workspace (`sunscreen chain build` succeeded).
- `solana` CLI on PATH (`solana --version`).
- A Solana keypair, funded with devnet SOL.

## Step 1 — Wallet

If you don't have a keypair yet:

```bash
sunscreen wallet new dev
```

This writes a new keypair under `.sunscreen/wallets/dev.json` and prints the public key. **Save the recovery words** if prompted.

Make it the default for localnet/devnet so subsequent commands pick it up:

```bash
sunscreen wallet set-default dev --cluster devnet
```

## Step 2 — Airdrop devnet SOL

You need ~2 SOL for a fresh deploy:

```bash
sunscreen wallet airdrop 2 --cluster devnet
```

If you see `Network: rate-limited`, the public faucet throttled you. Options:

- Wait a few minutes and retry.
- Use a public web faucet: <https://faucet.solana.com/>.
- Run `solana airdrop 2 --url devnet` directly.

Check the balance:

```bash
sunscreen wallet balance --cluster devnet
```

## Step 3 — Deploy plan (dry run)

Always inspect the plan first:

```bash
sunscreen deploy devnet --dry-run
```

The dry run prints the planned Anchor invocation, the wallet that will pay, and the balance check.

## Step 4 — Deploy

```bash
sunscreen deploy devnet
```

Or deploy only one program in a multi-program workspace:

```bash
sunscreen deploy devnet --program my_app
```

On success Anchor updates `Anchor.toml` with the new program ID. Sunscreen surfaces the result and the program addresses.

## Step 5 — Sanity check

```bash
solana program show <program-id> --url devnet
```

You should see the program account with the correct authority and data length.

## Step 6 — Regenerate clients

If you have a frontend:

```bash
sunscreen generate clients
```

This rewrites `clients/<program>/` against the now-deployed program ID. Restart your `pnpm dev` so it picks up the new clients.

## Re-deploying after code changes

Each subsequent deploy is incremental:

```bash
sunscreen chain build
sunscreen deploy devnet
```

Solana's `solana program deploy` handles the upgrade in place, reusing the program ID.

## Common pitfalls

| Symptom | Cause | Fix |
|---------|-------|-----|
| `insufficient funds` | not enough SOL | airdrop or use the web faucet |
| `program too large` | binary > buffer size | check `--max-len` on `solana program deploy`, or upgrade in chunks |
| `transaction simulation failed` | program-side check failed | run the test suite locally first |
| `BlockhashNotFound` | RPC overloaded or clock skew | retry; consider a private RPC (Helius, Triton) |

## Next

- [Deploying to mainnet](./deploying-to-mainnet.md) — the same flow with more caution.
- [Working with plugins](./plugins.md).
- [`deploy` reference](../reference/cli/onboarding.md#deploy).
