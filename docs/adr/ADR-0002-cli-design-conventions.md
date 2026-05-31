# ADR-0002 — CLI Design Conventions for `sunscreen`

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-05-31 |
| **Authors** | Danilo Lacombe |
| **Tags** | cli, ux, conventions, clap, ergonomics |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | ADR-0001 (sunscreen CLI), ADR-0003 (Documentation Strategy) |

---

## TL;DR

This ADR codifies the user-facing conventions of the `sunscreen` CLI: flag names, subcommand names, exit codes, error formatting, output channels (human vs `--json`), color, verbosity, stdin/stdout piping, and configuration precedence. The goal is a CLI that feels like `cargo`, `gh`, and `kubectl` rather than the inconsistent patchwork that today's Solana tooling exposes (`anchor` mixes camelCase and kebab-case flags; `mucho` follows yet another style; `solana` CLI has its own conventions).

The chosen stack — already wired in `src/cli/root.rs` and `src/error.rs` — is:

- **`clap` v4 derive** for argument parsing.
- **`thiserror`** for the public `SunscreenError` boundary; **`anyhow`** for internal propagation.
- **`comfy-table`** for tables and **`owo-colors`** for color, both with TTY auto-detection and `--no-color`/`NO_COLOR` respect.
- **Exit codes** `0/1/2/3/4` matching `SunscreenError::exit_code` in `src/error.rs`.
- **`--json`** as a global boolean toggle that switches both successful structured output (where supported) and error output into a stable `{ error, kind }` schema (see `src/cli/root.rs::execute`).
- **Configuration precedence** flag > env (`SUNSCREEN_*`) > `sunscreen.yml` > built-in defaults.

These conventions apply to every subcommand currently stubbed in `src/cli/root.rs::Command` (`version`, `doctor`, `scaffold`, `chain`, `generate`, `app`) and any future surface.

---

## 1. Context

### 1.1 Problem framing

`sunscreen` ships as a single binary that will accumulate dozens of subcommands across multiple verbs (`scaffold`, `generate`, `chain`, `app`, `doctor`, …). Without a written convention, contributors will reproduce the inconsistencies that plague the Solana toolchain today:

- `anchor` uses `--provider.cluster` (dotted), `-p` (program), `--skip-lint` (kebab), and inconsistent boolean handling.
- `solana` CLI uses `--url` and `-u`, but also `--json-rpc-url`, with overlapping semantics.
- `mucho` introduces `Solana.toml` keys that do not match any flag spelling, forcing users to learn two vocabularies.
- `cargo` (the gold standard) is rigorously kebab-case, single-letter shorts where idiomatic, `-v/-vv` for verbosity, `--quiet` for inverse.

The CLI is the only artifact the user interacts with — IDE integrations, CI scripts, plugin authors, AI agents, and `sunscreen chain doctor --json` consumers all see this surface. Inconsistency here is a tax on every future feature.

### 1.2 Constraints inherited from ADR-0001

- ADR-0001 § 3 (Option C) chose Go originally; this Rust implementation preserves the same UX commitments: **single binary**, **deterministic output**, **agent-friendly JSON mode**.
- ADR-0001 § 7.7 (Filesystem Transaction Semantics) requires a `--dry-run` story; this ADR specifies how `--dry-run` is spelled across commands.
- ADR-0001 § 7.6 (Configuration Format) settled on YAML for `sunscreen.yml`; this ADR specifies how flags relate to that file.

### 1.3 Current implementation reality

`src/cli/root.rs` already commits to several choices that this ADR formalizes rather than invents:

```rust
#[arg(short, long, global = true, action = clap::ArgAction::Count)]
pub verbose: u8,

#[arg(long, global = true, default_value_t = false)]
pub json: bool,
```

`--verbose` is a counted flag (`-v`, `-vv`, `-vvv`); `--json` is a global boolean. `src/error.rs` defines four error variants mapped to exit codes 1/2/3/4. This ADR documents the rules that produced those choices so the next 200 flags follow the same logic.

---

## 2. Decision Drivers

- **DD1 — Familiarity over novelty.** A developer who knows `cargo`, `gh`, `kubectl`, and `solana` should be able to guess `sunscreen` flag spellings 90% of the time.
- **DD2 — Agent-readable mode is first-class.** `--json` is not an afterthought; every command that prints structured data must support it, and the schema must be stable across patch releases.
- **DD3 — Exit codes are part of the API.** CI and shell pipelines branch on `$?`. Once published, an exit code cannot change meaning without a major version bump.
- **DD4 — Color and table output never break pipelines.** `isatty(stdout)` auto-detect; `NO_COLOR` honored; `--no-color` global flag wins over both.
- **DD5 — Quietness is composable.** `-q` and `-v` cancel each other in well-defined ways; default log level is `WARN` so a successful invocation prints nothing on stderr by default.
- **DD6 — Configuration must be debuggable.** Users must be able to ask "where did this value come from?" — flag, env, file, or default — and get a clear answer.
- **DD7 — Single-binary, no surprise dependencies.** Conventions must be implementable with the crates already in `Cargo.toml` (clap, anyhow, thiserror, comfy-table, owo-colors, serde_json).
- **DD8 — Pipeable.** `sunscreen generate <foo> -` reading from stdin and writing to stdout enables UNIX composition.

---

## 3. Considered Options

### 3.1 Argument parser

| Option | Pro | Con | Verdict |
|---|---|---|---|
| **clap derive** | Type-safe; minimal boilerplate; auto-help; widely used in Rust ecosystem; generates shell completions via `clap_complete` | Macro magic is opaque on errors; slightly higher compile time | **Selected** (already in use) |
| clap builder | Full programmatic control; useful for plugin-driven dynamic subcommands | Verbose; reinvents what derive gives for free; would force every subcommand author to learn the builder API | Rejected for MVP; reconsider when plugin system (ADR-0001 § 7.5) lands |
| `argh` | Tiny, fast compile | Less mature; weaker subcommand support; no env integration | Rejected |
| Hand-rolled | Zero dependencies | Reinvents help, completions, validation | Rejected |

### 3.2 Error handling library

| Option | Pro | Con | Verdict |
|---|---|---|---|
| **`thiserror` at the boundary + `anyhow` internally** | `thiserror` gives stable `kind_str()` for JSON; `anyhow` keeps internal call sites ergonomic with `?` | Two crates instead of one | **Selected** (already wired in `src/error.rs`) |
| `eyre` everywhere | One crate; nicer reports | Less established; harder to expose a stable error taxonomy to `--json` consumers | Rejected |
| `anyhow` everywhere | Simplest | No stable discriminant for JSON; every error becomes an opaque string | Rejected |
| Custom enum (no crate) | Full control | Reinvents `#[error]` derive | Rejected |

### 3.3 Color crate

| Option | Pro | Con | Verdict |
|---|---|---|---|
| **`owo-colors`** | No allocation, ergonomic, `if_supports_color` integration, respects `NO_COLOR` | — | **Selected** (already in `Cargo.toml`) |
| `colored` | Familiar API | Less performant; global state | Rejected |
| `termcolor` | Cargo uses it | More verbose API | Rejected |

### 3.4 Table renderer

| Option | Pro | Con | Verdict |
|---|---|---|---|
| **`comfy-table`** | Unicode-aware; presets; width handling | Pulls in `strum` | **Selected** (already in `Cargo.toml`) |
| `tabled` | Derive-based | Heavier; macro-first | Rejected |
| Manual `println!` | Zero deps | Re-invents alignment | Rejected for any tabular output |

---

## 4. Decision

The conventions below are **normative**. Any subcommand violating them is a bug.

### 4.1 Flag naming

- **Long flags are kebab-case.** `--dry-run`, `--skip-deps`, `--no-color`, `--allow-breaking`. Never `--dryRun` or `--dry_run`.
- **Short flags are single ASCII letters**, allocated sparingly. Reserved globally:
  - `-v` → `--verbose` (counted, see § 4.6)
  - `-q` → `--quiet`
  - `-h` → `--help` (clap default)
  - `-V` → `--version` (clap default)
- **Boolean flags are bare.** `--json`, `--no-color`, `--dry-run`. No `--json=true` style; clap derive with `default_value_t = false` (as in `src/cli/root.rs`).
- **Negation is `--no-<x>`.** `--color` / `--no-color`, `--cache` / `--no-cache`.
- **Repeated values use plural noun + repetition.** `--feature foo --feature bar`, not `--features foo,bar` (CSV parsing is a footgun for paths and names with commas).
- **Path values** use `value_name = "FILE"` or `"DIR"` in help, as `--workdir <DIR>` and `--config <FILE>` already do in `src/cli/root.rs`.

### 4.2 Subcommand naming

- **Subcommands are verbs in plain form**, never snake_case or kebab-case unless the verb itself contains a hyphen.
  - Good: `scaffold`, `build`, `doctor`, `serve`, `deploy`, `version`, `chain`, `generate`, `app`.
  - Bad: `scaffold_program`, `do-build`, `runValidator`.
- **Compound nouns become space-separated subcommands**, not hyphenated ones.
  - `sunscreen scaffold program <name>` ✅
  - `sunscreen scaffold-program <name>` ❌
- **Nesting depth ≤ 3.** `sunscreen chain serve` is fine; `sunscreen chain validator local start` is not — collapse to `sunscreen chain serve --local`.
- **Stub commands declare themselves as such** in their `about` until implemented (see `Scaffold`, `Chain`, `Generate`, `App` in `src/cli/root.rs::Command` which currently print `"<verb>: TODO"`).

### 4.3 Exit codes

The mapping is fixed by `SunscreenError::exit_code` in `src/error.rs`:

| Code | Meaning | Source variant |
|---|---|---|
| `0` | Success | n/a — returned by successful `dispatch` arms |
| `1` | Generic / uncategorized failure | `SunscreenError::Other(anyhow::Error)` |
| `2` | Toolchain or precondition missing (rustc, solana, anchor, surfpool, pnpm, …) | `SunscreenError::ToolchainMissing(_)` |
| `3` | Configuration invalid (`sunscreen.yml` malformed, schema violation) | `SunscreenError::ConfigInvalid(_)` |
| `4` | User input invalid (bad flag value, missing required arg, invalid name) | `SunscreenError::UserInput(_)` |

**Reserved for future use**: `5` (network/RPC failure), `6` (idempotency conflict — file exists). These are not yet emitted but are reserved so plugins do not claim them.

**Compatibility rule:** an exit code's meaning is part of the public API. Once a command emits exit code `N` for situation `S`, it must continue to do so in every subsequent minor and patch release of the same major version.

### 4.4 Error formatting

The two output modes are implemented in `src/cli/root.rs::execute`:

**Human mode (default):**

```text
error: invalid configuration: missing required field `program.name`
```

The `"error: "` prefix is mandatory and lowercase, matching `cargo` and `rustc`. The message is the `Display` of the `SunscreenError` variant.

**JSON mode (`--json`):**

```json
{"error":"invalid configuration: missing required field `program.name`","kind":"config_invalid"}
```

The `kind` field comes from `SunscreenError::kind_str` and is one of: `config_invalid`, `toolchain_missing`, `user_input`, `other`. **This vocabulary is stable**; new variants append new kinds rather than rename existing ones.

Errors are written to **stderr** in both modes; stdout is reserved for successful structured output so users can pipe `command --json | jq` without contamination.

### 4.5 Output channels and color

- **stdout** = program output (data, JSON, tables). Never log messages.
- **stderr** = diagnostics (logs, progress, errors).
- **Tables** use `comfy-table` with the `UTF8_FULL` preset on TTYs and `ASCII_MARKDOWN` when stdout is not a TTY (so piping into a markdown file just works).
- **Color** uses `owo-colors`' `if_supports_color(Stream::Stdout, …)` so:
  1. `NO_COLOR=1` env → no color (per <https://no-color.org/>).
  2. `--no-color` flag → no color (wins over env if both set).
  3. stdout not a TTY → no color.
  4. Otherwise → color.
- `--json` implies `--no-color` for the JSON payload itself.

### 4.6 Verbosity and quietness

`src/cli/root.rs` already declares:

```rust
#[arg(short, long, global = true, action = clap::ArgAction::Count)]
pub verbose: u8,
```

The mapping (to be wired into the logging layer when introduced):

| Invocation | Log level |
|---|---|
| `-q` / `--quiet` | `ERROR` only |
| (default) | `WARN` and above |
| `-v` | `INFO` and above |
| `-vv` | `DEBUG` and above |
| `-vvv` | `TRACE` and above |

`-q` and `-v` are mutually exclusive at the clap layer (`conflicts_with`). Default is intentionally `WARN`, not `INFO`: a successful `sunscreen build` should produce silence on stderr, leaving stdout clean for output capture.

### 4.7 Stdin / stdout piping

For any subcommand under `generate` and any future `scaffold` artifact that emits a single file:

- The literal argument `-` means **stdin** when used as an input path and **stdout** when used as an output path.
- Example targets: `sunscreen generate clients --from - --out -`, `sunscreen scaffold instruction --from-spec - …`.
- When stdout is the target, all diagnostics shift to stderr unconditionally — no exceptions.

### 4.8 Configuration precedence

Resolved per-key, highest wins:

1. **Explicit CLI flag** (`--workdir /tmp/foo`).
2. **Environment variable** with prefix `SUNSCREEN_` and screaming-snake-case key (`SUNSCREEN_WORKDIR=/tmp/foo`).
3. **`sunscreen.yml`** (or whatever `--config <FILE>` points at; see `Cli::config` in `src/cli/root.rs`).
4. **Compiled-in defaults.**

`sunscreen doctor` (see `src/cli/doctor.rs`) is the canonical debugger: it must, eventually, print the resolved value and its source for any contested key. A `--explain <key>` flag on `doctor` is reserved for this.

### 4.9 Global flags inventory

The following flags are `global = true` on the root `Cli` struct (`src/cli/root.rs`):

| Flag | Type | Status |
|---|---|---|
| `-v` / `--verbose` | count | implemented |
| `--workdir <DIR>` | `Option<PathBuf>` | implemented |
| `--config <FILE>` | `Option<PathBuf>` | implemented |
| `--json` | bool | implemented |
| `-q` / `--quiet` | bool | reserved (this ADR) |
| `--no-color` | bool | reserved (this ADR) |
| `--dry-run` | bool | reserved (this ADR; semantics per subcommand) |

---

## 5. Consequences

### 5.1 Positive

- **Predictability.** A user who has run `sunscreen doctor --json` once can guess that `sunscreen chain serve --json` emits structured events.
- **Scriptability.** Stable exit codes + JSON error schema let CI distinguish "user typo" (exit 4) from "Solana not installed" (exit 2) from "RPC timeout" (future exit 5) without grepping `stderr`.
- **Pipe safety.** Auto-detection of TTY for color and table style means `sunscreen scaffold list | less` and `sunscreen scaffold list > FILE.md` both produce sensible output without flags.
- **Plugin-friendly.** When ADR-0001 § 7.5 plugins arrive, they inherit a documented convention rather than each plugin author inventing their own.
- **AI-agent-friendly.** The MCP-style usage patterns (agents calling `sunscreen … --json`) work today without command-by-command negotiation.

### 5.2 Negative

- **Convention enforcement is a review burden.** There is no compile-time check that a contributor used kebab-case or honored `--no-color`. Mitigated by adding a `tests/conventions.rs` integration test that walks the `clap` command tree and asserts naming rules, and by linting via `cargo clippy` for `print!`/`println!` calls in modules that should use the logger.
- **Two error layers (`thiserror` + `anyhow`).** Slight cognitive cost; mitigated because the boundary is exactly `src/error.rs` and internal modules need only `anyhow::Result<T>`.
- **Reserved exit codes restrict future use.** Codes 5 and 6 are now off the table for ad-hoc reuse; this is intentional.
- **`--json` doubles the testing surface.** Every subcommand needs both a human-mode golden test and a JSON-mode snapshot. Mitigated by `insta` (already in `dev-dependencies`) for snapshots.

---

## 6. Open Questions

- **OQ1** — Should `-q` fully suppress stderr or only suppress non-error logs? Current proposal: suppress everything below `ERROR`. Revisit after first CI integration where users may want `--silent` to also drop `ERROR`.
- **OQ2** — Should `--json` imply machine-friendly exit codes for partial successes (e.g. `doctor` reporting 3/4 checks passing)? Current proposal: `doctor` exits `0` only if all checks pass and `2` if any required check fails, regardless of `--json`. Needs validation against real CI use.
- **OQ3** — Color in CI: GitHub Actions sets `CI=true` but its log viewer renders ANSI. Do we treat `CI=true` as "color OK" (current `owo-colors` behavior) or as "no color" (cargo's behavior pre-1.70)? Current proposal: trust `owo-colors`' default + `NO_COLOR`, document the override.
- **OQ4** — Should `--config` accept a directory (auto-resolving `sunscreen.yml` inside) or only a file? Currently file-only per `value_name = "FILE"`. Re-evaluate when workspace concept lands.
- **OQ5** — Shell completion installation: ship a `sunscreen completions <shell>` subcommand or rely on `cargo-dist` to package them? Defer until first stable release.
- **OQ6** — Plugin subcommands (ADR-0001 § 7.5) inherit these conventions, but how do we enforce them when the plugin is in TypeScript? Likely a `sunscreen-plugin verify` lint step at install time. Defer to the plugin ADR.

---

## 7. References

- ADR-0001 § 6 (Command Surface) and § 7.5 (Plugin protocol).
- `src/cli/root.rs` — `Cli` struct, `execute`, `dispatch`.
- `src/error.rs` — `SunscreenError`, `exit_code`, `kind_str`.
- `src/cli/doctor.rs`, `src/cli/version.rs` — existing implementations following these conventions.
- [clap derive book](https://docs.rs/clap/latest/clap/_derive/index.html).
- [NO_COLOR specification](https://no-color.org/).
- [Command Line Interface Guidelines](https://clig.dev/) — general inspiration for flag and exit-code rules.
- `cargo` source as the de-facto Rust CLI reference.
