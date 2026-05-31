# cli-architect — Week 2 Done

## Scope delivered

1. **CI** — `.github/workflows/ci.yml`
   - Jobs: `fmt`, `clippy`, `test`, `build`, `bench` (non-blocking).
   - Matrix `ubuntu-latest` + `macos-latest` for clippy/test/build.
   - `Swatinem/rust-cache@v2` on every job that compiles.
   - Triggers: push on `main`, all PRs targeting `main`.

2. **Release** — `.github/workflows/release.yml`
   - Trigger: tags `v[0-9]+.[0-9]+.[0-9]+*` (+ workflow_dispatch).
   - Uses **cargo-dist 0.22.1**: jobs `plan` → `build` (matrix of 4 targets) → `publish` (draft GitHub release).
   - Cross-compile shim for `aarch64-unknown-linux-gnu` via `gcc-aarch64-linux-gnu`.
   - Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.

3. **cargo-dist config** — added to `Cargo.toml`
   - `[workspace.metadata.dist]` with cargo-dist 0.22.1, installers `shell` + `powershell`, the 4 targets, `pr-run-mode = "plan"`.
   - Added `[profile.dist]` inheriting from `release`.
   - Enriched `[package]` metadata (`repository`, `homepage`, `readme`, `keywords`, `categories`) which cargo-dist requires.

4. **Cold-start bench**
   - `benches/cold_start.rs` — standalone harness (`harness = false`), runs `sunscreen --help` 10x, reports mean/p95/min/max, warns if p95 > 50 ms but never fails.
   - `scripts/bench.sh` — shell wrapper (uses `python3` for portable nanosecond timing on macOS + Linux). Builds release if missing. Always exits 0.
   - Registered as `[[bench]] name = "cold_start"` in `Cargo.toml`.
   - Local measurement: **mean ~3.2 ms, p95 ~3.6 ms** (well under 50 ms target).

5. **CODEOWNERS** — `.github/CODEOWNERS` with `* @sunscreen-cli/maintainers`.

6. **PR template** — `.github/pull_request_template.md` (Summary / Changes / Checklist / Related Issues).

7. **Issue templates** — `.github/ISSUE_TEMPLATE/bug_report.md` and `feature_request.md`.

8. **Version bump** — `Cargo.toml` `version = "0.0.0"` → `"0.1.0-dev"`.

## Files created / modified

Created:
- `/Users/pantani/Desktop/projects/rust/sunscreen/.github/workflows/ci.yml`
- `/Users/pantani/Desktop/projects/rust/sunscreen/.github/workflows/release.yml`
- `/Users/pantani/Desktop/projects/rust/sunscreen/.github/CODEOWNERS`
- `/Users/pantani/Desktop/projects/rust/sunscreen/.github/pull_request_template.md`
- `/Users/pantani/Desktop/projects/rust/sunscreen/.github/ISSUE_TEMPLATE/bug_report.md`
- `/Users/pantani/Desktop/projects/rust/sunscreen/.github/ISSUE_TEMPLATE/feature_request.md`
- `/Users/pantani/Desktop/projects/rust/sunscreen/benches/cold_start.rs`
- `/Users/pantani/Desktop/projects/rust/sunscreen/scripts/bench.sh` (chmod +x)

Modified:
- `/Users/pantani/Desktop/projects/rust/sunscreen/Cargo.toml`
  - Version bump 0.0.0 → 0.1.0-dev
  - Package metadata (repo/homepage/readme/keywords/categories)
  - `[[bench]] cold_start` entry
  - `[profile.dist]` + `[workspace.metadata.dist]`

## Verification

- `cargo build --release` — OK (16.6 s)
- `cargo bench --bench cold_start --no-run` — compiles OK
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `bash scripts/bench.sh` — mean 3.22 ms, p95 3.58 ms

## Hand-offs / coordination

- Did not touch `docs/` or `src/config/` (left to docs-writer / config-engineer).
- Release workflow assumes `cargo-dist init` was *not* run — it pins v0.22.1 explicitly and is self-contained. When the team is ready to switch to fully generated `cargo dist` CI, run `cargo dist init && cargo dist generate` and overwrite `release.yml`.
- CODEOWNERS team `@sunscreen-cli/maintainers` is a placeholder; replace once the GitHub org/team exists.
