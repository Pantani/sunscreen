//! `sunscreen chain` subcommand group.
//!
//! Currently implements `new` (workspace bootstrap). Other subcommands
//! (`serve`, `build`, `deploy`) are stubs.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::{Args, Subcommand, ValueEnum};

use crate::config::schema::{Config, Framework as CfgFramework, Frontend as CfgFrontend};
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::runtime::pipeline::{BuildPipeline, PipelineError, PipelineOptions, PipelineStep};
use crate::runtime::serve::{HeadlessServeLoop, NotifyWatchSource, ServeLoopInput};
use crate::runtime::subprocess::SubprocessRunner;
use crate::templates::{render_dispatch_segment, render_workspace, ArgSpec, InstructionDispatch};
use crate::toolchain::preflight::{self, PreflightError};

const ANCHOR_VERSION: &str = "0.30.1";
const SOLANA_VERSION: &str = "1.18.18";
const RUST_EDITION: &str = "2021";

/// Subcommands grouped under `sunscreen chain`.
#[derive(Debug, Subcommand)]
pub enum ChainCmd {
    /// Bootstrap a new Solana workspace.
    New(NewArgs),
    /// Run a local validator + program build loop.
    Serve(ServeArgs),
    /// Build all programs in the workspace.
    Build(BuildArgs),
    /// Deploy programs to a cluster (stub).
    Deploy,
    /// Audit workspace marker integrity. Pass `--fix-markers` to insert
    /// missing markers in-place.
    Doctor(DoctorArgs),
}

/// Flags for `sunscreen chain doctor`.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Insert missing markers into source files (otherwise report-only).
    #[arg(long, default_value_t = false)]
    pub fix_markers: bool,
}

/// Framework selector for `chain new`.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Framework {
    /// Anchor 0.30+ (default; only option supported in this phase).
    Anchor,
}

/// Frontend selector for `chain new`.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum Frontend {
    /// Next.js scaffold (under `app/`).
    Next,
    /// Vite scaffold (under `app/`).
    Vite,
    /// No frontend.
    None,
}

impl Frontend {
    fn to_config(self) -> CfgFrontend {
        match self {
            Frontend::Next => CfgFrontend::Next,
            Frontend::Vite => CfgFrontend::Vite,
            Frontend::None => CfgFrontend::None,
        }
    }
}

impl From<Frontend> for preflight::Frontend {
    fn from(f: Frontend) -> Self {
        match f {
            Frontend::Next | Frontend::Vite => preflight::Frontend::Js,
            Frontend::None => preflight::Frontend::None,
        }
    }
}

/// Flags for `sunscreen chain new`.
#[derive(Debug, Args)]
pub struct NewArgs {
    /// Project name (becomes the workspace directory and the program crate).
    pub name: String,
    /// Framework to scaffold.
    #[arg(long, value_enum, default_value_t = Framework::Anchor)]
    pub framework: Framework,
    /// Frontend flavor to scaffold.
    #[arg(long, value_enum, default_value_t = Frontend::None)]
    pub frontend: Frontend,
    /// Output directory. Defaults to `./<name>`.
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
    /// Print the planned file list without writing anything.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Flags for `sunscreen chain build`.
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Emit line-delimited JSON events suitable for CI logs.
    #[arg(long, default_value_t = false)]
    pub headless: bool,
    /// Skip Codama client regeneration after a successful Anchor build.
    #[arg(long, default_value_t = false)]
    pub no_codama: bool,
}

/// Flags for `sunscreen chain serve`.
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Emit line-delimited JSON events and skip the TUI.
    #[arg(long, default_value_t = false)]
    pub headless: bool,
    /// Skip Codama client regeneration after a successful Anchor build.
    #[arg(long, default_value_t = false)]
    pub no_codama: bool,
    /// Debounce filesystem changes before running the build pipeline.
    #[arg(long, default_value_t = 150)]
    pub debounce_ms: u64,
}

/// Dispatch entry point invoked from `cli::root`.
pub fn run(cmd: &ChainCmd, json: bool) -> Result<i32, SunscreenError> {
    match cmd {
        ChainCmd::New(args) => run_new(args, json),
        ChainCmd::Serve(args) => run_serve(args, json),
        ChainCmd::Build(args) => run_build(args, json),
        ChainCmd::Deploy => stub("chain deploy"),
        ChainCmd::Doctor(args) => run_doctor(args, json),
    }
}

fn stub(name: &str) -> Result<i32, SunscreenError> {
    eprintln!("{name}: TODO (Phase 2+)");
    Ok(0)
}

fn run_serve(args: &ServeArgs, json: bool) -> Result<i32, SunscreenError> {
    use crate::workspace;

    let structured = json || args.headless;
    if !structured {
        eprintln!("chain serve: TUI mode TODO (Phase 3); use --headless for the watcher loop");
        return Ok(0);
    }
    if args.debounce_ms == 0 {
        return Err(SunscreenError::UserInput(
            "--debounce-ms must be greater than zero".into(),
        ));
    }

    let ws = workspace::find_root(None)?;
    let debounce = Duration::from_millis(args.debounce_ms);
    emit_serve_event(serde_json::json!({
        "event": "chain_serve_started",
        "workspace": ws.root.display().to_string(),
        "codama": !args.no_codama,
        "debounce_ms": args.debounce_ms,
    }));

    let source = NotifyWatchSource::new(&ws.root)
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("watch workspace: {err}")))?;
    let mut loop_ = HeadlessServeLoop::new(
        &ws.root,
        debounce,
        PipelineOptions {
            run_codama: !args.no_codama,
        },
    );
    let runner = SubprocessRunner;

    loop {
        let input = match source
            .recv_timeout(debounce)
            .map_err(|err| SunscreenError::Other(anyhow::anyhow!("watch workspace: {err}")))?
        {
            Some(event) => ServeLoopInput::NotifyEvent(event, Instant::now()),
            None => ServeLoopInput::Tick(Instant::now()),
        };
        let events = loop_
            .handle_input(input, &runner)
            .map_err(|err| map_pipeline_err(err, "sunscreen chain serve"))?;
        for event in events {
            emit_serve_event(event);
        }
    }
}

fn run_build(args: &BuildArgs, json: bool) -> Result<i32, SunscreenError> {
    use crate::workspace;

    let ws = workspace::find_root(None)?;
    let structured = json || args.headless;
    let programs: Vec<_> = ws
        .programs
        .iter()
        .map(|program| program.name.clone())
        .collect();

    if structured {
        emit_build_event(serde_json::json!({
            "event": "chain_build_started",
            "workspace": ws.root.display().to_string(),
            "programs": programs,
            "codama": !args.no_codama,
        }));
    } else {
        println!(
            "chain build: running build pipeline in {}",
            ws.root.display()
        );
    }

    let report = BuildPipeline::new(&ws.root)
        .run(
            &SubprocessRunner,
            PipelineOptions {
                run_codama: !args.no_codama,
            },
        )
        .map_err(|err| map_pipeline_err(err, "sunscreen chain build"))?;
    let exit_code = report.exit_code;
    let success = report.success();
    let status = if success { "ok" } else { "failed" };

    if structured {
        for event in &report.events {
            emit_build_event(event.to_json());
        }
        emit_build_event(serde_json::json!({
            "event": "chain_build_finished",
            "status": status,
            "exit_code": exit_code,
        }));
    } else {
        for event in &report.events {
            if event.event != "command_finished" {
                continue;
            }
            if let Some(stdout) = &event.stdout {
                print!("{stdout}");
            }
            if let Some(stderr) = &event.stderr {
                eprint!("{stderr}");
            }
        }
        if success {
            println!("chain build: ok");
        } else {
            eprintln!("chain build: pipeline failed with exit code {exit_code}");
        }
    }

    Ok(if success { 0 } else { exit_code })
}

fn emit_build_event(payload: serde_json::Value) {
    println!("{payload}");
}

fn emit_serve_event(payload: serde_json::Value) {
    println!("{payload}");
}

fn map_pipeline_err(err: PipelineError, command: &str) -> SunscreenError {
    if err.source.is_not_found() {
        let (tool, install_hint) = match err.step {
            PipelineStep::AnchorBuild => ("anchor", "install Anchor"),
            PipelineStep::CodamaRun => ("pnpm", "install pnpm before running Codama"),
        };
        SunscreenError::ToolchainMissing(format!(
            "{tool} not found on PATH; {install_hint} before running `{command}`"
        ))
    } else {
        SunscreenError::Other(anyhow::anyhow!("run build pipeline: {err}"))
    }
}

fn run_new(args: &NewArgs, json: bool) -> Result<i32, SunscreenError> {
    validate_name(&args.name)?;

    // The on-disk config requires kebab-case names; user input is
    // normalized here so that callers can pass `MyApp` or `my_app`.
    use heck::ToKebabCase;
    let cfg_name = args.name.to_kebab_case();
    if cfg_name != args.name {
        eprintln!(
            "warning: project name `{}` normalized to `{}` for on-disk config",
            args.name, cfg_name
        );
    }

    // Build the bootstrap config eagerly so any schema validation failure
    // surfaces with a stable exit code (3) before we touch disk.
    let cfg = Config::new_for_workspace(
        &cfg_name,
        match args.framework {
            Framework::Anchor => CfgFramework::Anchor,
        },
        args.frontend.to_config(),
    );
    cfg.validate()
        .map_err(|e| SunscreenError::ConfigInvalid(e.to_string()))?;

    // Gate 6: preflight required toolchain BEFORE any disk work.
    // `SUNSCREEN_SKIP_PREFLIGHT=1` bypasses the gate (used by integration
    // tests and CI environments that don't ship anchor/solana on PATH).
    let skip_preflight = std::env::var_os("SUNSCREEN_SKIP_PREFLIGHT")
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(false);
    if skip_preflight {
        eprintln!("warning: SUNSCREEN_SKIP_PREFLIGHT set; skipping toolchain preflight");
    } else {
        match preflight::preflight_chain_new(&cfg, args.frontend.into()) {
            Ok(report) => {
                for w in &report.warnings {
                    eprintln!("warning: {w}");
                }
            }
            Err(PreflightError::Failed(msg)) => {
                return Err(SunscreenError::ToolchainMissing(msg));
            }
        }
    }

    let dest = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(&args.name));

    let ctx = build_context(&args.name, args.frontend);

    // Stage everything into a temporary location. For dry-run we use a
    // throwaway tempdir so nothing inside `dest` is ever touched.
    let dry_run = args.dry_run;
    let staging_root: PathBuf = if dry_run {
        tempfile::tempdir()
            .map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?
            .keep()
    } else {
        if dest.exists() && dir_not_empty(&dest)? {
            return Err(SunscreenError::UserInput(format!(
                "destination already exists and is not empty: {}",
                dest.display()
            )));
        }
        dest.clone()
    };

    let mut tx = Transaction::new(&staging_root).map_err(map_tx_err)?;

    // Render the Anchor workspace into the staging dir.
    render_workspace("anchor-multiple", &ctx, tx.staging())
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render anchor-multiple: {e}")))?;

    // Render frontend scaffold (if requested) into staging/app/.
    match args.frontend {
        Frontend::Next => {
            let app = tx.staging().join("app");
            std::fs::create_dir_all(&app).map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?;
            render_workspace("frontend-next", &ctx, &app)
                .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render frontend-next: {e}")))?;
        }
        Frontend::Vite => {
            let app = tx.staging().join("app");
            std::fs::create_dir_all(&app).map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?;
            render_workspace("frontend-vite", &ctx, &app)
                .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render frontend-vite: {e}")))?;
        }
        Frontend::None => {
            render_workspace("frontend-none", &ctx, tx.staging())
                .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render frontend-none: {e}")))?;
        }
    }

    // Register every file the renderer wrote so the two-phase commit
    // can plan / commit / rollback them.
    tx.adopt_staged_tree().map_err(map_tx_err)?;

    let plan: Vec<String> = tx.plan().iter().map(|p| p.path.clone()).collect();

    if dry_run {
        emit_dry_run(&dest, &plan, json);
        // tx Drop cleans the throwaway staging dir.
        return Ok(0);
    }

    let written = tx.commit().map_err(map_tx_err)?;

    if json {
        let payload = serde_json::json!({
            "ok": true,
            "project": args.name,
            "path": dest.display().to_string(),
            "files": written.len(),
        });
        println!("{payload}");
    } else {
        println!(
            "created workspace `{}` at {} ({} files)",
            args.name,
            dest.display(),
            written.len()
        );
        println!("\nnext steps:");
        println!("  cd {}", dest.display());
        println!("  anchor build");
        println!("  anchor test");
    }
    Ok(0)
}

fn emit_dry_run(dest: &Path, plan: &[String], json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "path": dest.display().to_string(),
            "files": plan,
        });
        println!("{payload}");
    } else {
        println!(
            "dry-run: would create {} files under {}",
            plan.len(),
            dest.display()
        );
        for path in plan {
            println!("  {path}");
        }
    }
}

fn build_context(name: &str, frontend: Frontend) -> serde_json::Value {
    use heck::ToSnakeCase;
    let frontend_str = match frontend {
        Frontend::Next => "next",
        Frontend::Vite => "vite",
        Frontend::None => "none",
    };
    serde_json::json!({
        "project_name": name,
        "program_name": name.to_snake_case(),
        "anchor_version": ANCHOR_VERSION,
        "solana_version": SOLANA_VERSION,
        "rust_edition": RUST_EDITION,
        "frontend": frontend_str,
        "cluster": "localnet",
    })
}

fn validate_name(name: &str) -> Result<(), SunscreenError> {
    if name.is_empty() {
        return Err(SunscreenError::UserInput("project name is empty".into()));
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err(SunscreenError::UserInput(
            "project name must start with an ASCII letter".into(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(SunscreenError::UserInput(
            "project name may only contain letters, digits, '-' and '_'".into(),
        ));
    }
    Ok(())
}

fn dir_not_empty(p: &Path) -> Result<bool, SunscreenError> {
    let mut rd = std::fs::read_dir(p)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read_dir {}: {e}", p.display())))?;
    Ok(rd.next().is_some())
}

fn map_tx_err(e: TxError) -> SunscreenError {
    match e {
        TxError::PathEscape(p) => SunscreenError::UserInput(format!("invalid template path: {p}")),
        TxError::DestinationExists(p) => {
            SunscreenError::UserInput(format!("destination already exists: {}", p.display()))
        }
        TxError::DuplicateStage(p) => {
            SunscreenError::Other(anyhow::anyhow!("template emitted duplicate path: {p}"))
        }
        TxError::Io(e) => SunscreenError::Other(anyhow::anyhow!(e)),
    }
}

// ---------------------------------------------------------------------------
// chain doctor — audit / repair workspace markers
// ---------------------------------------------------------------------------

/// One expected marker site in a program crate.
#[derive(Debug)]
struct MarkerSite {
    /// Workspace-relative path to the file owning the segment.
    rel_path: PathBuf,
    /// Absolute path on disk.
    abs_path: PathBuf,
    /// Segment name (e.g. `dispatch`, `instructions`).
    segment: &'static str,
    /// Whether the file is allowed to be missing (then nothing to check).
    optional: bool,
    /// Whether — when the file exists but has no markers — `--fix-markers`
    /// is allowed to append the marker block at end of file.
    appendable: bool,
}

fn run_doctor(args: &DoctorArgs, json: bool) -> Result<i32, SunscreenError> {
    use crate::rustpatch::{scan, MarkerKind};
    use crate::workspace;

    let ws = workspace::find_root(None)?;

    let mut findings: Vec<serde_json::Value> = Vec::new();
    let mut fixed_files: Vec<String> = Vec::new();
    let mut drift_count = 0usize;

    for program in &ws.programs {
        let sites = expected_sites(program, &ws.root);
        for site in &sites {
            if site.optional && !site.abs_path.exists() {
                continue;
            }
            let rel_str = site.rel_path.to_string_lossy().replace('\\', "/");
            if !site.abs_path.exists() {
                findings.push(serde_json::json!({
                    "program": program.name,
                    "file": rel_str,
                    "segment": site.segment,
                    "status": "file_missing",
                }));
                drift_count += 1;
                continue;
            }
            let contents = std::fs::read_to_string(&site.abs_path).map_err(|e| {
                SunscreenError::Other(anyhow::anyhow!("read {}: {e}", site.abs_path.display()))
            })?;
            // Treat scan failures (malformed/unbalanced markers) as drift
            // findings instead of aborting the whole run — those are exactly
            // the cases the doctor exists to surface.
            let markers = match scan(&contents) {
                Ok(m) => m,
                Err(e) => {
                    findings.push(serde_json::json!({
                        "program": program.name,
                        "file": rel_str,
                        "segment": site.segment,
                        "status": "scan_error",
                        "error": e.to_string(),
                    }));
                    drift_count += 1;
                    continue;
                }
            };
            let has_segment = markers
                .iter()
                .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == site.segment);
            if has_segment {
                findings.push(serde_json::json!({
                    "program": program.name,
                    "file": rel_str,
                    "segment": site.segment,
                    "status": "ok",
                }));
                continue;
            }
            drift_count += 1;
            if args.fix_markers {
                let patched = if site.appendable {
                    Some(append_marker_block(&contents, site.segment))
                } else {
                    repair_non_appendable_site(&contents, site, program)?
                };
                if let Some(patched) = patched {
                    std::fs::write(&site.abs_path, &patched).map_err(|e| {
                        SunscreenError::Other(anyhow::anyhow!(
                            "write {}: {e}",
                            site.abs_path.display()
                        ))
                    })?;
                    fixed_files.push(rel_str.clone());
                    findings.push(serde_json::json!({
                        "program": program.name,
                        "file": rel_str,
                        "segment": site.segment,
                        "status": "fixed",
                    }));
                } else {
                    findings.push(serde_json::json!({
                        "program": program.name,
                        "file": rel_str,
                        "segment": site.segment,
                        "status": "missing_marker",
                    }));
                }
            } else {
                findings.push(serde_json::json!({
                    "program": program.name,
                    "file": rel_str,
                    "segment": site.segment,
                    "status": "missing_marker",
                }));
            }
        }
    }

    let unresolved = drift_count.saturating_sub(fixed_files.len());

    if json {
        let payload = serde_json::json!({
            "ok": unresolved == 0,
            "fix_markers": args.fix_markers,
            "findings": findings,
            "fixed": fixed_files,
            "drift_count": drift_count,
            "unresolved": unresolved,
        });
        println!("{payload}");
    } else if drift_count == 0 {
        println!(
            "chain doctor: all markers present ({} sites checked)",
            findings.len()
        );
    } else {
        for f in &findings {
            let status = f.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            if status == "ok" {
                continue;
            }
            println!(
                "  [{status}] {}  segment={}",
                f.get("file").and_then(|v| v.as_str()).unwrap_or(""),
                f.get("segment").and_then(|v| v.as_str()).unwrap_or(""),
            );
        }
        if args.fix_markers {
            println!(
                "chain doctor: drift={drift_count}, fixed={}, unresolved={unresolved}",
                fixed_files.len(),
            );
        } else {
            println!("chain doctor: drift={drift_count} (re-run with --fix-markers to repair)");
        }
    }

    if unresolved > 0 {
        // Treat unresolved drift as exit 6 (drift) when reporting only; when
        // fix-markers ran but couldn't fix something (non-appendable site),
        // still surface it as drift.
        Ok(6)
    } else {
        Ok(0)
    }
}

fn expected_sites(program: &crate::workspace::ProgramView, ws_root: &Path) -> Vec<MarkerSite> {
    let rel = |abs: &Path| {
        abs.strip_prefix(ws_root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| abs.to_path_buf())
    };
    let src = &program.src_dir;
    vec![
        MarkerSite {
            rel_path: rel(&program.lib_rs),
            abs_path: program.lib_rs.clone(),
            segment: "dispatch",
            optional: false,
            // dispatch lives inside `#[program] mod {}` — cannot blindly append
            // at EOF. Reporter-only.
            appendable: false,
        },
        MarkerSite {
            rel_path: rel(&program.instructions_mod_rs),
            abs_path: program.instructions_mod_rs.clone(),
            segment: "instructions",
            optional: false,
            appendable: true,
        },
        MarkerSite {
            rel_path: rel(&src.join("state").join("mod.rs")),
            abs_path: src.join("state").join("mod.rs"),
            segment: "accounts",
            optional: true,
            appendable: true,
        },
        MarkerSite {
            rel_path: rel(&src.join("events.rs")),
            abs_path: src.join("events.rs"),
            segment: "events",
            optional: true,
            appendable: true,
        },
        MarkerSite {
            rel_path: rel(&src.join("errors.rs")),
            abs_path: src.join("errors.rs"),
            segment: "error_variants",
            optional: true,
            // error_variants markers live INSIDE `#[error_code] pub enum ... {}`
            // (see build_errors_host in src/cli/scaffold.rs). A blind EOF
            // append would place variants outside the enum on the next
            // `scaffold error` run, producing invalid Rust. Report-only.
            appendable: false,
        },
    ]
}

fn append_marker_block(existing: &str, segment: &str) -> String {
    let nl = crate::cli::scaffold::detect_line_ending(existing);
    let mut out = existing.to_string();
    if !out.ends_with('\n') {
        out.push_str(nl);
    }
    out.push_str(nl);
    out.push_str(&format!(
        "// === sunscreen:auto-generated:begin segment={segment} version=1 generator=doctor ==={nl}"
    ));
    out.push_str(&format!(
        "// === sunscreen:auto-generated:end segment={segment} ==={nl}"
    ));
    out
}

fn repair_non_appendable_site(
    existing: &str,
    site: &MarkerSite,
    program: &crate::workspace::ProgramView,
) -> Result<Option<String>, SunscreenError> {
    match site.segment {
        "dispatch" => rebuild_dispatch_marker_block(existing, program),
        "error_variants" => Ok(rewrap_error_variants_marker_block(existing)),
        _ => Ok(None),
    }
}

fn rebuild_dispatch_marker_block(
    existing: &str,
    program: &crate::workspace::ProgramView,
) -> Result<Option<String>, SunscreenError> {
    let insert_line = match find_program_module_close_line(existing) {
        Some(line) => line,
        None => return Ok(None),
    };
    let dispatches = collect_instruction_dispatches(program)?;
    let body = render_dispatch_segment(&program.name, &dispatches);
    Ok(Some(insert_dispatch_marker_block(
        existing,
        insert_line,
        &body,
    )))
}

fn collect_instruction_dispatches(
    program: &crate::workspace::ProgramView,
) -> Result<Vec<InstructionDispatch>, SunscreenError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&program.instructions_dir).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!(
            "read {}: {e}",
            program.instructions_dir.display()
        ))
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == "mod" {
            continue;
        }
        let source = std::fs::read_to_string(&path)
            .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
        out.push(InstructionDispatch {
            name: stem.to_string(),
            args: parse_handler_args(&source),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn parse_handler_args(source: &str) -> Vec<ArgSpec> {
    let Some(start) = source.find("pub fn handler(") else {
        return Vec::new();
    };
    let tail = &source[start + "pub fn handler(".len()..];
    let Some(params) = take_param_list(tail) else {
        return Vec::new();
    };
    split_params(params)
        .into_iter()
        .filter_map(|param| {
            let param = param.trim();
            if param.is_empty() || param.starts_with("ctx:") || param.starts_with("ctx :") {
                return None;
            }
            let (name, ty) = param.split_once(':')?;
            let name = name.trim();
            let ty = ty.trim();
            if name.is_empty() || ty.is_empty() {
                return None;
            }
            Some(ArgSpec {
                name: name.to_string(),
                ty: ty.to_string(),
            })
        })
        .collect()
}

fn take_param_list(tail: &str) -> Option<&str> {
    let mut angle_depth = 0usize;
    for (idx, ch) in tail.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            ')' if angle_depth == 0 => return Some(&tail[..idx]),
            _ => {}
        }
    }
    None
}

fn split_params(params: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    for (idx, ch) in params.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            '(' => paren_depth += 1,
            ')' if paren_depth > 0 => paren_depth -= 1,
            ',' if angle_depth == 0 && paren_depth == 0 => {
                parts.push(&params[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&params[start..]);
    parts
}

fn find_program_module_close_line(existing: &str) -> Option<usize> {
    let lines: Vec<&str> = existing.lines().collect();
    let mut saw_program_attr = false;
    let mut start = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "#[program]" {
            saw_program_attr = true;
            continue;
        }
        if saw_program_attr && trimmed.starts_with("pub mod ") && trimmed.contains('{') {
            start = Some(idx);
            break;
        }
        if saw_program_attr && !trimmed.is_empty() && !trimmed.starts_with("#[") {
            saw_program_attr = false;
        }
    }

    let start = start?;
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' if saw_open => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(idx);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn insert_dispatch_marker_block(existing: &str, insert_line: usize, body: &str) -> String {
    let nl = crate::cli::scaffold::detect_line_ending(existing);
    let trailing_nl = existing.ends_with('\n');
    let lines: Vec<&str> = existing.lines().collect();
    let close_indent: String = lines
        .get(insert_line)
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).collect())
        .unwrap_or_default();
    let marker_indent = format!("{close_indent}    ");

    let mut out = Vec::with_capacity(lines.len() + body.lines().count() + 4);
    for (idx, line) in lines.iter().enumerate() {
        if idx == insert_line {
            if out
                .last()
                .is_some_and(|prev: &String| !prev.trim().is_empty())
            {
                out.push(String::new());
            }
            out.push(format!(
                "{marker_indent}// === sunscreen:auto-generated:begin segment=dispatch version=1 generator=doctor ==="
            ));
            for body_line in body.strip_suffix('\n').unwrap_or(body).lines() {
                out.push(body_line.to_string());
            }
            out.push(format!(
                "{marker_indent}// === sunscreen:auto-generated:end segment=dispatch ==="
            ));
        }
        out.push((*line).to_string());
    }
    let mut joined = out.join(nl);
    if trailing_nl {
        joined.push_str(nl);
    }
    joined
}

fn rewrap_error_variants_marker_block(existing: &str) -> Option<String> {
    let (open_line, close_line) = find_error_enum_bounds(existing)?;
    Some(insert_error_variants_marker_block(
        existing, open_line, close_line,
    ))
}

fn find_error_enum_bounds(existing: &str) -> Option<(usize, usize)> {
    let lines: Vec<&str> = existing.lines().collect();
    let mut saw_error_attr = false;
    let mut start = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "#[error_code]" {
            saw_error_attr = true;
            continue;
        }
        if saw_error_attr && trimmed.starts_with("pub enum ") && trimmed.contains('{') {
            start = Some(idx);
            break;
        }
        if saw_error_attr && !trimmed.is_empty() && !trimmed.starts_with("#[") {
            saw_error_attr = false;
        }
    }

    let start = start?;
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' if saw_open => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((start, idx));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn insert_error_variants_marker_block(
    existing: &str,
    open_line: usize,
    close_line: usize,
) -> String {
    let nl = crate::cli::scaffold::detect_line_ending(existing);
    let trailing_nl = existing.ends_with('\n');
    let lines: Vec<&str> = existing.lines().collect();
    let enum_indent: String = lines
        .get(open_line)
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).collect())
        .unwrap_or_default();
    let marker_indent = format!("{enum_indent}    ");

    let mut out = Vec::with_capacity(lines.len() + 2);
    for (idx, line) in lines.iter().enumerate() {
        out.push((*line).to_string());
        if idx == open_line {
            out.push(format!(
                "{marker_indent}// === sunscreen:auto-generated:begin segment=error_variants version=1 generator=doctor ==="
            ));
        }
        if idx + 1 == close_line {
            out.push(format!(
                "{marker_indent}// === sunscreen:auto-generated:end segment=error_variants ==="
            ));
        }
    }
    let mut joined = out.join(nl);
    if trailing_nl {
        joined.push_str(nl);
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_accepts_basic() {
        assert!(validate_name("my_app").is_ok());
        assert!(validate_name("MyApp").is_ok());
        assert!(validate_name("foo-bar").is_ok());
    }

    #[test]
    fn validate_name_rejects_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("1abc").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("dot.name").is_err());
    }
}
