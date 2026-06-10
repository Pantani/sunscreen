//! Workspace bootstrap shared by `chain new` and onboarding flows.

use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};

use crate::config::schema::{Config, Framework as CfgFramework, Frontend as CfgFrontend};
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::templates::render_workspace;
use crate::toolchain::preflight::{self, PreflightError};

const ANCHOR_VERSION: &str = "0.30.1";
const PINOCCHIO_VERSION: &str = "0.11.1";
const SOLANA_VERSION: &str = "1.18.18";
const RUST_EDITION: &str = "2021";
const PINOCCHIO_MIN_RUST_VERSION: &str = "1.89.0";

/// Framework selector for workspace bootstrap.
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

    pub(crate) fn next_build_command(self) -> &'static str {
        match self {
            Self::Anchor => "anchor build",
            Self::Pinocchio => "cargo build-sbf",
        }
    }
}

/// Frontend selector for workspace bootstrap.
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

/// Flags for workspace bootstrap.
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

/// Result of materializing a workspace through the shared bootstrap path.
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

pub(crate) fn validate_name(name: &str) -> Result<(), SunscreenError> {
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
