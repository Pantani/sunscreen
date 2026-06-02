# Onboarding commands

Beginner-friendly shortcuts that compose other sunscreen commands. All require the `onboarding` feature (enabled by default in release builds).

## `init`

```
sunscreen init [<NAME>] [FLAGS]
```

Interactive wizard that asks 3–5 questions and runs `chain new` under the hood.

| Flag | Default | Description |
|------|---------|-------------|
| `--non-interactive` | off | disable prompts and require flag-based input |
| `--from-preset <NAME>` | none | preset to apply when no prompts are available |
| `--frontend <name>` | `vite` | `none`, `vite`, `next` |
| `--path <DIR>` | `./<NAME>` | output directory |
| `--dry-run` | off | print planned files without writing |

## `examples`

```
sunscreen examples <SUBCOMMAND>
```

| Subcommand | What it does |
|------------|-------------|
| `list [--tag <TAG>]` | list embedded examples (optionally filtered) |
| `describe <NAME>` | print one example's README |
| `use <NAME> [<PATH>] [--non-interactive] [--dry-run]` | copy an example onto disk |

## `quickstart`

```
sunscreen quickstart <RECIPE> [FLAGS]
```

Composite recipes for "I want a working X in 30 seconds".

| Recipe | What it builds |
|--------|---------------|
| `token` | Anchor workspace + SPL Token recipe |
| `nft` | Anchor workspace + Metaplex NFT recipe |
| `dao` | Anchor workspace + DAO voting scaffolds |
| `blog` | Anchor workspace + CRUD `Post` resource |

| Flag | Default | Description |
|------|---------|-------------|
| `--name <NAME>` | prompted | project name (required in `--non-interactive`) |
| `--cluster <NAME>` | `localnet` | `localnet`, `devnet`, `mainnet` — used for the generated next steps |
| `--non-interactive` | off | disable prompts |
| `--frontend <name>` | `vite` | `none`, `vite`, `next` |
| `--path <DIR>` | `./<NAME>` | output directory |
| `--dry-run` | off | print planned operations without writing |

## `wallet`

```
sunscreen wallet <SUBCOMMAND>
```

| Subcommand | What it does |
|------------|-------------|
| `new [<NAME>] [--out <FILE>] [--no-bip39-passphrase] [--dry-run]` | Generate a keypair. When `--out` is omitted, lands under `.sunscreen/wallets/<NAME>.json` |
| `list` | List wallets discovered under `.sunscreen/wallets/` |
| `airdrop [<AMOUNT>] [--cluster <NAME>] [--to <PUBKEY>] [--dry-run]` | Request SOL. `AMOUNT` defaults to `1.0`. `--cluster` defaults to `devnet`. `--to` defaults to the Solana CLI default keypair |
| `balance [<ADDRESS>] [--cluster <NAME>]` | Print a wallet balance |
| `set-default <NAME> [--cluster <NAME>]` | Set the default wallet path in `sunscreen.yml` for a cluster |

**Examples:**

```bash
sunscreen wallet new dev
sunscreen wallet airdrop 2 --cluster devnet
sunscreen wallet balance --cluster devnet
sunscreen wallet set-default dev --cluster localnet
```

## `deploy`

```
sunscreen deploy <TARGET> [FLAGS]
```

Build and deploy programs to a Solana cluster.

| Arg / flag | Default | Description |
|------------|---------|-------------|
| `<TARGET>` | required | `localnet`, `devnet`, or `mainnet` (positional, value-enum) |
| `--program <NAME>` | all programs | pass through to Anchor for a single program |
| `--verify` | off | run `anchor verify` after deploy when supported |
| `--yes-i-understand-cost` | off | **required for `mainnet`** |
| `--dry-run` | off | print deployment plan without running Anchor |

**Exit codes:** `0` ok · `2` toolchain · `4` invalid args / insufficient balance · `5` no workspace.

**Examples:**

```bash
sunscreen deploy devnet
sunscreen deploy devnet --program my_app --dry-run
sunscreen deploy mainnet --yes-i-understand-cost
```

## `learn`

```
sunscreen learn [<TOPIC>]
```

Print an embedded topic in the terminal. Omit `<TOPIC>` to list available topics.

## `next_step` contract

Every onboarding error includes a `next_step` field in JSON output and a final line in human output telling the user exactly what to do. The contract is tested in `tests/errors_contract.rs` and is part of sunscreen's stable surface.

Example error:

```text
error: Network: rate-limited (exit 4)
next_step: Try the web faucet at https://faucet.solana.com/ or wait 10 minutes.
```
