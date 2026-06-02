---
name: sunscreen-publisher
description: Orchestrates the multi-channel publishing team for the sunscreen CLI — Homebrew, Snap Store, APT — on top of the existing `release.yml` pipeline (cargo-dist, tag-driven). Use WHENEVER the user asks to "publish a release", "distribute via homebrew/snap/apt", "expand the release pipeline", "add a distribution channel", "configure a brew tap", "snapcraft", "cargo-deb", "cloudsmith", "ppa", "apt/brew/snap installer", "automate the release", "publish a new version to the package managers", "make it installable via apt/brew/snap", or any work on `.github/workflows/release.yml` related to distribution channels. Also trigger on requests to "update publish", "fix the X channel publish", "re-run publish", "republish", "bump version on the package managers". DO NOT use for general Rust builds, doctor, or scaffolds — only for the release/distribution axis.
---

# Sunscreen Publisher Harness

## Goal
Extend the existing `release.yml` (cargo-dist binaries to GitHub Releases) so each `vX.Y.Z` tag also publishes to **Homebrew tap**, **Snap Store**, and **APT via Cloudsmith**. The simplest, most efficient path wins — no Launchpad PPA, no building inside the snap, no self-hosted APT repo when Cloudsmith's free tier covers it.

## Team

| Agent | Responsibility |
|--------|------------------|
| `release-orchestrator` | Workflow shape, job ordering, secrets, smoke tests, aggregate docs |
| `homebrew-publisher` | cargo-dist homebrew installer + tap repo |
| `snap-publisher` | classic snapcraft.yaml + snapcore/action-publish |
| `apt-publisher` | cargo-deb + Cloudsmith upload |

## Execution Mode
**Hybrid**: the orchestrator lays the workflow skeleton (sequential, Phase 1) → the 3 publishers work in **parallel** as subagents (Phase 2) → the orchestrator consolidates (Phase 3).

## Phase 0: Context
1. Read the current `.github/workflows/release.yml`.
2. Check `_workspace/done_*-publisher.md` — if any exist, this is a **rerun** (fixing a specific channel). Otherwise, it's an **initial build**.
3. Confirm with the user:
   - Owner of the Homebrew tap repo (e.g. `Pantani/homebrew-sunscreen`)?
   - Is the snap name `sunscreen` available?
   - Cloudsmith org/repo, or use the `gh-pages` APT fallback?

## Phase 1: Skeleton (release-orchestrator)
The orchestrator edits `release.yml` to add 3 stub jobs `publish-homebrew`, `publish-snap`, `publish-apt` with `needs: [host]` and `continue-on-error: true`. Creates `docs/reference/distribution.md` with the secrets table.

## Phase 2: Publishers (parallel)
Spawn in parallel via `Agent` with `run_in_background: true`:
- `homebrew-publisher` → fills the job in + edits `Cargo.toml` (`[workspace.metadata.dist] installers/tap`).
- `snap-publisher` → creates `snap/snapcraft.yaml` + fills the job in.
- `apt-publisher` → adds `[package.metadata.deb]` + fills the job in.

Each one drops `_workspace/done_*-publisher.md`.

## Phase 3: Consolidation (release-orchestrator)
1. Reads the 3 `done_*` files.
2. Resolves any version-string conflict.
3. Adds a `release-summary` step that aggregates the 3 channels' status.
4. Updates `README.md` with badges + install commands.
5. Adds an entry to `CHANGELOG.md`.
6. Runs `cargo dist plan` locally to validate config (no push).

## Phase 4: Validation
Before asking for merge:
- [ ] `cargo dist plan` exits 0.
- [ ] `actionlint .github/workflows/release.yml` exits 0.
- [ ] List of required secrets published in `docs/reference/distribution.md`.
- [ ] A smoke step for each job is documented.
- [ ] A test plan with a `v0.0.0-test.N` tag is in place (dry-run via `workflow_dispatch`).

## Non-Negotiable Principles
- **A channel failure does not block the release** (`continue-on-error` + summary).
- **Same version across all channels** — derived from the tag.
- **Zero rebuild** in channels that can consume the binary from the release (snap, apt).
- **Idempotent** — rerunning the same tag does not duplicate or break anything.
- **Secrets never in logs** — use `${{ secrets.* }}` and default masking.

## Re-execution
When the user asks "republish the snap" or "fix homebrew":
1. Phase 0 detects an existing `done_*`.
2. The orchestrator identifies the affected channel.
3. Only that channel's publisher is invoked.
4. The other channels are skipped.

## Test Scenarios
- **Happy path**: tag `v0.2.0` → 3 channels publish → `apt/brew/snap install sunscreen` returns `sunscreen 0.2.0` in clean containers.
- **Failure**: Snap Store returns 503 → the workflow finishes with a summary showing `snap: failed`, `brew: ok`, `apt: ok` — the release is not deleted.
