# SPL Token recipe

```
sunscreen scaffold spl-token <NAME> --program <PROGRAM> [FLAGS]
```

Generates an SPL Token mint+transfer slice inside an existing program.

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--program <name>` | required | which program to scaffold into |
| `--decimals <n>` | `9` | mint decimals |
| `--frontend-hook` | off | generate matching React/Solid hook |
| `--dry-run` | off | print plan only |
| `--json` | off | summary on stdout |
| `--force` | off | overwrite marker content even on conflict |

## Generated files

For `scaffold spl-token MyToken --program app`:

| Path | Status |
|------|--------|
| `programs/app/src/state/my_token.rs` | created (mint metadata account) |
| `programs/app/src/instructions/init_my_token.rs` | created |
| `programs/app/src/instructions/mint_my_token.rs` | created |
| `programs/app/src/instructions/transfer_my_token.rs` | created |
| `programs/app/src/instructions/mod.rs` | patched |
| `programs/app/src/lib.rs` | patched (dispatch) |
| `programs/app/src/events.rs` | patched |
| `programs/app/src/errors.rs` | patched |
| `tests/my_token.spec.ts` | created |

## Generated instructions

| Instruction | Effect |
|-------------|--------|
| `init_my_token` | Creates the mint PDA, sets authority and decimals |
| `mint_my_token` | Mints `amount` to a destination token account (CPI to `spl-token`) |
| `transfer_my_token` | Transfers `amount` between token accounts |

## Generated events

- `TokenInitialized { mint, authority, decimals }`
- `TokenMinted { mint, recipient, amount }`
- `TokenTransferred { from, to, amount }`

## Generated errors

- `MintUnauthorized`
- `InsufficientBalance`

## Notes

- This recipe uses the classic SPL Token program (`Tokenkeg…`). For SPL Token-2022, install the [`sunscreen-apps/spl-token-2022`](../plugin-protocol/index.md) plugin and use `sunscreen scaffold spl-token-2022 …`.
- The mint is a PDA seeded by `["mint", name.as_bytes()]`, so the same name yields the same address per program.

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `4` | preflight conflict |
| `5` | not in a workspace |
