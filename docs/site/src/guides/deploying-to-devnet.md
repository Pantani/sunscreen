# Deploying to devnet

⏱ 6 min · 🎯 you'll have: your program deployed on Solana devnet, with the program ID wired into your config.

## Pre-requisites

- Built workspace (`sunscreen chain build` succeeded).
- `solana` CLI on PATH (`solana --version`).
- A Solana keypair, funded with devnet SOL.

## Step 1 — Wallet

If you don't have a keypair yet:

```bash
sunscreen wallet new
```

This writes `~/.config/solana/id.json` and prints the public key. **Save the recovery words** if prompted.

If you already have a keypair, point to it:

```bash
export SUNSCREEN_WALLET=$HOME/.config/solana/id.json
```

Or set `wallet:` in `sunscreen.yml`. The CLI defaults to the solana-cli default location.

## Step 2 — Airdrop devnet SOL

You need ~2 SOL for a fresh deploy.

```bash
sunscreen wallet airdrop --network devnet --amount 2
```

If you see `Network: rate-limited`, the public faucet throttled you. Options:

- Wait a few minutes and retry.
- Use a public web faucet: <https://faucet.solana.com/>.
- Run `solana airdrop 2 --url devnet` directly.

## Step 3 — Deploy plan (dry run)

Always inspect the plan first:

```bash
sunscreen deploy --network devnet --dry-run
```

You'll see:

```text
plan
─ network: devnet (https://api.devnet.solana.com)
─ payer:   <your-pubkey> (balance: 2.0 SOL)
─ programs:
    my_app  → target/deploy/my_app.so (180 KB)
─ estimated cost: ~1.6 SOL
```

If the estimated cost exceeds your balance, the dry run fails with `exit 4` and tells you to airdrop more.

## Step 4 — Deploy

```bash
sunscreen deploy --network devnet
```

Under the hood, this runs `solana program deploy` for each program in your workspace and updates `Anchor.toml` + `sunscreen.yml` with the new program IDs.

On success:

```
✓ deployed my_app at <program-id>
✓ Anchor.toml updated
✓ sunscreen.yml updated
```

Your IDL is also published if you pass `--with-idl`.

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

This rewrites `app/src/clients/` against the now-deployed program ID. Restart your `pnpm dev` so it picks up the new clients.

## Re-deploying after code changes

Each subsequent deploy is incremental:

```bash
sunscreen chain build
sunscreen deploy --network devnet
```

Solana's `solana program deploy` handles the upgrade in place, reusing the program ID.

## Common pitfalls

| Symptom | Cause | Fix |
|---------|-------|-----|
| `insufficient funds` | not enough SOL | airdrop or use the web faucet |
| `program too large` | binary > buffer size | check `--max-len`, or upgrade in chunks |
| `transaction simulation failed` | program-side check failed | run the test suite locally first |
| `BlockhashNotFound` | RPC overloaded or clock skew | retry; consider a private RPC (Helius, Triton) |

## Next

- [Deploying to mainnet](./deploying-to-mainnet.md) — the same flow with more caution.
- [Working with plugins](./plugins.md).
- [`deploy` reference](../reference/cli/onboarding.md#deploy).
