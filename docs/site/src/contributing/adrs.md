# Architecture decisions

Sunscreen's design rationale lives in Architecture Decision Records (ADRs) in [`docs/adr/`](https://github.com/Pantani/sunscreen/tree/main/docs/adr).

## Current ADRs

| ID | Title | Status |
|----|-------|--------|
| [ADR-0001](https://github.com/Pantani/sunscreen/blob/main/docs/adr/ADR-0001-solis-cli.md) | The sunscreen CLI: scope, vision, principles | Accepted |
| [ADR-0002](https://github.com/Pantani/sunscreen/blob/main/docs/adr/ADR-0002-cli-design-conventions.md) | CLI design conventions (exit codes, flags, JSON contract) | Accepted |
| [ADR-0003](https://github.com/Pantani/sunscreen/blob/main/docs/adr/ADR-0003-documentation-strategy.md) | Documentation strategy | Accepted |
| [ADR-0004](https://github.com/Pantani/sunscreen/blob/main/docs/adr/ADR-0004-incremental-scaffolding.md) | Incremental scaffolding (markers) | Accepted |
| [ADR-0005](https://github.com/Pantani/sunscreen/blob/main/docs/adr/ADR-0005-beginner-onboarding.md) | Beginner onboarding surface | Accepted |
| [ADR-0006](https://github.com/Pantani/sunscreen/blob/main/docs/adr/ADR-0006-pinocchio-bootstrap.md) | Pinocchio bootstrap | Accepted |

## ADR format

Each ADR has:

- A meta table (status, date, authors, tags, supersedes, related).
- TL;DR.
- Context.
- Decision drivers.
- Considered options.
- Decision.
- Consequences.

Template: see ADR-0001.

## Writing a new ADR

Open a PR adding `docs/adr/ADR-NNNN-<slug>.md`, status `Proposed`. Discuss in the PR. When merged, status flips to `Accepted`. If a later ADR overrides it, set `Status: Superseded by ADR-NNNN` rather than deleting.

ADRs document *why we chose what we chose*, not *how to use the code*. For user-facing docs, contribute to this site instead.
