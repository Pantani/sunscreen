//! Root clap definition and dispatch for sunscreen.
//!
//! Cold-start sensitive: avoid heavy initialization at parse time.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;

use crate::cli::app::{self, AppCmd};
use crate::cli::chain::{self, ChainCmd};
use crate::cli::generate::{self, GenerateCmd};
#[cfg(feature = "onboarding")]
use crate::cli::onboarding::{
    self, DeployArgs, ExamplesCmd, InitArgs, LearnArgs, QuickstartArgs, WalletCmd,
};
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
        /// Attempt automatic repairs for missing or outdated tools.
        #[arg(long, default_value_t = false)]
        fix: bool,
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
    /// Code generation utilities (clients, IDL, frontend hooks).
    Generate {
        #[command(subcommand)]
        cmd: GenerateCmd,
    },
    #[cfg(feature = "onboarding")]
    /// Beginner-friendly workspace wizard.
    Init(InitArgs),
    #[cfg(feature = "onboarding")]
    /// Browse and copy embedded starter examples.
    Examples {
        #[command(subcommand)]
        cmd: ExamplesCmd,
    },
    #[cfg(feature = "onboarding")]
    /// Create a complete starter dApp from a recipe.
    Quickstart(QuickstartArgs),
    #[cfg(feature = "onboarding")]
    /// Manage local Solana wallets.
    Wallet {
        #[command(subcommand)]
        cmd: WalletCmd,
    },
    #[cfg(feature = "onboarding")]
    /// Deploy programs to a Solana cluster.
    Deploy(DeployArgs),
    #[cfg(feature = "onboarding")]
    /// Read embedded Solana/Anchor learning topics.
    Learn(LearnArgs),
    /// Declarative lifecycle for application plugins in `sunscreen.yml`.
    App {
        #[command(subcommand)]
        cmd: AppCmd,
    },
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
                    "next_step": err.next_step(),
                    "exit_code": err.exit_code(),
                });
                eprintln!("{payload}");
            } else {
                eprintln!("error: {err}");
                if let Some(next_step) = err.next_step().filter(|value| !value.is_empty()) {
                    if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
                        eprintln!("{} {next_step}", "try:".cyan());
                    }
                }
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
        Command::Doctor { component, fix } => {
            doctor::run(cli.json, cli.config.as_deref(), component.as_deref(), *fix)
                .map_err(SunscreenError::from)
        }
        Command::Scaffold { cmd } => scaffold::run(cmd, cli.json),
        Command::Chain { cmd } => chain::run(cmd, cli.json),
        Command::Generate { cmd } => generate::run(cmd, cli.json),
        #[cfg(feature = "onboarding")]
        Command::Init(args) => onboarding::run_init(args, cli.json),
        #[cfg(feature = "onboarding")]
        Command::Examples { cmd } => onboarding::run_examples(cmd, cli.json),
        #[cfg(feature = "onboarding")]
        Command::Quickstart(args) => onboarding::run_quickstart(args, cli.json),
        #[cfg(feature = "onboarding")]
        Command::Wallet { cmd } => onboarding::run_wallet(cmd, cli.json),
        #[cfg(feature = "onboarding")]
        Command::Deploy(args) => onboarding::run_deploy(args, cli.json),
        #[cfg(feature = "onboarding")]
        Command::Learn(args) => onboarding::run_learn(args, cli.json),
        Command::App { cmd } => app::run(cmd, cli.json),
    }
}
