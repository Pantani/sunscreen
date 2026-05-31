# docs-writer — DONE

**Date:** 2026-05-31
**Agent:** docs-writer

## Deliverables

Two ADRs written under `docs/adr/`:

1. **`docs/adr/ADR-0002-cli-design-conventions.md`** — CLI conventions for `sunscreen`.
   - Status: Proposed
   - Covers flag naming (kebab-case longs, single-letter shorts, `--verbose=count`, `--json` bool), subcommand naming (plain verbs, no snake_case, depth ≤ 3), exit codes 0/1/2/3/4 mapped to `SunscreenError::exit_code` in `src/error.rs`, error formatting (human `error: …` vs `--json` `{error, kind}` per `src/cli/root.rs::execute`), tables via `comfy-table`, color via `owo-colors` with TTY auto-detect + `NO_COLOR`/`--no-color`, verbosity (`-q`/`-v`/`-vv`/`-vvv`; default `WARN`), stdin/stdout piping (`-` convention for `generate`), configuration precedence (flag > env `SUNSCREEN_*` > `sunscreen.yml` > defaults).
   - Considered Options: clap derive (selected) vs builder vs argh vs hand-rolled; `thiserror` + `anyhow` (selected) vs `eyre` vs `anyhow`-only vs custom enum; `owo-colors` (selected) vs `colored` vs `termcolor`; `comfy-table` (selected) vs `tabled` vs manual.
   - Decision: clap derive at boundary + `thiserror` for public `SunscreenError` + `anyhow` internally.

2. **`docs/adr/ADR-0003-documentation-strategy.md`** — Documentation site strategy.
   - Status: Proposed
   - Evaluates mdBook, Docusaurus, Astro Starlight, Nextra against 8 decision drivers (toolchain locality, CommonMark portability, embedded examples, client-side search, static GitHub Pages output, directory-based versioning, build speed, syntax highlighting).
   - Decision: **mdBook** + GitHub Pages, with `mdbook-cmdrun` (live `--help` embedding), `mdbook-include` (single-source ADRs), `mdbook-pagetoc`, `mdbook-linkcheck`.
   - IA: Introduction → Quick Start → Concepts (Workspace/Program/Scaffold) → Commands (one page per verb from ADR-0002 § 4.2) → Recipes → ADRs.
   - Deploy: GitHub Actions builds and pushes to `gh-pages`, one subdirectory per minor release (`v0.3/`, `v0.4/`, `next/`).
   - Out of scope: i18n, Algolia DocSearch, interactive playgrounds, PR preview deploys.

## Style notes

- Both ADRs follow the ADR-0001 meta-table format (Status, Date, Authors, Tags, Supersedes, Superseded by, Related).
- Both include: TL;DR, Context (problem framing), Decision Drivers (DD1..DDn), Considered Options (≥ 2), Decision, Consequences (positive + negative), Open Questions, References.
- Length: ADR-0002 ≈ 280 lines, ADR-0003 ≈ 320 lines — within the 300–500 target band.
- Concrete references to in-repo source: `src/error.rs`, `src/cli/root.rs`, `src/cli/doctor.rs`, `Cargo.toml` dependencies (clap, thiserror, anyhow, comfy-table, owo-colors, insta).
- Concrete tool references: `mdbook`, `mdbook-cmdrun`, `mdbook-include`, `mdbook-pagetoc`, `mdbook-linkcheck`, `peaceiris/actions-gh-pages`, `cargo-dist`.

## Open follow-ups (cross-referenced inside the ADRs)

- ADR-0002 OQ4 / ADR-0003 OQ2: where command reference auto-generation lives.
- ADR-0003 OQ4: move `ADR-0001-solis-cli.md` from repo root into `docs/adr/` as a separate PR.
- ADR-0002 § 4.9: implement `-q`, `--no-color`, `--dry-run` as global flags on `Cli` in `src/cli/root.rs`.
