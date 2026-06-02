# Recipes Reference

Phase 5 adds composite scaffolders under `sunscreen scaffold`. Recipes reuse the
marker-based primitives from Phase 2 (`account`, `event`, `error`,
`instruction`) and do not write Phase 4 generated paths such as
`clients/js/src/generated` or `app/src/generated/sunscreen`.

## Commands

### `sunscreen scaffold crud <Resource>`

Generates a resource slice:

- `state/<resource>.rs`
- `create_<resource>`, `read_<resource>`, `update_<resource>`, and
  `delete_<resource>` instructions
- `<Resource>Created`, `<Resource>Updated`, and `<Resource>Deleted` events
- `<Resource>NotFound` and `<Resource>Unauthorized` errors
- `tests/<program>/<resource>.test.ts`
- `app/src/hooks/use-<resource>.ts` when the workspace has a frontend

Options:

- `--program <NAME>` selects the target program.
- `--fields <LIST>` accepts the same `name:type` list as `scaffold account`.
- `--no-update` skips `update_<resource>`.
- `--no-delete` skips `delete_<resource>` and the delete hook/event.
- `--no-events` skips event structs and instruction `emit!` stubs.
- `--no-frontend` skips the recipe hook.
- `--dry-run` validates and reports the plan without writing.

`read_<resource>` is generated as a full instruction alongside
`create_<resource>`, `update_<resource>`, and `delete_<resource>`. The frontend
hook still exposes `use<Resource>(address)` as the read query wrapper, matching
the ADR's client-facing shape.

### `sunscreen scaffold spl-token <Name>`

Generates an internal SPL-token-oriented slice:

- `<Name>` account with authority, mint, and supply fields
- `initialize_<name>`, `mint_<name>`, and `transfer_<name>` instructions
- initialized, minted, and transferred events
- `InvalidMint` and `<Name>Unauthorized` errors
- `tests/<program>/<name>-spl-token.test.ts`

This is a core recipe, not a plugin. Token-2022 extension work now belongs to
the Phase 6 plugin/reference marketplace path.

### `sunscreen scaffold metaplex-nft <Name>`

Generates an internal Token Metadata-oriented slice:

- `<Name>` account with collection mint and item count fields
- `create_<name>`, `mint_<name>`, and `verify_<name>` instructions
- created, minted, and verified events
- `InvalidMetadata` and `<Name>Unauthorized` errors
- `tests/<program>/<name>-metaplex-nft.test.ts`

This command prepares the Anchor program surface. Onboarding commands that wrap
it into one-shot mint flows (`quickstart nft`, wallet setup, deploy, examples,
learn) belong to Phase 5.5.

## Idempotency

Recipes run a dry preflight over every primitive step before writing. If an
existing generated artifact has the same bytes, it is treated as unchanged. If a
recipe-owned extra file already exists with different contents, the command
exits with `user_input` and asks the user to edit or remove the file.

Under `--json`, recipe commands emit one object:

```json
{
  "ok": true,
  "recipe": "crud",
  "resource": "post",
  "program": "blog_app",
  "dry_run": false,
  "unchanged": false,
  "steps": 10,
  "files": ["programs/blog_app/src/state/post.rs"],
  "written": 1
}
```

## Verification

Normal CI covers recipe CLI shape, JSON output, idempotency, generated files,
and frontend hook placement in `tests/scaffold_recipes.rs`.

`tests/compile_generated.rs` includes gated recipe compile cases. Run them with:

```text
cargo fetch && SUNSCREEN_COMPILE_TESTS=1 cargo test --test compile_generated
```

Real Anchor coverage is ignored by default:

```text
cargo test --test integration_anchor scaffold_crud_recipe_builds_and_emits_idl_methods -- --ignored
```
