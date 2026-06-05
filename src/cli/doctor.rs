//! `sunscreen doctor` — probe the local toolchain and report status.
//!
//! Owned by the `toolchain-detector` agent. Defaults to a colored table;
//! `--json` emits a machine-readable array. Exit code is `2` if any
//! required tool is missing, below its minimum, or yields an unparsable
//! version; otherwise `0`.

use std::path::Path;

use comfy_table::{Cell, Color, ContentArrangement, Table};
use owo_colors::OwoColorize;
use serde::Serialize;

use crate::runtime::subprocess::SubprocessRunner;
use crate::toolchain::{
    detect_all, finalize_fix_results, fix_reports, known, RealRunner, Status, ToolFixResult,
    ToolFixStatus, ToolReport,
};

/// Run doctor diagnostics. `config_path` overrides automatic config discovery.
/// When `component` is `Some`, only that tool is probed; if the name is
/// unknown, an `anyhow` error is returned (mapped by the caller to a
/// non-zero exit).
pub fn run(
    json: bool,
    config_path: Option<&Path>,
    component: Option<&str>,
    fix: bool,
) -> anyhow::Result<i32> {
    let cfg = crate::config::load(config_path).unwrap_or_default();
    let runner = RealRunner;
    let specs = match component {
        None => known(),
        Some(name) => {
            let filtered: Vec<_> = known().into_iter().filter(|s| s.name == name).collect();
            if filtered.is_empty() {
                anyhow::bail!(
                    "unknown component {:?} (known: {})",
                    name,
                    known()
                        .iter()
                        .map(|s| s.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            filtered
        }
    };
    let reports = detect_all(&runner, &specs, &cfg.toolchain.required);

    if fix {
        let failed_before = any_required_failed(&reports);
        let process_runner = SubprocessRunner;
        let mut fixes = fix_reports(&runner, &process_runner, &reports, component.is_some());
        let reports_after = detect_all(&runner, &specs, &cfg.toolchain.required);
        finalize_fix_results(&mut fixes, &reports_after);
        let failed_after = any_required_failed(&reports_after);

        if json {
            let payload = FixPayload {
                ok_before: !failed_before,
                ok_after: !failed_after,
                reports_before: &reports,
                fixes: &fixes,
                reports_after: &reports_after,
            };
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("{}", "Before".bold());
            print_table(&reports);
            println!("{}", "Fixes".bold());
            print_fix_table(&fixes);
            println!("{}", "After".bold());
            print_table(&reports_after);
        }

        let failed_fix = fixes.iter().any(|fix| {
            (fix.required || component.is_some())
                && matches!(
                    fix.status,
                    ToolFixStatus::Failed
                        | ToolFixStatus::NeedsShellReload
                        | ToolFixStatus::Unsupported
                )
        });
        return Ok(if failed_after || failed_fix { 2 } else { 0 });
    }

    if json {
        // Stable machine-readable schema: a JSON array of `ToolReport`
        // (each entry carries `name`, `version`, `found`, `available`,
        // `status`, ...). Downstream integration tests pick out the
        // tool they care about and read its `available` boolean. This
        // preserves the array shape contracted by the file header so
        // existing `sunscreen doctor --json` consumers don't break.
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        print_table(&reports);
    }

    let failed = any_required_failed(&reports);
    Ok(if failed { 2 } else { 0 })
}

/// Back-compat shim for the existing single-argument call site in
/// `cli::root` until that file is updated to pass `--config`.
pub fn run_compat(json: bool) -> anyhow::Result<i32> {
    run(json, None, None, false)
}

#[derive(Serialize)]
struct FixPayload<'a> {
    ok_before: bool,
    ok_after: bool,
    reports_before: &'a [ToolReport],
    fixes: &'a [ToolFixResult],
    reports_after: &'a [ToolReport],
}

fn any_required_failed(reports: &[ToolReport]) -> bool {
    reports.iter().any(|r| {
        r.required
            && matches!(
                r.status,
                Status::MissingRequired | Status::BelowMin | Status::UnknownVersion
            )
    })
}

fn print_table(reports: &[ToolReport]) {
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Tool", "Found", "Version", "Min", "Status"]);

    for r in reports {
        let found = if r.found { "yes" } else { "no" };
        let version = r
            .version
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".into());
        let min = r
            .min_version
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".into());
        let status_cell = match r.status {
            Status::Ok => Cell::new(format!("{}", "ok".green())).fg(Color::Green),
            Status::MissingRequired => Cell::new(format!("{}", "missing".red())).fg(Color::Red),
            Status::MissingOptional => {
                Cell::new(format!("{}", "missing (optional)".dimmed())).fg(Color::DarkGrey)
            }
            Status::BelowMin => Cell::new(format!("{}", "below-min".red())).fg(Color::Red),
            Status::UnknownVersion => {
                Cell::new(format!("{}", "unknown-version".yellow())).fg(Color::Yellow)
            }
        };
        table.add_row(vec![
            Cell::new(&r.name),
            Cell::new(found),
            Cell::new(version),
            Cell::new(min),
            status_cell,
        ]);
    }

    println!("{table}");
}

fn print_fix_table(fixes: &[ToolFixResult]) {
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Tool", "Status", "Message"]);

    for fix in fixes {
        let status_cell = match fix.status {
            ToolFixStatus::Fixed => Cell::new(format!("{}", "fixed".green())).fg(Color::Green),
            ToolFixStatus::Skipped => {
                Cell::new(format!("{}", "skipped".dimmed())).fg(Color::DarkGrey)
            }
            ToolFixStatus::NeedsShellReload => {
                Cell::new(format!("{}", "reload-shell".yellow())).fg(Color::Yellow)
            }
            ToolFixStatus::Unsupported => {
                Cell::new(format!("{}", "unsupported".yellow())).fg(Color::Yellow)
            }
            ToolFixStatus::Failed => Cell::new(format!("{}", "failed".red())).fg(Color::Red),
            ToolFixStatus::Attempted => Cell::new(format!("{}", "attempted".yellow())),
        };
        table.add_row(vec![
            Cell::new(&fix.name),
            status_cell,
            Cell::new(&fix.message),
        ]);
    }

    println!("{table}");
}
