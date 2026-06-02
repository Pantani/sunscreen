# Onboarding Commands

Phase 5.5 adds beginner-facing commands on top of the existing expert surface.
They are additive: `chain new`, `scaffold`, `generate`, `chain build`, and
`chain serve` keep their existing contracts.

## Workspace Entry

```sh
sunscreen init my-app --non-interactive --frontend vite
```

`init` uses the same workspace construction path as `chain new`, then prints a
single next step. In CI, pass `--non-interactive`; when stdin is not a TTY,
missing promptable values return `user_input` (exit 4).

## Quickstarts

```sh
sunscreen quickstart token --name token-app --cluster localnet --non-interactive
sunscreen quickstart nft --name nft-app --cluster devnet --non-interactive
sunscreen quickstart dao --name dao-app --cluster localnet --non-interactive
sunscreen quickstart blog --name blog-app --cluster localnet --non-interactive
```

Quickstarts compose `chain new` with the Phase 5 recipe scaffolders. They do
not shell out to the `sunscreen` binary. `--json` emits one structured object
with `operations` and `next_steps`.

## Examples And Learn

```sh
sunscreen examples list
sunscreen examples describe nft-collection
sunscreen examples use blog-crud ./blog-crud
sunscreen learn
sunscreen learn pda
```

Examples and learning topics are embedded in the binary through the default
`onboarding` Cargo feature. Build with `--no-default-features` to remove this
asset bundle.

## Wallet And Deploy

```sh
sunscreen wallet new --out ~/.config/solana/id.json --no-bip39-passphrase
sunscreen wallet airdrop 1 --cluster devnet
sunscreen wallet balance --cluster devnet
sunscreen deploy devnet --program my_program
```

`wallet` and `deploy` use the shared subprocess boundary. Missing external
tools map to `toolchain_missing` (exit 2); RPC/deploy failures map to `network`
(exit 8). `deploy mainnet` requires `--yes-i-understand-cost`.

## Error Contract

JSON errors keep the legacy `error` and `kind` fields and add:

- `next_step`: remediation hint
- `exit_code`: numeric process exit code

New Phase 5.5 exit codes:

- `7`: `path_conflict`
- `8`: `network`
