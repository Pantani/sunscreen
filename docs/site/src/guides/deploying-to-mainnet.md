# Deploying to mainnet

⏱ 10 min · 🎯 you'll have: your program live on mainnet, with the same shape as your devnet deploy.

Mainnet is real money. Read this whole page before running anything.

## Pre-flight checklist

- [ ] Tests pass locally (`anchor test`).
- [ ] Deployed to devnet first and tested end-to-end.
- [ ] You have a dedicated mainnet keypair, separate from your dev wallet.
- [ ] That keypair has ~3 SOL (typical Anchor program deploy is 1.5–2.5 SOL).
- [ ] You're using a private RPC (Helius, Triton, QuickNode). Public mainnet RPCs throttle aggressively.
- [ ] Your program's upgrade authority is your control (a multisig, ideally).

## Step 1 — Configure your wallet and RPC

Create or import a mainnet wallet and make it the default for `mainnet`:

```bash
sunscreen wallet new prod
sunscreen wallet set-default prod --cluster mainnet
```

Sunscreen passes through to the Solana CLI for RPC selection. Set a private endpoint in your shell or `solana config`:

```bash
solana config set --url https://your-rpc-endpoint.example.com
```

## Step 2 — Dry run

```bash
sunscreen deploy mainnet --yes-i-understand-cost --dry-run
```

`--yes-i-understand-cost` is **required** for mainnet — sunscreen refuses to run without it. The `--dry-run` makes this safe: it prints the plan without sending transactions.

Read the plan carefully:

- Confirm the **payer** is your mainnet wallet, not your devnet one.
- Confirm the **program count** matches what you intend.
- Confirm your balance is enough.

If anything looks off, stop and investigate. Mainnet deploys do not refund "I clicked too fast".

## Step 3 — Deploy

```bash
sunscreen deploy mainnet --yes-i-understand-cost
```

The CLI builds (if `target/deploy/*.so` is missing or stale), then runs `anchor deploy --provider.cluster mainnet`. A typical deploy takes 30–90 seconds per program, depending on RPC.

## Step 4 — Verify

```bash
solana program show <program-id> --url mainnet-beta
```

Confirm the program is owned by `BPFLoaderUpgradeab1e11111111111111111111111` and the upgrade authority is what you expect.

If you want users to verify the source matches the deployed bytecode, publish the program with [Solana verifiable builds](https://github.com/Ellipsis-Labs/solana-verifiable-build) (sunscreen doesn't automate this yet).

## Step 5 — Lock down upgrade authority

By default, the deploying keypair becomes the upgrade authority. For production, transfer it to a multisig (e.g. [Squads](https://squads.so/)) or, if you don't want anyone to upgrade, set it to `None`:

```bash
solana program set-upgrade-authority <program-id> --final
```

```admonish warning`--final` is irreversible. The program can never be upgraded again. Only do this for programs that you've audited and tested exhaustively.
```
## Step 6 — Update clients

```bash
sunscreen generate clients
```

Commit the regenerated clients. Your frontend now points at mainnet program IDs.

## Common pitfalls

| Symptom | Cause | Fix |
|---------|-------|-----|
| `RPC error: 429 Too Many Requests` | public RPC throttled | use a private RPC via `solana config set --url` |
| `BlockhashNotFound` mid-deploy | RPC dropped you | retry; Anchor's deploy is resumable |
| Wrong wallet used | environment variable leaked | inspect `solana config get` before deploying |
| Forgot to update IDL | clients have stale shape | `sunscreen generate clients` and redeploy frontend |

## Going further

- [`deploy` reference](../reference/cli/onboarding.md#deploy)
- [Squads multisig docs](https://docs.squads.so/main/) — to manage upgrade authority.
- [Helius RPC](https://www.helius.dev/) or [Triton](https://triton.one/) for private endpoints.
