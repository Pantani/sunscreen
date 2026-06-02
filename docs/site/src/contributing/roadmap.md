# Roadmap

The canonical, living roadmap is [`ROADMAP.md`](https://github.com/Pantani/sunscreen/blob/main/ROADMAP.md) at the repo root. It is the source of truth for what's done, in progress, and queued.

This page summarizes the high-level shape so you can orient quickly.

## Current state

| Phase | Status |
|-------|--------|
| 0 — Foundation (CLI shell, config, toolchain, templates, error) | ✅ closed |
| 1 — Workspace bootstrap (`chain new` for Anchor + frontend variants) | ✅ closed |
| 2 — Incremental scaffolders (`scaffold instruction/account/event/error/program` + `doctor --fix-markers`) | ✅ closed |
| 3 — Build/serve pipeline + watcher + runtime supervisor | ✅ closed |
| 4 — Codegen (`generate clients/idl/frontend-hooks`) | ✅ closed |
| 5 — Recipes (`scaffold crud/spl-token/metaplex-nft`) | ✅ closed |
| 5.5 — Onboarding surface (`init`, `quickstart`, `wallet`, `deploy`, `learn`) | ✅ closed |
| 6 — Plugin runtime (manifest, JSON-RPC, sandbox, marketplace) | ✅ closed |
| 7 — Pinocchio bootstrap (`chain new --framework pinocchio` + build) | ✅ closed |
| **8 — Distribution & docs (this site, completions, Homebrew, Windows)** | **in progress** |

## Phase 8 work items

- ✅ tag-driven `cargo-dist` release pipeline (Linux + macOS).
- ✅ documentation site (this site).
- ⏳ shell completions (`bash`, `zsh`, `fish`, `pwsh`).
- ⏳ Homebrew tap.
- ⏳ Windows artifact (`cargo-dist` matrix).
- ⏳ `cargo dist plan` CI verification.
- ⏳ `cargo-binstall` support.

## After v1.0

- Remote plugin artifact download.
- Pinocchio-native scaffolders.
- Richer marketplace (signed plugins, search).
- Codama provider abstraction (alternative codegen backends).

## Semantic versioning

`v0.x` is a preview line. Breaking changes can happen between minor versions, always called out in [`CHANGELOG.md`](https://github.com/Pantani/sunscreen/blob/main/CHANGELOG.md). `v1.0` will lock the CLI surface and the JSON contracts. After v1.0, breaking changes require a major bump.

## How to contribute to the roadmap

- For larger work: open a discussion or draft an ADR (see [Architecture decisions](./adrs.md)).
- For Phase 8 items: pick an unticked box above, comment on the issue, ship a PR.
- For polish/docs: PRs are welcome on `docs/site/` without an issue.
