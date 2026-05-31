//! Root clap definition and dispatch for sunscreen.
//!
//! Cold-start sensitive: avoid heavy initialization at parse time.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
    Doctor,
    /// Scaffold a new Solana project (stub).
    Scaffold,
    /// Manage local validator / chain operations (stub).
    Chain,
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
        Command::Doctor => {
            doctor::run(cli.json, cli.config.as_deref()).map_err(SunscreenError::from)
        }
        Command::Scaffold => {
            eprintln!("scaffold: TODO (template-engineer)");
            Ok(0)
        }
        Command::Chain => {
            eprintln!("chain: TODO");
            Ok(0)
        }
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
