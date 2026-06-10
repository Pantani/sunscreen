//! `sunscreen chain` subcommand group.
//!
//! Currently implements `new` (workspace bootstrap). Other subcommands
//! (`serve`, `build`, `deploy`) are stubs.

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use clap::{Args, Subcommand, ValueEnum};

use crate::config::schema::{
    Config, Framework as CfgFramework, Frontend as CfgFrontend, RuntimeEngine,
};
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::runtime::pipeline::{
    BuildKind, BuildPipeline, PipelineError, PipelineOptions, PipelineStep,
};
use crate::runtime::serve::{HeadlessServeLoop, NotifyWatchSource, ServeLoopInput};
use crate::runtime::subprocess::{ProcessSpawner, SubprocessRunner};
use crate::runtime::supervisor::{RuntimeStartReport, RuntimeSupervisor, RuntimeSupervisorError};
use crate::runtime::surfpool::SurfpoolRuntime;
use crate::runtime::testvalidator::TestValidatorRuntime;
use crate::runtime::validator::RuntimePorts;
use crate::templates::{render_dispatch_segment, render_workspace, ArgSpec, InstructionDispatch};
use crate::toolchain::preflight::{self, PreflightError};
use crate::tui::serve_model::ServeModel;

const ANCHOR_VERSION: &str = "0.30.1";
const PINOCCHIO_VERSION: &str = "0.11.1";
const SOLANA_VERSION: &str = "1.18.18";
const RUST_EDITION: &str = "2021";
const PINOCCHIO_MIN_RUST_VERSION: &str = "1.89.0";

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
    /// Anchor 0.30+ (default).
    Anchor,
    /// Pinocchio no_std Solana program scaffold.
    Pinocchio,
}

impl Framework {
    fn to_config(self) -> CfgFramework {
        match self {
            Self::Anchor => CfgFramework::Anchor,
            Self::Pinocchio => CfgFramework::Pinocchio,
        }
    }

    fn workspace_template(self) -> &'static str {
        match self {
            Self::Anchor => "anchor-multiple",
            Self::Pinocchio => "pinocchio-minimal",
        }
    }

    fn next_build_command(self) -> &'static str {
        match self {
            Self::Anchor => "anchor build",
            Self::Pinocchio => "cargo build-sbf",
        }
    }
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

/// Local validator runtime selector for `chain serve`.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum RuntimeChoice {
    /// Surfpool local runtime.
    Surfpool,
    /// Agave `solana-test-validator` fallback runtime.
    TestValidator,
}

impl From<RuntimeEngine> for RuntimeChoice {
    fn from(engine: RuntimeEngine) -> Self {
        match engine {
            RuntimeEngine::Surfpool => Self::Surfpool,
            RuntimeEngine::TestValidator => Self::TestValidator,
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
    /// Skip frontend reload notifications.
    #[arg(long, default_value_t = false)]
    pub no_frontend: bool,
    /// Local runtime to launch. Defaults to `sunscreen.yml`; Surfpool falls
    /// back to `solana-test-validator` when not explicitly requested.
    #[arg(long, value_enum)]
    pub runtime: Option<RuntimeChoice>,
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
    if args.debounce_ms == 0 {
        return Err(SunscreenError::UserInput(
            "--debounce-ms must be greater than zero".into(),
        ));
    }

    let ws = workspace::find_root(None)?;
    let build_kind = build_kind_for_config(ws.config.project.framework);
    let run_codama = build_kind == BuildKind::Anchor && !args.no_codama;
    let debounce = Duration::from_millis(args.debounce_ms);
    let runtime_choice = args
        .runtime
        .unwrap_or_else(|| ws.config.runtime.engine.into());
    let ports = runtime_ports(ws.config.runtime.port)?;
    let runner = SubprocessRunner;
    let mut runtime = start_runtime_with_fallback(
        &ws.root,
        runtime_choice,
        args.runtime.is_some(),
        ports,
        &runner,
    )?;
    let runtime_report = runtime.report().clone();
    let shutdown = install_ctrlc_flag()?;

    if !structured {
        let model = ServeModel::new(runtime_report.runtime, !args.no_frontend);
        println!("{}", model.render_text());
    }

    if structured {
        emit_serve_event(serde_json::json!({
            "event": "chain_serve_started",
            "workspace": ws.root.display().to_string(),
            "runtime": runtime_report.runtime,
            "rpc_endpoint": runtime_report.rpc_endpoint,
            "ws_endpoint": runtime_report.ws_endpoint,
            "framework": framework_str(ws.config.project.framework),
            "codama": run_codama,
            "frontend": !args.no_frontend,
            "debounce_ms": args.debounce_ms,
        }));

        emit_serve_event(serde_json::json!({
            "event": "runtime_started",
            "runtime": runtime_report.runtime,
            "pid": runtime_report.pid,
            "rpc_endpoint": runtime_report.rpc_endpoint,
            "ws_endpoint": runtime_report.ws_endpoint,
        }));
    }

    let source = match NotifyWatchSource::new(&ws.root) {
        Ok(source) => source,
        Err(err) => {
            let _ = runtime.stop();
            return Err(SunscreenError::Other(anyhow::anyhow!(
                "watch workspace: {err}"
            )));
        }
    };
    let mut loop_ = HeadlessServeLoop::new(
        &ws.root,
        debounce,
        PipelineOptions {
            build_kind,
            run_codama,
            notify_frontend: !args.no_frontend,
            frontend_path: ws.config.workspace.frontend_path.clone().map(PathBuf::from),
        },
    );

    while !shutdown.load(Ordering::SeqCst) {
        let input = match source.recv_timeout(debounce) {
            Ok(input) => input,
            Err(err) => {
                let _ = runtime.stop();
                return Err(SunscreenError::Other(anyhow::anyhow!(
                    "watch workspace: {err}"
                )));
            }
        };
        let input = match input {
            Some(event) => ServeLoopInput::NotifyEvent(event, Instant::now()),
            None => ServeLoopInput::Tick(Instant::now()),
        };
        let events = match loop_.handle_input(input, &runner) {
            Ok(events) => events,
            Err(err) => {
                let _ = runtime.stop();
                return Err(map_pipeline_err(err, "sunscreen chain serve"));
            }
        };
        for event in events {
            if structured {
                emit_serve_event(event);
            }
        }
    }

    runtime.stop().map_err(map_runtime_stop_err)?;
    if structured {
        emit_serve_event(serde_json::json!({
            "event": "chain_serve_stopped",
            "runtime": runtime_report.runtime,
        }));
    }
    Ok(0)
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
    let build_kind = build_kind_for_config(ws.config.project.framework);
    let run_codama = build_kind == BuildKind::Anchor && !args.no_codama;

    emit_build_start(
        structured,
        &ws.root,
        framework_str(ws.config.project.framework),
        programs,
        run_codama,
    );

    let report = BuildPipeline::new(&ws.root)
        .run(
            &SubprocessRunner,
            PipelineOptions {
                build_kind,
                run_codama,
                notify_frontend: true,
                frontend_path: ws.config.workspace.frontend_path.clone().map(PathBuf::from),
            },
        )
        .map_err(|err| map_pipeline_err(err, "sunscreen chain build"))?;
    let exit_code = report.exit_code;
    let success = report.success();
    emit_build_report(structured, &report);

    Ok(if success { 0 } else { exit_code })
}

fn emit_build_start(
    structured: bool,
    workspace_root: &Path,
    framework: &'static str,
    programs: Vec<String>,
    run_codama: bool,
) {
    if structured {
        emit_build_event(serde_json::json!({
            "event": "chain_build_started",
            "workspace": workspace_root.display().to_string(),
            "framework": framework,
            "programs": programs,
            "codama": run_codama,
        }));
    } else {
        println!(
            "chain build: running build pipeline in {}",
            workspace_root.display()
        );
    }
}

fn emit_build_report(structured: bool, report: &crate::runtime::pipeline::PipelineReport) {
    if structured {
        emit_structured_build_report(report);
    } else {
        emit_text_build_report(report);
    }
}

fn emit_structured_build_report(report: &crate::runtime::pipeline::PipelineReport) {
    for event in &report.events {
        emit_build_event(event.to_json());
    }
    emit_build_event(serde_json::json!({
        "event": "chain_build_finished",
        "status": if report.success() { "ok" } else { "failed" },
        "exit_code": report.exit_code,
    }));
}

fn emit_text_build_report(report: &crate::runtime::pipeline::PipelineReport) {
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
    if report.success() {
        println!("chain build: ok");
    } else {
        eprintln!(
            "chain build: pipeline failed with exit code {}",
            report.exit_code
        );
    }
}

fn emit_build_event(payload: serde_json::Value) {
    println!("{payload}");
}

fn emit_serve_event(payload: serde_json::Value) {
    println!("{payload}");
}

fn runtime_ports(rpc_port: u16) -> Result<RuntimePorts, SunscreenError> {
    let ws_port = rpc_port.checked_add(1).ok_or_else(|| {
        SunscreenError::UserInput(
            "runtime.port must be less than 65535 so a websocket port can be allocated".into(),
        )
    })?;
    Ok(RuntimePorts::new(rpc_port, ws_port))
}

enum ManagedRuntime {
    Surfpool {
        supervisor: RuntimeSupervisor<SurfpoolRuntime>,
        report: RuntimeStartReport,
    },
    TestValidator {
        supervisor: RuntimeSupervisor<TestValidatorRuntime>,
        report: RuntimeStartReport,
    },
}

impl ManagedRuntime {
    fn report(&self) -> &RuntimeStartReport {
        match self {
            Self::Surfpool { report, .. } | Self::TestValidator { report, .. } => report,
        }
    }

    fn stop(&mut self) -> Result<(), RuntimeSupervisorError> {
        match self {
            Self::Surfpool { supervisor, .. } => supervisor.stop(),
            Self::TestValidator { supervisor, .. } => supervisor.stop(),
        }
    }
}

fn start_runtime_with_fallback<S: ProcessSpawner>(
    workspace_root: &Path,
    choice: RuntimeChoice,
    explicit: bool,
    ports: RuntimePorts,
    spawner: &S,
) -> Result<ManagedRuntime, SunscreenError> {
    match choice {
        RuntimeChoice::Surfpool => {
            let mut supervisor =
                RuntimeSupervisor::new(SurfpoolRuntime::new(ports), workspace_root);
            match supervisor.start(spawner) {
                Ok(report) => Ok(ManagedRuntime::Surfpool { supervisor, report }),
                Err(RuntimeSupervisorError::Start(err)) if err.is_not_found() && !explicit => {
                    eprintln!(
                        "warning: surfpool not found on PATH; falling back to solana-test-validator"
                    );
                    start_test_validator_runtime(workspace_root, ports, spawner)
                }
                Err(err) => Err(map_runtime_start_err(err, "surfpool")),
            }
        }
        RuntimeChoice::TestValidator => {
            start_test_validator_runtime(workspace_root, ports, spawner)
        }
    }
}

fn start_test_validator_runtime<S: ProcessSpawner>(
    workspace_root: &Path,
    ports: RuntimePorts,
    spawner: &S,
) -> Result<ManagedRuntime, SunscreenError> {
    let mut supervisor = RuntimeSupervisor::new(TestValidatorRuntime::new(ports), workspace_root);
    let report = supervisor
        .start(spawner)
        .map_err(|err| map_runtime_start_err(err, "solana-test-validator"))?;
    Ok(ManagedRuntime::TestValidator { supervisor, report })
}

fn map_runtime_start_err(err: RuntimeSupervisorError, tool: &str) -> SunscreenError {
    match err {
        RuntimeSupervisorError::Start(source) if source.is_not_found() => {
            SunscreenError::ToolchainMissing(format!(
                "{tool} not found on PATH; install {tool} before running `sunscreen chain serve`"
            ))
        }
        other => SunscreenError::Other(anyhow::anyhow!("start local runtime: {other}")),
    }
}

fn map_runtime_stop_err(err: RuntimeSupervisorError) -> SunscreenError {
    SunscreenError::Other(anyhow::anyhow!("stop local runtime: {err}"))
}

fn install_ctrlc_flag() -> Result<Arc<AtomicBool>, SunscreenError> {
    let flag = Arc::new(AtomicBool::new(false));
    let handler_flag = Arc::clone(&flag);
    ctrlc::set_handler(move || {
        handler_flag.store(true, Ordering::SeqCst);
    })
    .map_err(|err| SunscreenError::Other(anyhow::anyhow!("install Ctrl-C handler: {err}")))?;
    Ok(flag)
}

fn map_pipeline_err(err: PipelineError, command: &str) -> SunscreenError {
    if err.source.is_not_found() {
        let (tool, install_hint) = match err.step {
            PipelineStep::AnchorBuild => ("anchor", "install Anchor"),
            PipelineStep::PinocchioBuild => (
                "cargo",
                "install the Solana CLI toolchain with cargo-build-sbf",
            ),
            PipelineStep::CodamaRun => ("pnpm", "install pnpm before running Codama"),
            PipelineStep::FrontendNotify => {
                return SunscreenError::Other(anyhow::anyhow!(
                    "notify frontend reload sentinel: {}",
                    err.source
                ));
            }
        };
        SunscreenError::ToolchainMissing(format!(
            "{tool} not found on PATH; {install_hint} before running `{command}`"
        ))
    } else {
        SunscreenError::Other(anyhow::anyhow!("run build pipeline: {err}"))
    }
}

/// Result of materializing a workspace through the shared `chain new` path.
#[derive(Debug, Clone)]
pub(crate) struct NewWorkspaceReport {
    pub project: String,
    pub path: PathBuf,
    pub dry_run: bool,
    pub files: Vec<String>,
    pub written: usize,
}

pub(crate) fn create_workspace(args: &NewArgs) -> Result<NewWorkspaceReport, SunscreenError> {
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
        args.framework.to_config(),
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
    // Always resolve to an absolute path so downstream code (Transaction,
    // WorkspaceRoot, ProgramView) never mixes relative roots with relative
    // sub-paths, which would produce double-prefix paths like
    // "my-app/my-app/programs/..." and fail with ENOENT.
    let dest = if dest.is_absolute() {
        dest
    } else {
        std::env::current_dir()
            .map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?
            .join(&dest)
    };

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
            return Err(SunscreenError::PathConflict(format!(
                "destination already exists and is not empty: {}",
                dest.display()
            )));
        }
        dest.clone()
    };

    let mut tx = Transaction::new(&staging_root).map_err(map_tx_err)?;

    // Render the selected program workspace into the staging dir.
    let template = args.framework.workspace_template();
    render_workspace(template, &ctx, tx.staging())
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render {template}: {e}")))?;

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
        // tx Drop cleans the throwaway staging dir.
        return Ok(NewWorkspaceReport {
            project: args.name.clone(),
            path: dest,
            dry_run: true,
            written: 0,
            files: plan,
        });
    }

    let written = tx.commit().map_err(map_tx_err)?;

    Ok(NewWorkspaceReport {
        project: args.name.clone(),
        path: dest,
        dry_run: false,
        written: written.len(),
        files: plan,
    })
}

fn run_new(args: &NewArgs, json: bool) -> Result<i32, SunscreenError> {
    let report = create_workspace(args)?;

    if json {
        let payload = serde_json::json!({
            "ok": true,
            "project": report.project,
            "path": report.path.display().to_string(),
            "files": report.files,
            "written": report.written,
            "dry_run": report.dry_run,
        });
        println!("{payload}");
    } else if report.dry_run {
        emit_dry_run(&report.path, &report.files);
    } else {
        println!(
            "created workspace `{}` at {} ({} files)",
            report.project,
            report.path.display(),
            report.written
        );
        println!("\nnext steps:");
        println!("  cd {}", report.path.display());
        println!("  {}", args.framework.next_build_command());
        if matches!(args.framework, Framework::Anchor) {
            println!("  anchor test");
        }
    }
    Ok(0)
}

fn emit_dry_run(dest: &Path, plan: &[String]) {
    println!(
        "dry-run: would create {} files under {}",
        plan.len(),
        dest.display()
    );
    for path in plan {
        println!("  {path}");
    }
}

fn build_context(name: &str, frontend: Frontend) -> serde_json::Value {
    use heck::ToSnakeCase;
    let frontend_str = match frontend {
        Frontend::Next => "next",
        Frontend::Vite => "vite",
        Frontend::None => "none",
    };
    let rust_edition = RUST_EDITION;
    serde_json::json!({
        "project_name": name,
        "program_name": name.to_snake_case(),
        "anchor_version": ANCHOR_VERSION,
        "pinocchio_version": PINOCCHIO_VERSION,
        "solana_version": SOLANA_VERSION,
        "rust_edition": rust_edition,
        "pinocchio_min_rust_version": PINOCCHIO_MIN_RUST_VERSION,
        "frontend": frontend_str,
        "cluster": "localnet",
    })
}

fn build_kind_for_config(framework: CfgFramework) -> BuildKind {
    match framework {
        CfgFramework::Anchor => BuildKind::Anchor,
        CfgFramework::Pinocchio => BuildKind::Pinocchio,
        CfgFramework::Shank => BuildKind::Anchor,
    }
}

fn framework_str(framework: CfgFramework) -> &'static str {
    match framework {
        CfgFramework::Anchor => "anchor",
        CfgFramework::Pinocchio => "pinocchio",
        CfgFramework::Shank => "shank",
    }
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
            SunscreenError::PathConflict(format!("destination already exists: {}", p.display()))
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
    use crate::workspace;

    let ws = workspace::find_root(None)?;
    let summary = collect_marker_checks(&ws.programs, &ws.root, args.fix_markers)?;
    let unresolved = summary
        .drift_count
        .saturating_sub(summary.fixed_files.len());

    emit_doctor_report(json, args.fix_markers, &summary, unresolved);

    if unresolved > 0 {
        // Treat unresolved drift as exit 6 (drift) when reporting only; when
        // fix-markers ran but couldn't fix something (non-appendable site),
        // still surface it as drift.
        Ok(6)
    } else {
        Ok(0)
    }
}

struct DoctorSummary {
    findings: Vec<serde_json::Value>,
    fixed_files: Vec<String>,
    drift_count: usize,
}

fn collect_marker_checks(
    programs: &[crate::workspace::ProgramView],
    ws_root: &Path,
    fix_markers: bool,
) -> Result<DoctorSummary, SunscreenError> {
    let mut summary = DoctorSummary {
        findings: Vec::new(),
        fixed_files: Vec::new(),
        drift_count: 0,
    };

    for program in programs {
        collect_program_marker_checks(&mut summary, program, ws_root, fix_markers)?;
    }

    Ok(summary)
}

fn collect_program_marker_checks(
    summary: &mut DoctorSummary,
    program: &crate::workspace::ProgramView,
    ws_root: &Path,
    fix_markers: bool,
) -> Result<(), SunscreenError> {
    let sites = expected_sites(program, ws_root);
    for site in &sites {
        let Some(check) = check_marker_site(fix_markers, program, site)? else {
            continue;
        };
        if check.drifted {
            summary.drift_count += 1;
        }
        if let Some(file) = check.fixed_file {
            summary.fixed_files.push(file);
        }
        summary.findings.push(check.finding);
    }
    Ok(())
}

fn emit_doctor_report(json: bool, fix_markers: bool, summary: &DoctorSummary, unresolved: usize) {
    if json {
        let payload = serde_json::json!({
            "ok": unresolved == 0,
            "fix_markers": fix_markers,
            "findings": summary.findings,
            "fixed": summary.fixed_files,
            "drift_count": summary.drift_count,
            "unresolved": unresolved,
        });
        println!("{payload}");
    } else if summary.drift_count == 0 {
        println!(
            "chain doctor: all markers present ({} sites checked)",
            summary.findings.len()
        );
    } else {
        for f in &summary.findings {
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
        if fix_markers {
            println!(
                "chain doctor: drift={}, fixed={}, unresolved={unresolved}",
                summary.drift_count,
                summary.fixed_files.len(),
            );
        } else {
            println!(
                "chain doctor: drift={} (re-run with --fix-markers to repair)",
                summary.drift_count
            );
        }
    }
}

struct MarkerCheck {
    finding: serde_json::Value,
    fixed_file: Option<String>,
    drifted: bool,
}

fn check_marker_site(
    fix_markers: bool,
    program: &crate::workspace::ProgramView,
    site: &MarkerSite,
) -> Result<Option<MarkerCheck>, SunscreenError> {
    use crate::rustpatch::{scan, MarkerKind};

    if site.optional && !site.abs_path.exists() {
        return Ok(None);
    }
    let rel_str = site.rel_path.to_string_lossy().replace('\\', "/");
    if !site.abs_path.exists() {
        return Ok(Some(marker_check(
            program,
            site,
            &rel_str,
            "file_missing",
            true,
        )));
    }

    let contents = std::fs::read_to_string(&site.abs_path).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read {}: {e}", site.abs_path.display()))
    })?;
    let markers = match scan(&contents) {
        Ok(markers) => markers,
        Err(err) => {
            return Ok(Some(MarkerCheck {
                finding: serde_json::json!({
                    "program": program.name,
                    "file": rel_str,
                    "segment": site.segment,
                    "status": "scan_error",
                    "error": err.to_string(),
                }),
                fixed_file: None,
                drifted: true,
            }));
        }
    };
    if markers
        .iter()
        .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == site.segment)
    {
        return Ok(Some(marker_check(program, site, &rel_str, "ok", false)));
    }
    repair_missing_marker(fix_markers, program, site, &rel_str, &contents)
}

fn marker_check(
    program: &crate::workspace::ProgramView,
    site: &MarkerSite,
    rel_str: &str,
    status: &str,
    drifted: bool,
) -> MarkerCheck {
    MarkerCheck {
        finding: serde_json::json!({
            "program": program.name,
            "file": rel_str,
            "segment": site.segment,
            "status": status,
        }),
        fixed_file: None,
        drifted,
    }
}

fn repair_missing_marker(
    fix_markers: bool,
    program: &crate::workspace::ProgramView,
    site: &MarkerSite,
    rel_str: &str,
    contents: &str,
) -> Result<Option<MarkerCheck>, SunscreenError> {
    if !fix_markers {
        return Ok(Some(marker_check(
            program,
            site,
            rel_str,
            "missing_marker",
            true,
        )));
    }
    let patched = if site.appendable {
        Some(append_marker_block(contents, site.segment))
    } else {
        repair_non_appendable_site(contents, site, program)?
    };
    let Some(patched) = patched else {
        return Ok(Some(marker_check(
            program,
            site,
            rel_str,
            "missing_marker",
            true,
        )));
    };
    std::fs::write(&site.abs_path, &patched).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("write {}: {e}", site.abs_path.display()))
    })?;
    Ok(Some(MarkerCheck {
        finding: serde_json::json!({
            "program": program.name,
            "file": rel_str,
            "segment": site.segment,
            "status": "fixed",
        }),
        fixed_file: Some(rel_str.to_string()),
        drifted: true,
    }))
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
    let (open_line, close_line) = match find_program_module_bounds(existing) {
        Some(bounds) => bounds,
        None => return Ok(None),
    };
    let dispatches = collect_instruction_dispatches(program)?;
    if program_module_contains_dispatch_wrappers(existing, open_line, close_line) {
        return Ok(None);
    }
    let body = render_dispatch_segment(&program.name, &dispatches);
    Ok(Some(insert_dispatch_marker_block(
        existing, close_line, &body,
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
        let Some(args) = parse_handler_args(&source) else {
            continue;
        };
        out.push(InstructionDispatch {
            name: stem.to_string(),
            args,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn parse_handler_args(source: &str) -> Option<Vec<ArgSpec>> {
    let start = source.find("pub fn handler(")?;
    let tail = &source[start + "pub fn handler(".len()..];
    let params = take_param_list(tail)?;
    Some(
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
            .collect(),
    )
}

fn take_param_list(tail: &str) -> Option<&str> {
    let mut angle_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for (idx, ch) in tail.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            '(' => paren_depth += 1,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' if brace_depth > 0 => brace_depth -= 1,
            ')' if angle_depth == 0
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                return Some(&tail[..idx]);
            }
            ')' if paren_depth > 0 => paren_depth -= 1,
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
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for (idx, ch) in params.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            '(' => paren_depth += 1,
            ')' if paren_depth > 0 => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' if brace_depth > 0 => brace_depth -= 1,
            ',' if angle_depth == 0
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                parts.push(&params[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&params[start..]);
    parts
}

fn find_program_module_bounds(existing: &str) -> Option<(usize, usize)> {
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
    let close = find_matching_brace_line(existing, start)?;
    Some((start, close))
}

fn program_module_contains_dispatch_wrappers(
    existing: &str,
    open_line: usize,
    close_line: usize,
) -> bool {
    let lines: Vec<&str> = existing.lines().collect();
    if close_line <= open_line + 1 {
        return false;
    }
    lines[open_line + 1..close_line]
        .iter()
        .any(|line| line.trim_start().starts_with("pub fn "))
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
    if close_line <= open_line {
        return None;
    }
    insert_error_variants_marker_block(existing, open_line, close_line)
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
    let close = find_matching_brace_line(existing, start)?;
    Some((start, close))
}

#[derive(Clone, Copy, Debug)]
enum StringScanMode {
    Normal { escaped: bool },
    Raw { hashes: usize },
}

#[derive(Debug, Default)]
struct RustBraceScanner {
    block_comment_depth: usize,
    string_mode: Option<StringScanMode>,
}

impl RustBraceScanner {
    fn scan_line(&mut self, line: &str, mut on_brace: impl FnMut(char)) {
        let mut idx = 0usize;
        while idx < line.len() {
            let rest = &line[idx..];
            if let Some(advance) = self.consume_block_comment(rest) {
                idx += advance;
                continue;
            }
            if let Some(advance) = self.consume_string(rest) {
                idx += advance;
                continue;
            }
            match self.scan_code(rest, &mut on_brace) {
                LineScanStep::Advance(advance) => idx += advance,
                LineScanStep::Stop => break,
            }
        }
    }

    fn consume_block_comment(&mut self, rest: &str) -> Option<usize> {
        if self.block_comment_depth == 0 {
            return None;
        }
        if rest.starts_with("/*") {
            self.block_comment_depth += 1;
            Some(2)
        } else if rest.starts_with("*/") {
            self.block_comment_depth -= 1;
            Some(2)
        } else {
            Some(next_char_len(rest))
        }
    }

    fn consume_string(&mut self, rest: &str) -> Option<usize> {
        match self.string_mode? {
            StringScanMode::Normal { escaped } => Some(self.consume_normal_string(rest, escaped)),
            StringScanMode::Raw { hashes } => Some(self.consume_raw_string(rest, hashes)),
        }
    }

    fn consume_normal_string(&mut self, rest: &str, escaped: bool) -> usize {
        let ch = rest.chars().next().expect("non-empty rest");
        if escaped {
            self.string_mode = Some(StringScanMode::Normal { escaped: false });
        } else if ch == '\\' {
            self.string_mode = Some(StringScanMode::Normal { escaped: true });
        } else if ch == '"' {
            self.string_mode = None;
        }
        ch.len_utf8()
    }

    fn consume_raw_string(&mut self, rest: &str, hashes: usize) -> usize {
        if let Some(end_len) = raw_string_end_len(rest, hashes) {
            self.string_mode = None;
            end_len
        } else {
            next_char_len(rest)
        }
    }

    fn scan_code(&mut self, rest: &str, on_brace: &mut impl FnMut(char)) -> LineScanStep {
        if rest.starts_with("//") {
            return LineScanStep::Stop;
        }
        if rest.starts_with("/*") {
            self.block_comment_depth += 1;
            return LineScanStep::Advance(2);
        }
        if let Some(hashes) = raw_string_hashes(rest) {
            self.string_mode = Some(StringScanMode::Raw { hashes });
            return LineScanStep::Advance(2 + hashes);
        }

        let ch = rest.chars().next().expect("non-empty rest");
        if ch == '"' {
            self.string_mode = Some(StringScanMode::Normal { escaped: false });
        } else if ch == '{' || ch == '}' {
            on_brace(ch);
        }
        LineScanStep::Advance(ch.len_utf8())
    }
}

enum LineScanStep {
    Advance(usize),
    Stop,
}

fn next_char_len(input: &str) -> usize {
    input.chars().next().map(char::len_utf8).unwrap_or(0)
}

fn raw_string_hashes(input: &str) -> Option<usize> {
    let mut chars = input.chars();
    if chars.next()? != 'r' {
        return None;
    }
    let mut hashes = 0usize;
    for ch in chars {
        match ch {
            '#' => hashes += 1,
            '"' => return Some(hashes),
            _ => return None,
        }
    }
    None
}

fn raw_string_end_len(input: &str, hashes: usize) -> Option<usize> {
    let after_quote = input.strip_prefix('"')?;
    let bytes = after_quote.as_bytes();
    if bytes.len() < hashes {
        return None;
    }
    if bytes[..hashes].iter().all(|byte| *byte == b'#') {
        Some(1 + hashes)
    } else {
        None
    }
}

fn find_matching_brace_line(existing: &str, start_line: usize) -> Option<usize> {
    let lines: Vec<&str> = existing.lines().collect();
    let mut scanner = RustBraceScanner::default();
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start_line) {
        scanner.scan_line(line, |ch| match ch {
            '{' => {
                depth += 1;
                saw_open = true;
            }
            '}' if saw_open => {
                depth -= 1;
            }
            _ => {}
        });
        if saw_open && depth == 0 {
            return Some(idx);
        }
    }
    None
}

fn insert_error_variants_marker_block(
    existing: &str,
    open_line: usize,
    close_line: usize,
) -> Option<String> {
    let nl = crate::cli::scaffold::detect_line_ending(existing);
    let trailing_nl = existing.ends_with('\n');
    let lines: Vec<&str> = existing.lines().collect();
    let enum_indent: String = lines
        .get(open_line)
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).collect())
        .unwrap_or_default();
    let marker_indent = format!("{enum_indent}    ");
    let begin_fragment = "sunscreen:auto-generated:begin segment=error_variants";
    let end_fragment = "sunscreen:auto-generated:end segment=error_variants";
    let begin_line = (open_line + 1..close_line).find(|idx| lines[*idx].contains(begin_fragment));
    let end_line = (open_line + 1..close_line).find(|idx| lines[*idx].contains(end_fragment));

    match (begin_line, end_line) {
        (Some(begin), Some(end)) if begin < end => {
            return Some(rewrite_error_variant_markers(
                &lines,
                begin,
                end,
                &marker_indent,
                nl,
                trailing_nl,
            ));
        }
        (None, None) => {}
        _ => return None,
    }

    if enum_body_has_content(&lines, open_line, close_line) {
        return None;
    }

    Some(insert_empty_error_variant_markers(
        &lines,
        open_line,
        close_line,
        &marker_indent,
        nl,
        trailing_nl,
    ))
}

fn rewrite_error_variant_markers(
    lines: &[&str],
    begin: usize,
    end: usize,
    marker_indent: &str,
    nl: &str,
    trailing_nl: bool,
) -> String {
    let mut out = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        if idx == begin {
            out.push(format!(
                "{marker_indent}// === sunscreen:auto-generated:begin segment=error_variants version=1 generator=doctor ==="
            ));
        } else if idx == end {
            out.push(format!(
                "{marker_indent}// === sunscreen:auto-generated:end segment=error_variants ==="
            ));
        } else {
            out.push((*line).to_string());
        }
    }
    join_preserving_trailing_newline(out, nl, trailing_nl)
}

fn enum_body_has_content(lines: &[&str], open_line: usize, close_line: usize) -> bool {
    lines[open_line + 1..close_line]
        .iter()
        .any(|line| !line.trim().is_empty())
}

fn insert_empty_error_variant_markers(
    lines: &[&str],
    open_line: usize,
    close_line: usize,
    marker_indent: &str,
    nl: &str,
    trailing_nl: bool,
) -> String {
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
    join_preserving_trailing_newline(out, nl, trailing_nl)
}

fn join_preserving_trailing_newline(lines: Vec<String>, nl: &str, trailing_nl: bool) -> String {
    let mut joined = lines.join(nl);
    if trailing_nl {
        joined.push_str(nl);
    }
    joined
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::path::PathBuf;
    use std::rc::Rc;

    use super::*;
    use crate::runtime::subprocess::{CommandSpec, ManagedProcess, ProcessError};

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

    #[test]
    fn parse_handler_args_preserves_tuple_types_and_no_arg_handlers() {
        let args = parse_handler_args(
            "pub fn handler(ctx: Context<Deposit>, pair: (u64, u64), maybe: Option<(u64, u64)>, callback: fn(u64) -> u64) -> Result<()> {}",
        )
        .expect("handler should parse");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].name, "pair");
        assert_eq!(args[0].ty, "(u64, u64)");
        assert_eq!(args[1].name, "maybe");
        assert_eq!(args[1].ty, "Option<(u64, u64)>");
        assert_eq!(args[2].name, "callback");
        assert_eq!(args[2].ty, "fn(u64) -> u64");

        let no_args =
            parse_handler_args("pub fn handler(ctx: Context<Ping>) -> Result<()> { Ok(()) }")
                .expect("no-arg handler should still be a handler");
        assert!(no_args.is_empty());

        assert!(parse_handler_args("pub fn helper() {}").is_none());
    }

    #[test]
    fn find_error_enum_bounds_ignores_braces_inside_messages() {
        let source = r#"use anchor_lang::prelude::*;

#[error_code]
pub enum DemoError {
    #[msg("bad } input")]
    BadInput,
}
"#;
        let (open_line, close_line) =
            find_error_enum_bounds(source).expect("error enum should be found");
        assert_eq!(open_line, 3);
        assert_eq!(close_line, 6);
    }

    #[test]
    fn rewrap_error_variants_refuses_ambiguous_and_single_line_enums() {
        let ambiguous = r#"use anchor_lang::prelude::*;

#[error_code]
pub enum DemoError {
    ExistingVariant,
}
"#;
        assert!(rewrap_error_variants_marker_block(ambiguous).is_none());

        let single_line = r#"use anchor_lang::prelude::*;

#[error_code]
pub enum DemoError {}
"#;
        assert!(rewrap_error_variants_marker_block(single_line).is_none());
    }

    #[test]
    fn runtime_ports_rejects_u16_overflow() {
        let ports = runtime_ports(8899).expect("valid ports");
        assert_eq!(ports.rpc, 8899);
        assert_eq!(ports.ws, 8900);

        let err = runtime_ports(u16::MAX).expect_err("65535 cannot allocate ws port");
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn runtime_start_falls_back_to_test_validator_when_surfpool_missing_and_implicit() {
        let root = PathBuf::from("/tmp/sunscreen-workspace");
        let killed = Rc::new(RefCell::new(false));
        let spawner = RuntimeFakeSpawner {
            calls: RefCell::new(Vec::new()),
            killed: Rc::clone(&killed),
            missing_surfpool: true,
        };

        let mut runtime = start_runtime_with_fallback(
            &root,
            RuntimeChoice::Surfpool,
            false,
            RuntimePorts::new(8899, 8900),
            &spawner,
        )
        .expect("fallback runtime");

        assert_eq!(runtime.report().runtime, "test-validator");
        let calls = spawner.calls.borrow();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].display_argv()[0], "surfpool");
        assert_eq!(calls[1].display_argv()[0], "solana-test-validator");
        drop(calls);
        runtime.stop().expect("stop fallback runtime");
        assert!(*killed.borrow());
    }

    #[test]
    fn runtime_start_errors_when_explicit_surfpool_missing() {
        let root = PathBuf::from("/tmp/sunscreen-workspace");
        let spawner = RuntimeFakeSpawner {
            calls: RefCell::new(Vec::new()),
            killed: Rc::new(RefCell::new(false)),
            missing_surfpool: true,
        };

        let err = match start_runtime_with_fallback(
            &root,
            RuntimeChoice::Surfpool,
            true,
            RuntimePorts::new(8899, 8900),
            &spawner,
        ) {
            Ok(_) => panic!("explicit missing surfpool should fail"),
            Err(err) => err,
        };

        assert_eq!(err.exit_code(), 2);
        assert!(err.to_string().contains("surfpool not found"));
        assert_eq!(spawner.calls.borrow().len(), 1);
    }

    struct RuntimeFakeSpawner {
        calls: RefCell<Vec<CommandSpec>>,
        killed: Rc<RefCell<bool>>,
        missing_surfpool: bool,
    }

    impl ProcessSpawner for RuntimeFakeSpawner {
        fn spawn(&self, spec: CommandSpec) -> Result<Box<dyn ManagedProcess>, ProcessError> {
            let program = spec.display_argv()[0].clone();
            self.calls.borrow_mut().push(spec);
            if self.missing_surfpool && program == "surfpool" {
                return Err(ProcessError::from_io(
                    "surfpool",
                    io::Error::new(io::ErrorKind::NotFound, "missing surfpool"),
                ));
            }
            Ok(Box::new(RuntimeFakeProcess {
                pid: 42,
                killed: Rc::clone(&self.killed),
            }))
        }
    }

    struct RuntimeFakeProcess {
        pid: u32,
        killed: Rc<RefCell<bool>>,
    }

    impl ManagedProcess for RuntimeFakeProcess {
        fn id(&self) -> u32 {
            self.pid
        }

        fn try_wait(&mut self) -> io::Result<Option<i32>> {
            Ok(None)
        }

        fn stop(&mut self) -> io::Result<()> {
            *self.killed.borrow_mut() = true;
            Ok(())
        }
    }
}
