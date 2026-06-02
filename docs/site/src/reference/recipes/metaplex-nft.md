# Metaplex NFT recipe

```
sunscreen scaffold metaplex-nft <NAME> --program <PROGRAM> [FLAGS]
```

Generates a Metaplex Token Metadata-compatible NFT mint slice.

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--program <name>` | required | which program to scaffold into |
| `--collection <name>` | none | optional collection account name |
| `--frontend-hook` | off | generate matching React/Solid hook |
| `--dry-run` | off | print plan only |
| `--json` | off | summary on stdout |
| `--force` | off | overwrite marker content even on conflict |

## Generated files

For `scaffold metaplex-nft MyNft --program app`:

| Path | Status |
|------|--------|
| `programs/app/src/state/my_nft.rs` | created |
| `programs/app/src/instructions/mint_my_nft.rs` | created |
| `programs/app/src/instructions/mod.rs` | patched |
| `programs/app/src/lib.rs` | patched (dispatch) |
| `programs/app/src/events.rs` | patched |
| `programs/app/src/errors.rs` | patched |
| `tests/my_nft.spec.ts` | created |

## Generated instruction: `mint_my_nft`

Accounts:

```
[
  authority: Signer,
  mint: UncheckedAccount (init),
  token_account: Account<TokenAccount> (init_if_needed),
  metadata: UncheckedAccount (metaplex pda),
  master_edition: UncheckedAccount (metaplex pda),
  ... + system_program, token_program, associated_token_program, rent
]
```

Effect:

1. Creates a mint with 0 decimals (NFT convention).
2. Mints 1 token to the authority's associated token account.
3. CPI to Metaplex Token Metadata to create `metadata` + `master_edition`.
4. Emits `NftMinted { mint, owner, uri }`.

## Generated events

- `NftMinted { mint, owner, uri }`

## Generated errors

- `MetadataUriTooLong`
- `MintAuthorityMismatch`

## Frontend hook (`--frontend-hook`)

```ts
useMintMyNft({ authority, uri, name, symbol });
```

Builds, signs, and submits the transaction. Invalidates the `useMyNftCollection` query on success.

## Notes

- The recipe uses Metaplex Token Metadata via CPI. Your `Cargo.toml` gets `mpl-token-metadata` added as a dep on first scaffold.
- `--collection` wires the minted NFT into a Metaplex collection account.

## Exit codes

| Code | When |
|------|------|
| `0` | success |
| `4` | preflight conflict |
| `5` | not in a workspace |
