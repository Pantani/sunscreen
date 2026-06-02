# Anchor vs Pinocchio

Sunscreen supports two Solana program frameworks. Choose based on what you're optimizing for.

## TL;DR

| | Anchor | Pinocchio |
|---|---|---|
| Mental model | High-level, macros do the heavy lifting | Low-level, you wire account checks yourself |
| Compile-time guarantees | Strong (`#[derive(Accounts)]` checks at build time) | Minimal (you assert at runtime) |
| Compute units | Higher overhead | Near-bare-metal |
| Code volume | Less code per instruction | More code per instruction |
| Sunscreen support | Full (scaffold, codegen, recipes, plugins) | Workspace bootstrap + build only |
| When to pick | New projects, productivity-first | Performance-critical paths, audit-sensitive code |

## Anchor in one paragraph

[Anchor](https://www.anchor-lang.com/) is a Rust framework on top of the Solana program SDK. It defines three macros (`#[program]`, `#[derive(Accounts)]`, `#[account]`) that hide most of the account-handling boilerplate. It also auto-generates an IDL, which sunscreen feeds into Codama for client generation. Default choice for productivity.

## Pinocchio in one paragraph

[Pinocchio](https://github.com/anza-xyz/pinocchio) is a thin, no-allocation alternative for Solana programs. It exposes the runtime more directly, so you pay fewer compute units (CUs) per instruction. There's no IDL emission, no codegen, no Anchor macros — you write the dispatch and account validation by hand. Use for programs where CU budget matters.

## Sunscreen's coverage today

| Feature | Anchor | Pinocchio |
|---------|--------|-----------|
| `chain new` | ✅ | ✅ (minimal template) |
| `chain build` | ✅ | ✅ (`cargo build-sbf`) |
| `chain serve` | ✅ | ✅ |
| `scaffold instruction/account/event/error` | ✅ | ❌ (manual) |
| `scaffold crud/spl-token/metaplex-nft` | ✅ | ❌ |
| `generate idl` | ✅ | ❌ (no IDL) |
| `generate clients` | ✅ | ❌ |
| `generate frontend-hooks` | ✅ | ❌ |
| Plugin `scaffold <noun>` | ✅ | partial (plugins decide) |

If you choose Pinocchio, sunscreen still gives you the workspace bootstrap, build, and local dev loop. Scaffolders and codegen require Anchor's IDL.

## Migrating between

There's no automatic migration. Anchor programs use macros and account layouts that don't map 1:1 to Pinocchio. If you want to migrate a hot path to Pinocchio, the typical pattern is:

1. Keep the Anchor program as the user-facing surface.
2. Extract the hot instruction into a separate Pinocchio program.
3. Have the Anchor program CPI into the Pinocchio program.

## See also

- [`chain new --framework`](../reference/cli/chain.md#new) — choosing at creation time.
- [Anchor docs](https://www.anchor-lang.com/).
- [Pinocchio repo](https://github.com/anza-xyz/pinocchio).
