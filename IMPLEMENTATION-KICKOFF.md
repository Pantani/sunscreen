# `solis` — Implementation Kickoff Checklist

Companion to **ADR-0001**. Tactical artifact: what to do in the first hours, days, and weeks. Reference-only; the ADR is the source of truth for design.

---

## Pre-Phase 0 — Hour Zero (do these before writing any Go)

### 1. Name & registry reservation

```bash
# Verify availability
gh repo view solis 2>/dev/null && echo "TAKEN" || echo "free"
npm view solis 2>/dev/null && echo "TAKEN" || echo "free"
cargo search solis --limit 5
```

If `solis` is taken on the relevant registries, candidates in priority order: `heliós`, `aurum`, `solid`, `kindle`. Decide before any branding work.

### 2. Repository creation

```bash
# Suggested org name: solis-cli (matches `ignite/cli` pattern)
gh repo create solis-cli/solis --public \
  --description "Solana CLI scaffolding & orchestration tool, inspired by Ignite CLI" \
  --license MIT
gh repo create solis-cli/apps --public \
  --description "Plugin registry for solis (analogous to ignite/apps)"
gh repo create solis-cli/templates --public \
  --description "Workspace and program templates consumed by solis"
gh repo create solis-cli/docs --public \
  --description "solis documentation site (Docusaurus or Nextra)"
```

### 3. Initial commit content

```
solis/
├── .editorconfig
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                 # unit + lint job only at this stage
│   │   └── release.yml            # goreleaser triggered on tags
│   ├── CODEOWNERS
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
├── .gitignore
├── .golangci.yml
├── .goreleaser.yml
├── CONTRIBUTING.md
├── LICENSE                        # MIT
├── README.md                      # one-paragraph + link to ADR
├── go.mod                         # module github.com/solis-cli/solis
├── Makefile
├── cmd/solis/main.go              # stub: prints `solis 0.0.0`
├── docs/
│   └── adr/
│       └── ADR-0001-solis-cli.md (moved to docs/adr/)  # ← committed from this output
└── internal/
    └── version/
        └── version.go             # const Version = "0.0.0-dev"
```

### 4. Makefile baseline

```makefile
SHELL := /bin/bash
GO ?= go
VERSION := $(shell git describe --tags --always --dirty 2>/dev/null || echo "0.0.0-dev")
LDFLAGS := -X github.com/solis-cli/solis/internal/version.Version=$(VERSION)

.PHONY: build test lint fmt cover golden compile integration e2e

build:
	$(GO) build -ldflags="$(LDFLAGS)" -o bin/solis ./cmd/solis

test:
	$(GO) test -race -coverprofile=cov.out ./internal/...

lint:
	golangci-lint run --timeout=5m

fmt:
	gofumpt -w .

cover: test
	$(GO) tool cover -html=cov.out -o coverage.html

golden:
	$(GO) test -tags=golden ./test/golden/...

compile:
	$(GO) test -tags=compile -timeout=20m ./test/compile/...

integration:
	$(GO) test -tags=integration -timeout=30m ./test/integration/...

e2e:
	$(GO) test -tags=e2e -timeout=45m ./test/e2e/...

ci-required: lint test golden
ci-all: ci-required compile integration e2e
```

---

## Phase 0 — Week 1 Checklist

### Monday — repo & CLI skeleton

- [ ] `git init`, push initial commit per § 3 above
- [ ] `go mod init github.com/solis-cli/solis`
- [ ] Add `cobra`, `viper`, `goldie`, `testify`, `gofumpt` as deps
- [ ] `cmd/solis/main.go` calls `cli.Execute()` from `internal/cli`
- [ ] `internal/cli/root.go` defines root cobra command with persistent flags `--verbose`, `--workdir`, `--config`
- [ ] `solis version` subcommand prints build info
- [ ] `go build && ./bin/solis version` works
- [ ] CI workflow runs `make ci-required` on push & PR

### Tuesday — config + toolchain

- [ ] `internal/config/schemas/solis.v1.json` — JSON Schema for solis.yml (start strict, evolve)
- [ ] `internal/config/config.go` — Go struct mirroring the schema
- [ ] `internal/config/loader.go` — viper integration; supports env overrides (`SOLIS_*`)
- [ ] `internal/toolchain/detect.go` — version detection for `anchor`, `solana`, `cargo`, `rustc`, `pnpm`, `node`, `surfpool`, `codama`
- [ ] Unit tests for config parse (valid + invalid fixtures)

### Wednesday — `solis doctor`

- [ ] `internal/cli/doctor.go` — runs all toolchain detections, prints table
- [ ] Table output via `lipgloss` (start simple; bubble tea comes later)
- [ ] Exit code 2 if any required tool missing
- [ ] Unit tests with mocked toolchain reports

### Thursday — golden infra & templates layout

- [ ] `internal/templates/embed.go` — `//go:embed assets/*` root
- [ ] `internal/templates/funcs.go` — register sprig + custom funcs (pascal, camel, snake)
- [ ] `internal/templates/render.go` — `Render(name string, data any) ([]byte, error)`
- [ ] `test/golden/` — directory layout established; one trivial test
- [ ] `make golden` works

### Friday — buffer day

- [ ] Address any blockers from M-Th
- [ ] First retrospective; adjust ADR open questions
- [ ] Tag `v0.0.1-phase0-week1`
- [ ] Post-week summary in repo discussions

---

## Phase 0 — Week 2 Checklist

### Goals

- [ ] `solis.yml` round-trip: load → validate → serialize → load again, no drift
- [ ] Migration framework (`internal/config/migrator.go`) — even though v1 only, the framework exists
- [ ] `goreleaser` produces binaries for linux/amd64, linux/arm64, darwin/amd64, darwin/arm64
- [ ] Cold start `solis --help` measured < 50 ms (CI bench)
- [ ] `solis doctor` works on macos-14 in CI
- [ ] ADR-0002 written: CLI design conventions (flag naming, error formatting, exit codes)
- [ ] ADR-0003 written: documentation strategy (Docusaurus? Nextra? Astro Starlight?)

---

## Bibliography for the First Week of Implementation

Read these before / during week 1:

1. **Ignite CLI source** — especially `ignite/services/scaffolder`, `ignite/services/chain`, `ignite/services/plugin`. Don't copy verbatim; understand the patterns.
2. **`create-solana-program` source** — particularly the template variables and post-generation hooks.
3. **Anchor 1.0 source** — `cli/src/template.rs` and `cli/src/lib.rs` for what `anchor init -t multiple` actually generates.
4. **Codama README + `nodes-from-anchor`** — for understanding the IDL → tree → renderer pipeline you'll wrap.
5. **Surfpool docs** — particularly its config file format and stdout JSON event stream (subscription target for TUI).
6. **bubble tea examples** — start with `examples/spinner` and `examples/realtime`.
7. **HashiCorp go-plugin "trivial" example** — for the gRPC plugin pattern that ships in Phase 6.

---

## Decision Log (for tracking during implementation)

Keep a `docs/decision-log.md` with one-liner per non-trivial choice that doesn't warrant a full ADR. Examples:

- "Switched template engine from `text/template` to `html/template` for frontend files to escape HTML automatically" — link to commit
- "Adopted `samber/lo` instead of stdlib for `lo.Map` ergonomics" — link to discussion

When > 5 entries reference the same area, promote to a real ADR.

---

## Risk Register Mirror

Mirror ADR § 11 in `RISKS.md` at repo root, with one column added: **Status** (`open` / `mitigated` / `realized` / `closed`). Update on every retro.

---

*End of kickoff checklist.*
