//! Root clap definition and dispatch for sunscreen.
//!
//! Cold-start sensitive: avoid heavy initialization at parse time.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::cli::chain::{self, ChainCmd};
use crate::cli::scaffold::{self, ScaffoldCmd};
use crate::cli::{doctor, version};
use crate::error::SunscreenError;

/// sunscreen — Solana CLI scaffolding & orchestration tool.
#[derive(Debug, Parser)]
#[command(
    name = "sunscreen",
    version,
    about = "Solana CLI scaffolding & orchestration tool",
    long_about = None,
    propagate_version = true,
)]
pub struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Override working directory (defaults to current dir).
    #[arg(long, global = true, value_name = "DIR")]
    pub workdir: Option<PathBuf>,

    /// Path to a sunscreen config file.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Emit structured JSON output where supported.
    #[arg(long, global = true, default_value_t = false)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print sunscreen version.
    Version,
    /// Diagnose local toolchain and environment.
    Doctor {
        /// Only probe a single tool by name (e.g. `anchor`, `solana`).
        #[arg(long, value_name = "NAME")]
        component: Option<String>,
    },
    /// Scaffold Anchor program artifacts (instruction, account, event, ...).
    Scaffold {
        #[command(subcommand)]
        cmd: ScaffoldCmd,
    },
    /// Workspace + chain operations (`new`, `serve`, `build`, `deploy`).
    Chain {
        #[command(subcommand)]
        cmd: ChainCmd,
    },
    /// Code generation utilities (stub).
    Generate,
    /// Application lifecycle commands (stub).
    App,
}

/// Entry point invoked from `main`. Returns a process exit code.
pub fn execute() -> i32 {
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(code) => code,
        Err(err) => {
            if cli.json {
                let payload = serde_json::json!({
                    "error": err.to_string(),
                    "kind": err.kind_str(),
                });
                eprintln!("{payload}");
            } else {
                eprintln!("error: {err}");
            }
            err.exit_code()
        }
    }
}

fn dispatch(cli: &Cli) -> Result<i32, SunscreenError> {
    match &cli.command {
        Command::Version => {
            version::run();
            Ok(0)
        }
        Command::Doctor { component } => {
            doctor::run(cli.json, cli.config.as_deref(), component.as_deref())
                .map_err(SunscreenError::from)
        }
        Command::Scaffold { cmd } => scaffold::run(cmd, cli.json),
        Command::Chain { cmd } => chain::run(cmd, cli.json),
        Command::Generate => {
            eprintln!("generate: TODO");
            Ok(0)
        }
        Command::App => {
            eprintln!("app: TODO");
            Ok(0)
        }
    }
}
