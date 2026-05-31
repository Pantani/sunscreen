//! `sunscreen chain` subcommand group.
//!
//! Currently implements `new` (workspace bootstrap). Other subcommands
//! (`serve`, `build`, `deploy`) are stubs.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};

use crate::config::schema::{Config, Framework as CfgFramework, Frontend as CfgFrontend};
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::templates::render_workspace;
use crate::toolchain::preflight::{self, PreflightError};

const ANCHOR_VERSION: &str = "0.30.1";
const SOLANA_VERSION: &str = "1.18.18";
const RUST_EDITION: &str = "2021";

/// Subcommands grouped under `sunscreen chain`.
#[derive(Debug, Subcommand)]
pub enum ChainCmd {
    /// Bootstrap a new Solana workspace.
    New(NewArgs),
    /// Run a local validator + program build loop (stub).
    Serve,
    /// Build all programs in the workspace (stub).
    Build,
    /// Deploy programs to a cluster (stub).
    Deploy,
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

/// Dispatch entry point invoked from `cli::root`.
pub fn run(cmd: &ChainCmd, json: bool) -> Result<i32, SunscreenError> {
    match cmd {
        ChainCmd::New(args) => run_new(args, json),
        ChainCmd::Serve => stub("chain serve"),
        ChainCmd::Build => stub("chain build"),
        ChainCmd::Deploy => stub("chain deploy"),
    }
}

fn stub(name: &str) -> Result<i32, SunscreenError> {
    eprintln!("{name}: TODO (Phase 2+)");
    Ok(0)
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
