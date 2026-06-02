# ADR-0003 — Documentation Strategy for `sunscreen`

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-05-31 |
| **Authors** | Danilo Lacombe |
| **Tags** | documentation, mdbook, github-pages, devx |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | ADR-0001 (sunscreen CLI), ADR-0002 (CLI Design Conventions) |

---

## TL;DR

`sunscreen` ships a static documentation site built with **mdBook**, hosted on **GitHub Pages**, sourced from `docs/` in this repository. mdBook is selected over Docusaurus, Astro Starlight, and Nextra because: (1) it is Rust-native and matches the project's toolchain — contributors already have `cargo`; (2) it produces a single static binary's worth of HTML with built-in client-side search; (3) Markdown is plain CommonMark with no MDX, keeping content portable and renderable on GitHub without the site; (4) it composes well with `mdbook-cmdrun` and `mdbook-include` to embed live `--help` output and ADRs without duplication; (5) versioned docs (one site per minor release) are trivially achieved with parallel `gh-pages` subdirectories.

The site's initial information architecture is: **Introduction → Quick Start → Concepts → Commands → Recipes → ADRs**. The build pipeline is a GitHub Actions workflow that runs `mdbook build` and deploys to `gh-pages`. i18n is explicitly out of scope for v1.

---

## 1. Context

### 1.1 Problem framing

`sunscreen` is a scaffolding and orchestration CLI whose value depends on discoverability. A Solana developer evaluating the tool must, within 60 seconds, find:

- What it does (one paragraph).
- How to install it (one command).
- A working "scaffold → build → test" loop (one quick-start page).
- Reference for every subcommand and flag (auto-generated from clap).
- A few opinionated recipes (CRUD, Token-2022, indexer wiring) that demonstrate the tool's reach.

Without dedicated docs infrastructure, this content lives in a single `README.md` that eventually balloons to thousands of lines, becomes hostile to navigation, and discourages contribution. ADR-0001 § 6 (Command Surface) alone implies dozens of subcommands; documenting each in README is untenable.

The `README.md` currently in the repo is 98 bytes — a placeholder. The first real prose lives in `ADR-0001-solis-cli.md` (84 KB) and this `docs/adr/` directory. We need a publication path before the surface grows further.

### 1.2 Audiences

- **First-time visitor**: needs a marketing-grade landing page and a 5-minute Quick Start.
- **Active user**: needs command reference, recipes, and a search box.
- **Plugin author** (future, ADR-0001 § 7.5): needs protocol specs and code-level reference.
- **Contributor**: needs ADRs, architecture docs, and the rendered site to verify their PR didn't break anything.
- **AI agent** (Claude Code, Cursor, etc.): consumes the site through `llms.txt`-style flattened markdown or through `--help --json`. Both must be cheap to produce.

### 1.3 Constraints

- **Single repo.** Docs live in `docs/` next to the code so a PR can change both atomically.
- **No Node toolchain required for contributors who only edit prose.** Rust contributors should not learn a JS build system to write a Markdown page.
- **GitHub Pages as the host.** Free, supports custom domains, integrates with Actions.
- **Search must work offline / client-side.** No Algolia DocSearch dependency at MVP (it's pay-walled below a threshold and adds a network round-trip).
- **Markdown must be portable.** A user reading `docs/src/quick-start.md` directly on github.com should see substantially the same content as on the site. This rules out MDX-heavy frameworks.
- **Versioning must be possible** when the CLI hits 1.0 — one site per supported minor release.

---

## 2. Decision Drivers

- **DD1 — Toolchain locality.** A Rust project's docs should build with `cargo`-adjacent tooling, not require a separate `node_modules` reckoning on every contributor's machine.
- **DD2 — Plain CommonMark.** Content must render on github.com unchanged. No MDX, no custom React components in source files.
- **DD3 — Embedded examples are first-class.** Embedding the live output of `sunscreen --help` and the live text of ADR files into the site must be a one-line directive, not a copy-paste maintenance burden.
- **DD4 — Client-side search out of the box.** No external service, no per-build index upload step at MVP.
- **DD5 — Static output on GitHub Pages.** No server, no JS framework runtime required for navigation.
- **DD6 — Versioning by directory.** `https://sunscreen.dev/v0.3/` and `https://sunscreen.dev/v0.4/` coexist via `gh-pages` subdirectories — no CMS, no migration script.
- **DD7 — Build is fast and CI-friendly.** A docs PR should take well under a minute from push to preview link.
- **DD8 — Syntax highlighting for Rust and TypeScript code blocks** is mandatory; the docs will show both languages constantly.

---

## 3. Considered Options

Four static-site generators were evaluated against DD1–DD8.

### 3.1 Option A — mdBook

[mdBook](https://rust-lang.github.io/mdBook/) is the Rust project's own docs tool (used by *The Rust Programming Language*, *The Cargo Book*, the *Rust Reference*, and most Rust subproject docs).

| DD | mdBook |
|---|---|
| DD1 (toolchain) | ✅ `cargo install mdbook`; one binary, no Node |
| DD2 (CommonMark) | ✅ Pure CommonMark + a handful of conventional extensions (footnotes, tables) |
| DD3 (embedding) | ✅ via [`mdbook-cmdrun`](https://github.com/FauconFan/mdbook-cmdrun) (run a shell command, embed output) and [`mdbook-include`](https://crates.io/crates/mdbook-include) (include arbitrary files) |
| DD4 (search) | ✅ Built-in client-side search via `elasticlunr.js` (no config required) |
| DD5 (static) | ✅ Pure static HTML/CSS/JS |
| DD6 (versioning) | ✅ Trivial: build into `gh-pages/v0.3/`, `gh-pages/v0.4/`, with a root index redirecting to latest |
| DD7 (build speed) | ✅ Hundreds of pages build in < 1 s |
| DD8 (highlight) | ✅ Highlight.js bundled; Rust and TypeScript both supported |

Bonus: well-known plugin ecosystem — [`mdbook-pagetoc`](https://github.com/JorelAli/mdBook-pagetoc) (per-page TOC sidebar), [`mdbook-linkcheck`](https://github.com/Michael-F-Bryan/mdbook-linkcheck), [`mdbook-mermaid`](https://github.com/badboy/mdbook-mermaid). All are themselves Rust binaries, `cargo install`-friendly.

Drawbacks: theming is constrained (the default theme is functional but not "marketing site"–level; customization requires editing CSS). No first-class i18n (deferred per § 6).

### 3.2 Option B — Docusaurus

[Docusaurus](https://docusaurus.io/) is Meta's React-based docs framework.

| DD | Docusaurus |
|---|---|
| DD1 | ❌ Requires Node + a JS package manager; every contributor must `npm install` |
| DD2 | ❌ Defaults to MDX, which breaks GitHub's renderer; CommonMark mode exists but loses the headline features |
| DD3 | ⚠️ Possible via custom React components; not a one-liner |
| DD4 | ⚠️ Local search via plugin; Algolia DocSearch is the recommended path (external service) |
| DD5 | ✅ Static build |
| DD6 | ⚠️ Versioning supported but requires per-release directory snapshots in the repo, doubling diff noise |
| DD7 | ⚠️ Build is heavier; cold builds in CI take 30–90 s for medium sites |
| DD8 | ✅ Prism-based |

Powerful but heavy. Wins on visual polish; loses on every other driver.

### 3.3 Option C — Astro Starlight

[Astro Starlight](https://starlight.astro.build/) is a docs theme on top of Astro.

| DD | Starlight |
|---|---|
| DD1 | ❌ Node toolchain |
| DD2 | ⚠️ Markdown + MDX; portable if MDX is avoided, but the templates use it |
| DD3 | ⚠️ Possible via Astro components |
| DD4 | ✅ Built-in Pagefind integration (good) |
| DD5 | ✅ Static |
| DD6 | ⚠️ No first-class versioning; manual directory build |
| DD7 | ⚠️ Cold build 10–30 s |
| DD8 | ✅ Shiki |

Beautiful default theme; first-class i18n; modern stack. Loses on DD1 and DD2 — the cost of forcing every Rust contributor to learn Astro is too high for a CLI project.

### 3.4 Option D — Nextra

[Nextra](https://nextra.site/) is a Next.js-based docs framework.

| DD | Nextra |
|---|---|
| DD1 | ❌ Next.js + Node |
| DD2 | ❌ MDX-first |
| DD3 | ⚠️ Possible via JSX |
| DD4 | ✅ FlexSearch built-in |
| DD5 | ✅ Static export supported |
| DD6 | ❌ No versioning story |
| DD7 | ❌ Next.js cold builds are slow |
| DD8 | ✅ Shiki |

Rejected on toolchain mismatch.

### 3.5 Decision matrix

| Driver | mdBook | Docusaurus | Starlight | Nextra |
|---|---|---|---|---|
| DD1 toolchain | ✅ | ❌ | ❌ | ❌ |
| DD2 CommonMark | ✅ | ❌ | ⚠️ | ❌ |
| DD3 embed | ✅ | ⚠️ | ⚠️ | ⚠️ |
| DD4 search | ✅ | ⚠️ | ✅ | ✅ |
| DD5 static | ✅ | ✅ | ✅ | ✅ |
| DD6 versioning | ✅ | ⚠️ | ⚠️ | ❌ |
| DD7 speed | ✅ | ⚠️ | ⚠️ | ❌ |
| DD8 highlight | ✅ | ✅ | ✅ | ✅ |

mdBook wins on every driver that has a clear winner.

---

## 4. Decision

Adopt **mdBook** as the documentation generator. Host on **GitHub Pages**. Source lives in `docs/` within this repository.

### 4.1 Repository layout

```
docs/
├── adr/                    # this directory; ADRs as Markdown
│   ├── ADR-0001-...md      # currently at repo root; will move here in a follow-up
│   ├── ADR-0002-cli-design-conventions.md
│   └── ADR-0003-documentation-strategy.md   ← this file
├── book.toml               # mdBook config (renderer, plugins, theme)
└── src/
    ├── SUMMARY.md          # left-nav table of contents
    ├── introduction.md
    ├── quick-start.md
    ├── concepts/
    │   ├── workspace.md
    │   ├── program.md
    │   └── scaffold.md
    ├── commands/
    │   ├── overview.md
    │   ├── scaffold.md
    │   ├── chain.md
    │   ├── generate.md
    │   ├── app.md
    │   └── doctor.md
    ├── recipes/
    │   ├── token-2022.md
    │   └── crud.md
    └── adrs.md             # imports ../adr/*.md via mdbook-include
```

The `docs/adr/` directory is the canonical home for ADRs. `docs/src/adrs.md` is a thin index that includes each ADR via `mdbook-include` directives so the website surfaces ADRs without duplicating their content.

### 4.2 `book.toml` (initial)

```toml
[book]
title = "sunscreen"
authors = ["Danilo Lacombe"]
description = "Solana CLI scaffolding & orchestration tool"
src = "src"
language = "en"

[output.html]
default-theme = "navy"
preferred-dark-theme = "navy"
git-repository-url = "https://github.com/Pantani/sunscreen"
edit-url-template = "https://github.com/Pantani/sunscreen/edit/main/docs/{path}"

[output.html.search]
enable = true
limit-results = 30
use-boolean-and = true

[preprocessor.cmdrun]
# mdbook-cmdrun: embed `sunscreen --help` etc.

[preprocessor.include]
# mdbook-include: pull in ../adr/*.md

[preprocessor.pagetoc]
# mdbook-pagetoc: per-page right-side TOC

[output.linkcheck]
# mdbook-linkcheck: fail CI on broken internal links
follow-web-links = false
```

### 4.3 Commands page strategy

The `docs/src/commands/*.md` pages embed live CLI output:

```markdown
## `sunscreen doctor`

<!-- cmdrun cargo run --quiet -- doctor --help -->
```

`mdbook-cmdrun` re-runs the command on every build, so the rendered help text never drifts from the actual implementation. As a fallback for environments where running the binary at docs-build time is undesirable (no toolchain), manual transcription is acceptable but must be re-verified per release.

ADR-0002 § 4.2 lists the verbs (`scaffold`, `chain`, `generate`, `app`, `doctor`, `version`); each gets its own page. The overview page tabulates them with one-line descriptions.

### 4.4 ADR rendering

`docs/src/adrs.md` reads:

```markdown
# Architecture Decision Records

{{#include ../adr/ADR-0001-solis-cli.md}}

---

{{#include ../adr/ADR-0002-cli-design-conventions.md}}

---

{{#include ../adr/ADR-0003-documentation-strategy.md}}
```

ADRs remain plain Markdown files navigable on github.com. The site surfaces them without forking content. New ADRs require one line added to `SUMMARY.md` and one `{{#include}}` line.

### 4.5 Build and deploy pipeline

A GitHub Actions workflow (`.github/workflows/docs.yml`) runs on pushes to `main` and on tags:

1. Checkout.
2. `cargo install mdbook mdbook-cmdrun mdbook-include mdbook-pagetoc mdbook-linkcheck` (cached via `actions/cache` on `~/.cargo/bin`).
3. `cargo build --quiet` so `mdbook-cmdrun` can invoke the binary.
4. `mdbook build docs/`.
5. Deploy `docs/book/` to `gh-pages` branch using `peaceiris/actions-gh-pages` or `actions/deploy-pages`.

For tagged releases (`v0.3.0`, `v0.4.0`, …), the workflow deploys to `gh-pages/v0.3/`, `gh-pages/v0.4/`, etc. The site root (`gh-pages/index.html`) is a tiny redirect to the latest stable version. Pre-release tags publish to `gh-pages/next/`.

PR builds run mdBook through `mdbook test` (link checking + code-block compilation where annotated) but do **not** deploy; preview deploys may be added later via Cloudflare Pages or Netlify if needed (out of scope here).

### 4.6 Initial content scope (v0)

| Page | Owner | Notes |
|---|---|---|
| `introduction.md` | docs-writer | What/why; one paragraph + bullet list |
| `quick-start.md` | docs-writer | `cargo install sunscreen` → `sunscreen scaffold …` → `sunscreen chain serve` |
| `concepts/workspace.md` | docs-writer | Mirror ADR-0001 § 5 high-level layout |
| `concepts/program.md` | docs-writer | Anchor `multiple` template per ADR-0001 § 7.3 |
| `concepts/scaffold.md` | docs-writer | Marker-bound editing per ADR-0001 § 7.1 |
| `commands/*.md` | docs-writer | One page per top-level verb |
| `recipes/token-2022.md` | future | Phase 5 |
| `recipes/crud.md` | future | Phase 3 |
| `adrs.md` | docs-writer | `{{#include}}` indirection |

---

## 5. Consequences

### 5.1 Positive

- **Zero new toolchain for Rust contributors.** `cargo install mdbook` and you can preview locally with `mdbook serve`.
- **Live `--help` in docs.** Help text never goes stale because `mdbook-cmdrun` regenerates it on every build.
- **ADRs are single-sourced.** They render on github.com as plain Markdown and on the site via include — no copy-paste drift.
- **Search works offline.** The published HTML is browsable from a local clone with `python -m http.server`.
- **CI minute cost is trivial.** A full docs build is sub-minute.
- **Versioning is mechanical.** New release tag → new subdirectory on `gh-pages`. No CMS state to manage.
- **github.com remains a first-class viewer.** Users browsing the repo see the same content as visitors to the site.

### 5.2 Negative

- **Theming ceiling.** mdBook's default theme is utilitarian; reaching "Stripe-grade" visual polish requires custom CSS and possibly a forked theme. Acceptable trade-off for v1; revisit if the project's marketing surface grows.
- **No MDX.** Interactive widgets (live playgrounds, embedded REPLs) are not possible without escaping into raw HTML. Acceptable: ADR-0001's scope does not include browser-based interactivity.
- **Plugin ecosystem is smaller** than Docusaurus's. We depend on `mdbook-cmdrun`, `mdbook-include`, `mdbook-pagetoc`, `mdbook-linkcheck` — all maintained, but if any goes unmaintained we are responsible. Mitigated: each plugin is a small Rust binary; forking is realistic.
- **No first-class i18n.** Translating docs requires manual directory cloning or third-party plugins. Explicitly deferred (§ 6).
- **mdBook search is `elasticlunr.js`-based** — adequate for hundreds of pages, less so for thousands. We will not exceed that scale before considering Pagefind (drop-in replacement that works post-build).

---

## 6. Out of Scope (for v1)

- **Internationalization (i18n).** English only. Translation tooling for mdBook exists (`mdbook-i18n-helpers` used by *The Rust Book*) but adds complexity. Revisit when there is concrete demand and a translator team.
- **Algolia DocSearch.** Client-side search is sufficient at MVP scale.
- **Interactive code playgrounds.** Out of scope; users run `sunscreen scaffold` locally instead.
- **PR preview deploys.** A nice-to-have, deferred.
- **API reference (`cargo doc`)** for the `sunscreen` library crate is published separately on docs.rs and linked from the introduction page, not bundled into the mdBook site.

---

## 7. Open Questions

- **OQ1** — Custom domain? `sunscreen.dev` is desirable but unregistered as of this writing. Decision deferred until pre-1.0; until then, `https://sunscreen-cli.github.io/sunscreen/` is the canonical URL.
- **OQ2** — Should `commands/*.md` pages be auto-generated from clap (via a `xtask` running `clap_mangen`-like extraction) instead of curated? Auto-generation guarantees freshness but loses prose explanation. Current plan: hand-write the prose, embed help via `mdbook-cmdrun` for the synopsis. Revisit if drift becomes a chore.
- **OQ3** — Where do `llms.txt` / agent-friendly flattened markdown variants live? Likely a post-build step that concatenates `docs/src/**/*.md` into `book/llms.txt`. Defer until first agent integration request.
- **OQ4** — Should ADR-0001 (currently 84 KB at repo root) move into `docs/adr/`? Mechanically yes; the move is a separate PR to keep this ADR reviewable. Tracked.
- **OQ5** — Versioning trigger: every release tag, or only minor tags? Patch releases rarely change docs; current proposal is **minor only** (`v0.3`, `v0.4`, …) with `next/` tracking `main`.
- **OQ6** — Diátaxis framework alignment (Tutorial / How-to / Reference / Explanation) is implicit in the IA above but not enforced. Worth a follow-up audit once content lands.

---

## 8. References

- [mdBook documentation](https://rust-lang.github.io/mdBook/)
- [mdbook-cmdrun](https://github.com/FauconFan/mdbook-cmdrun)
- [mdbook-include](https://crates.io/crates/mdbook-include)
- [mdbook-pagetoc](https://github.com/JorelAli/mdBook-pagetoc)
- [mdbook-linkcheck](https://github.com/Michael-F-Bryan/mdbook-linkcheck)
- [Docusaurus](https://docusaurus.io/)
- [Astro Starlight](https://starlight.astro.build/)
- [Nextra](https://nextra.site/)
- [Zola](https://www.getzola.org/) — Rust SSG; considered as a more flexible alternative but rejected because docs-specific features (TOC, search, theme) would need to be assembled by hand.
- [Diátaxis framework](https://diataxis.fr/) — IA reference.
- ADR-0001 § 5, § 6, § 7 (architecture and surface that the docs must mirror).
- ADR-0002 (CLI conventions that the `commands/` section documents).
