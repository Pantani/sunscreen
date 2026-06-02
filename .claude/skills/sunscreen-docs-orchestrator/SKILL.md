---
name: sunscreen-docs-orchestrator
description: Orchestrates the documentation team for the sunscreen CLI — mdBook site, GitHub Pages, the Learn/Guides/Reference/Concepts tracks, visual identity, and docs QA. Use whenever the user asks to create, write, expand, review, update, polish, redesign, or publish sunscreen documentation — including "docs site", "GitHub Pages", "tutorials", "quickstart", "reference", "primer", "glossary", "landing", "docs theme", "beautiful docs", "TMDCP-style docs", "Phase 8 docs", "beginner docs", "professional docs", "guide", "how to use", "manual", "mdBook", "publish docs". Do not use it for ADRs (those belong to sunscreen-orchestrator → docs-writer) or for code implementation.
---

# Sunscreen Docs Orchestrator

Coordinates 5 agents to ship the sunscreen documentation site, the Phase 8 target in `ROADMAP.md`. Target style: TMDCP — premium, editorial, beginner-friendly, and dense for professionals.

## Phase 0: Context

1. Re-read `CLAUDE.md`, `ROADMAP.md`, `README.md`, `docs/adr/ADR-0003-documentation-strategy.md`, and `docs/reference/*`.
2. List what already exists under `docs/site/` (if any) and under `_workspace/`.
3. Pick an execution mode:
   - `docs/site/` is missing → **full initial run** (Phases 1→6).
   - `docs/site/` exists + a specific request (e.g. "update the chain serve reference") → **partial run** (only the agent that owns that area).
   - `_workspace/docs-review.md` exists and lists blockers → **fix run** (call the original author of the defect).

## Team

| Agent | Domain |
|--------|---------|
| `docs-architect` | `docs/site/book.toml`, `SUMMARY.md`, Pages workflow, base theme |
| `docs-tutorial-writer` | `learn/`, `guides/` |
| `docs-reference-writer` | `reference/`, `concepts/` |
| `docs-designer` | CSS theme, landing, logo, diagrams |
| `docs-reviewer` | cross-doc QA, link check, build check |

**Execution: hybrid.** With subagents, spawn in parallel where possible; without subagents, run locally in order.

## Phases

### Phase 1: Architecture (sequential, blocks everything else)
**Owner**: `docs-architect`.
Produces `book.toml`, `SUMMARY.md`, the directory skeleton, and the Pages workflow. Output: `_workspace/done_docs-architect.md` listing routes and gaps.

### Phase 2: Visual identity (parallel with Phase 3)
**Owner**: `docs-designer`.
First delivers `_workspace/palettes.md` with 3 palettes. **The orchestrator pauses and asks the user to pick one** (via `AskUserQuestion`) before applying the theme. Then: theme CSS, logo, favicon, landing.

### Phase 3: Content (parallel)
**Owners**: `docs-tutorial-writer` + `docs-reference-writer`.
They work in disjoint directories — no file conflicts. Each signals completion via `_workspace/done_<agent>.md`.

### Phase 4: Diagrams (depends on Phase 3 + Phase 2)
**Owner**: `docs-designer` (mermaid) in collaboration with `docs-reference-writer` (diagram content).
Diagrams for: architecture, build pipeline, plugin runtime, marker lifecycle.

### Phase 5: Review (sequential, after everything else)
**Owner**: `docs-reviewer`.
Runs a full audit and produces `_workspace/docs-review.md`. If there are blockers → the orchestrator re-invokes the authors (max 2 iterations; afterwards reports to the user).

### Phase 6: Build & Deploy check
- `mdbook build docs/site/` locally with no warnings.
- `mdbook test docs/site/` (validates tagged Rust snippets).
- Validate the Pages workflow with `act` if available, otherwise inspect manually.
- Do not deploy automatically — tell the user "ready to merge; Pages will publish on push to main".

## Data flow

- **`_workspace/`** is the shared scratch area. Each agent writes `done_<agent>.md` when finished.
- Important signals (chosen palette, content gaps) go in dedicated files (`_workspace/palettes.md`, `_workspace/content-gaps.md`).
- The final site lives in `docs/site/`. Do not touch `docs/adr/` or `docs/reference/` (owned by the main harness — only link to / republish).

## Error handling

- An agent fails → 1 retry with the error message.
- It keeps failing → report to the user with file, command, and output. Do not work around it with generic manual edits.
- The review finds a blocker → re-invoke the original author (max 2 iterations). If it persists, document it in `docs-review.md` and move on.
- Design conflict between `docs-designer` and `docs-architect` → the orchestrator sides with `docs-architect` (structure > aesthetics).

## Report

When finished, summarise:
- Files created under `docs/site/` grouped by track (learn/guides/reference/concepts).
- Status of `mdbook build` and `mdbook-linkcheck`.
- Remaining review blockers.
- The publish URL (`https://<org>.github.io/sunscreen/`) — confirm the org with the user.
- Next steps (deploy, screenshot, announcement).

## Re-run / partial requests

When the user asks "only update X":
1. Identify the owner agent.
2. Skip Phase 1 (architecture already exists).
3. Run only the owner agent + `docs-reviewer` at the end (scoped to the changed area).
4. Update the variation log in CLAUDE.md.

## Do not use this skill for

- ADRs → `sunscreen-orchestrator` (agent `docs-writer`).
- CLI feature implementation → `sunscreen-orchestrator`.
- Conceptual Solana questions with no file change → answer directly.
