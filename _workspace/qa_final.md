# QA Final Report — sunscreen CLI integration round 1

## Status: GREEN

| Check                         | Result                                  |
|-------------------------------|-----------------------------------------|
| `cargo fmt --check`           | PASS (after autofmt)                    |
| `cargo build`                 | PASS (0 warnings)                       |
| `cargo clippy -D warnings`    | PASS (0 warnings)                       |
| `cargo test`                  | PASS — 16 passed / 0 failed (4 suites)  |
| `sunscreen --help`            | PASS — all 6 stub commands listed       |
| `sunscreen version`           | PASS — prints `sunscreen 0.0.0`         |
| `sunscreen doctor --json`     | PASS — valid JSON array of ToolReport   |

## Cross-module shape checks

- `Config.toolchain.required: BTreeMap<String,String>` (config/schema.rs:55)
  consumed by `detect_all(.., overrides: &BTreeMap<String,String>)`
  (toolchain/detect.rs:95-99) via `cli/doctor.rs:20` — types match.
- `ToolReport` serialized as JSON array (doctor --json) matches inspected output.
- Template engine `render("version.txt.jinja", &ctx)` reachable via
  `sunscreen::templates::render` — golden snapshot now committed.

## Fixes applied by QA

1. `cargo fmt` — applied to 5 files (loader.rs, engine.rs, funcs.rs, detect.rs).
   Cause: agents produced output not normalized through rustfmt.
   Responsible: config-engineer, template-engineer, toolchain-detector.
2. Promoted `tests/golden/snapshots/render_basic__version_basic.snap.new`
   to `.snap`. Output matched expected (`sunscreen 0.1.0\nproject: my_project`).
   Responsible: template-engineer (forgot to accept initial snapshot).

## Remaining defects

None blocking. Minor observations (non-blocking, deferrable):

- `sunscreen version` prints `0.0.0`. Cargo.toml version is likely `0.0.0`
  default; orchestrator may want to bump to `0.1.0` to align with template
  fixture. Owner: cli-architect / orchestrator decision.
- All non-`version`/`doctor` subcommands are stubs by design (per ADR).
