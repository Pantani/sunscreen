# Solana primer (in 5 minutes)

⏱ 5 min · 🎯 you'll understand: accounts, programs, PDAs, fees, devnet/mainnet.

Everything on Solana is an **account**. Programs are accounts. Wallets are accounts. State is accounts. Once you internalize that, the rest follows.

## Accounts

An account on Solana is a chunk of memory at an address (a 32-byte public key). It has:

- **lamports**: balance in the native token (1 SOL = 1,000,000,000 lamports).
- **data**: arbitrary bytes (program-defined layout).
- **owner**: the program that controls writes to this account.
- **executable**: `true` if the account *is* a program.

Two accounts of interest in your code:

- **Wallet account**: a keypair (you control). Holds SOL, signs transactions.
- **Data account**: holds program state (a counter, a user profile, an NFT mint).

## Programs

A **program** is an account marked `executable`. Its `data` is BPF bytecode. Programs are stateless — they read and write *other* accounts. Your Anchor program is exactly this: a single executable account, deployed to an address (the *program ID*).

## Transactions and instructions

You don't call a program directly. You build a **transaction** containing one or more **instructions**:

```text
transaction
├── signers: [wallet]
├── instructions:
│   └── { program_id, accounts, data }
```

The runtime delivers the instruction to the program, which mutates the listed accounts and returns success or error.

## PDAs (Program-Derived Addresses)

A normal address has a private key. A **PDA** does *not* — it's deterministically derived from seeds + a program ID, and only that program can sign for it. This is how programs "own" data accounts.

```rust
let (pda, bump) = Pubkey::find_program_address(
    &[b"counter", user.key().as_ref()],
    program_id,
);
```

You'll see PDAs everywhere in Anchor: `#[account(seeds = [...], bump)]` declares them.

## Fees

Every transaction pays a small fee in SOL, charged to the *fee payer* (usually the signer). On devnet this is essentially free. On mainnet, 5,000 lamports per signature is the base.

Account creation also costs **rent**: a one-time deposit proportional to the account's data size. You get it back when you close the account.

## Networks

Three official networks plus your local validator:

| Network | URL | Use |
|---------|-----|-----|
| `localnet` | `http://127.0.0.1:8899` | a validator on your laptop (sunscreen's `chain serve` launches one) |
| `devnet` | `https://api.devnet.solana.com` | free SOL via faucet, public, test programs here first |
| `testnet` | `https://api.testnet.solana.com` | validator testing, not for app dev |
| `mainnet-beta` | `https://api.mainnet-beta.solana.com` | production |

## The development flow you'll use

1. Write the program (sunscreen scaffolds it).
2. `chain serve` — runs against `localnet` with hot reload.
3. `deploy --network devnet` — push to devnet, test with real fees + real wallets.
4. `deploy --network mainnet-beta` — production.

## Common pitfalls (the ones that bite everyone)

- **You forgot to airdrop SOL** before deploying to devnet. Run `sunscreen wallet airdrop`.
- **Wrong program ID**: after a deploy, your client must use the new ID. Sunscreen's Codama clients regenerate automatically.
- **Account already initialized**: PDAs are deterministic. If you `init` the same PDA twice, the second call fails. Use `init_if_needed` or design for one-shot init.
- **Out of rent**: closing an account refunds rent; not closing leaves SOL locked.

## Want more?

The Solana docs are good: <https://solana.com/docs>. For Anchor specifically: <https://www.anchor-lang.com/>.

## Next

- [Your first workspace](./first-workspace.md) — apply this to a real program.
- [Glossary](./glossary.md) — every term in one place.
