# Onboarding commands

Beginner-friendly shortcuts that compose other sunscreen commands.

## `init`

```
sunscreen init [--non-interactive] [--name <NAME>] [--framework <fw>] [--frontend <fe>]
```

Interactive wizard that asks 3–5 questions and runs `chain new` under the hood. With `--non-interactive`, behaves like `chain new` with explicit flags.

## `examples`

```
sunscreen examples [list|show <NAME>|init <NAME> [--out <PATH>]]
```

Browse embedded example projects. `init <NAME>` copies the example into a new directory.

Available examples (embedded at compile time):

- `counter` — minimal counter program.
- `token-faucet` — SPL token mint with a free-claim instruction.
- `nft-collection` — Metaplex NFT collection with mint.
- `dao-voting` — a stripped-down DAO voting program.

## `quickstart`

```
sunscreen quickstart <KIND> <NAME>
```

Composite recipes for "I want a working X in 30 seconds":

| Kind | What it builds |
|------|---------------|
| `token` | Anchor workspace + SPL Token recipe + React frontend with mint UI |
| `nft` | Anchor workspace + Metaplex NFT recipe + React frontend with mint UI |
| `dao` | Anchor workspace + DAO voting scaffolds |
| `blog` | Anchor workspace + CRUD `Post` resource + React frontend |

Equivalent to running `chain new` + the matching `scaffold` recipe + `generate frontend-hooks`.

## `wallet`

```
sunscreen wallet new [--out <PATH>]
sunscreen wallet airdrop --network <NETWORK> --amount <SOL> [--address <PUBKEY>]
sunscreen wallet show [--network <NETWORK>]
```

| Subcommand | What it does |
|------------|-------------|
| `new` | Generate a new keypair at `~/.config/solana/id.json` (or `--out`) |
| `airdrop` | Request SOL from a network's faucet |
| `show` | Print the current wallet's pubkey and balance on a network |

## `deploy`

```
sunscreen deploy [--network <NETWORK>] [--rpc-url <URL>] [--with-idl] [--dry-run] [--json]
```

Build and deploy programs in the workspace to a Solana network.

| Flag | Default | Description |
|------|---------|-------------|
| `--network` | `localnet` | `localnet`, `devnet`, `testnet`, `mainnet-beta` |
| `--rpc-url` | network default | override RPC endpoint |
| `--with-idl` | off | also publish IDL on chain |
| `--dry-run` | off | print plan only |
| `--json` | off | structured output |

**Exit codes:** `0` ok · `2` toolchain · `4` insufficient balance / network unreachable · `5` no workspace.

## `learn`

```
sunscreen learn [list|<TOPIC>]
```

Open an embedded topic in the terminal pager. Topics:

- `markers` — the marker protocol in 1 page.
- `pdas` — PDA basics.
- `idl-flow` — IDL → Codama → clients.
- `rent` — Solana rent in 1 page.

`learn` requires no network. It's the offline equivalent of pointing users at the docs site.

## Exit code: `next_step` contract

Every onboarding error includes a `next_step` field in JSON output and a final line in human output telling the user exactly what to do. Example:

```text
error: Network: rate-limited (exit 4)
next_step: Try the web faucet at https://faucet.solana.com/ or wait 10 minutes.
```

This contract is tested in `tests/errors_contract.rs` and is part of sunscreen's stable API surface.
