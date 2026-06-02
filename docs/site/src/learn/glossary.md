# Glossary

One- or two-sentence definitions for every term used elsewhere in the docs. Linked from primers and tutorials.

## Solana

**Account** — addressable chunk of memory on Solana. Holds lamports, data, an owner program, and an executable flag. Everything on chain is an account.

**Anchor** — a Rust framework that hides Solana's low-level account boilerplate behind macros (`#[program]`, `#[derive(Accounts)]`, `#[account]`). Sunscreen targets Anchor by default.

**BPF** — Berkeley Packet Filter. The bytecode format Solana programs compile to.

**Codama** — a code generator that produces typed clients (JavaScript, Rust, …) from an Anchor IDL. Sunscreen wraps it.

**Devnet** — the public Solana test network. Free SOL via faucet, throwaway, perfect for staging before mainnet.

**Discriminator** — an 8-byte prefix Anchor writes at the start of every program-owned account, identifying its type.

**Fee payer** — the signer who pays transaction fees. Usually the wallet that initiated the transaction.

**IDL (Interface Definition Language)** — JSON file describing a program's instructions and account layouts. Anchor builds emit one; sunscreen exports it.

**Instruction** — a single call to a program inside a transaction. Specifies the program ID, the accounts touched, and a data payload.

**Lamport** — the smallest unit of SOL. 1 SOL = 10⁹ lamports.

**Localnet** — a local Solana validator running on your laptop. Sunscreen's `chain serve` launches one (via Surfpool or `solana-test-validator`).

**Mainnet-beta** — Solana production.

**Metaplex** — a collection of standards and tools for NFTs on Solana. The Token Metadata program is the canonical NFT spec.

**PDA (Program-Derived Address)** — an address derived deterministically from seeds + a program ID. Has no private key. Only the owning program can sign for it.

**Pinocchio** — a minimal-overhead alternative to Anchor for Solana programs. Sunscreen supports it via `chain new --framework pinocchio`.

**Program** — an executable account on Solana. Stateless. Reads and writes other accounts.

**Program ID** — the public key of a deployed program.

**Pubkey** — a 32-byte public key. Addresses, program IDs, signers — all `Pubkey`s.

**Rent** — a one-time deposit when creating an account, proportional to its data size. Refunded on close.

**RPC** — the JSON-RPC endpoint of a Solana validator (e.g. `https://api.devnet.solana.com`).

**Signer** — an account whose private key signed the transaction. Required for any account that gets debited or whose authority is asserted.

**SOL** — the native token. Used for fees and rent.

**SPL Token** — Solana's fungible token standard (and its program). Sunscreen's `scaffold spl-token` recipe generates a slice that mints / transfers.

**Surfpool** — a community-driven local validator alternative to `solana-test-validator`, optimized for dev loops. Sunscreen prefers it when present.

**Transaction** — a bundle of instructions, signed by one or more accounts, submitted to the network atomically.

## Sunscreen

**Chain (subcommand)** — `chain new`, `chain build`, `chain serve`, `chain doctor`. Workspace lifecycle.

**Codama config** — a JSON file Sunscreen manages so Codama knows where to read the IDL and where to write clients.

**Frontend hooks** — React/Solid Query hooks Sunscreen generates from an IDL via `generate frontend-hooks`.

**Generator tag** — the value (e.g. `account`, `event`) Sunscreen writes inside a marker so re-runs know what to regenerate.

**Marker** — a comment pair (`// sunscreen:begin {generator=…}` … `// sunscreen:end`) inside generated files. Sunscreen edits only the content between them on subsequent runs. Read [Marker protocol](../reference/markers.md).

**NDJSON events** — newline-delimited JSON emitted by `chain build` / `chain serve` for editor and CI integration. See [NDJSON events](../reference/events.md).

**Plugin** — an external binary speaking sunscreen's JSON-RPC protocol that extends `scaffold <noun>`. See [Plugin protocol](../reference/plugin-protocol/index.md).

**Recipe** — a composite scaffold built on top of primitives. `scaffold crud`, `scaffold spl-token`, `scaffold metaplex-nft`.

**Scaffold (subcommand)** — adds an instruction, account, event, error, program, or recipe to an existing workspace.

**Sunscreen.yml** — the project's config file. Declarative source of truth for programs, plugins, runtime preferences.

**Quickstart** — beginner-friendly composite command that wires `chain new` + recipes + frontend hooks in one step.
