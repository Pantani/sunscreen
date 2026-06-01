//! `sunscreen doctor` — probe the local toolchain and report status.
//!
//! Owned by the `toolchain-detector` agent. Defaults to a colored table;
//! `--json` emits a machine-readable array. Exit code is `2` if any
//! required tool is missing, below its minimum, or yields an unparsable
//! version; otherwise `0`.

use std::path::Path;

use comfy_table::{Cell, Color, ContentArrangement, Table};
use owo_colors::OwoColorize;

use crate::toolchain::{detect_all, known, RealRunner, Status, ToolReport};

/// Run doctor diagnostics. `config_path` overrides automatic config discovery.
/// When `component` is `Some`, only that tool is probed; if the name is
/// unknown, an `anyhow` error is returned (mapped by the caller to a
/// non-zero exit).
pub fn run(json: bool, config_path: Option<&Path>, component: Option<&str>) -> anyhow::Result<i32> {
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

    if json {
        // Stable machine-readable schema:
        //   {
        //     "tools": [ ToolReport, ... ],         // full per-tool detail
        //     "available": { "<name>": bool, ... }  // flat skip-helper map
        //   }
        // The flat `available` map is what downstream integration tests
        // consume to decide whether to skip Anchor / codama / surfpool
        // suites. The per-tool reports also each carry an `available`
        // boolean so single-tool consumers don't need the top-level map.
        let available: std::collections::BTreeMap<&str, bool> = reports
            .iter()
            .map(|r| (r.name.as_str(), r.available))
            .collect();
        let envelope = serde_json::json!({
            "tools": &reports,
            "available": available,
        });
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        print_table(&reports);
    }

    let failed = reports.iter().any(|r| {
        r.required
            && matches!(
                r.status,
                Status::MissingRequired | Status::BelowMin | Status::UnknownVersion
            )
    });
    Ok(if failed { 2 } else { 0 })
}

/// Back-compat shim for the existing single-argument call site in
/// `cli::root` until that file is updated to pass `--config`.
pub fn run_compat(json: bool) -> anyhow::Result<i32> {
    run(json, None, None)
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
