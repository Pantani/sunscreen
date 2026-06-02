//! Beginner onboarding command surface.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::cli::chain::Frontend;
use crate::error::SunscreenError;

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Project name. Required in non-interactive mode.
    pub name: Option<String>,
    /// Preset to apply when no prompts are available.
    #[arg(long, value_name = "NAME")]
    pub from_preset: Option<String>,
    /// Disable prompts and require flag-based input.
    #[arg(long, default_value_t = false)]
    pub non_interactive: bool,
    /// Frontend flavor to scaffold.
    #[arg(long, value_enum, default_value_t = Frontend::Vite)]
    pub frontend: Frontend,
    /// Output directory. Defaults to `./<name>`.
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
    /// Print planned files without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Subcommand)]
pub enum ExamplesCmd {
    /// List embedded examples.
    List(ExamplesListArgs),
    /// Show one embedded example README.
    Describe(ExampleNameArgs),
    /// Copy an embedded example to disk.
    Use(ExampleUseArgs),
}

#[derive(Debug, Args)]
pub struct ExamplesListArgs {
    /// Filter examples by tag.
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExampleNameArgs {
    /// Embedded example name.
    pub name: String,
}

#[derive(Debug, Args)]
pub struct ExampleUseArgs {
    /// Embedded example name.
    pub name: String,
    /// Output directory. Defaults to `./<example-name>`.
    pub path: Option<PathBuf>,
    /// Disable prompts and fail on conflicts.
    #[arg(long, default_value_t = false)]
    pub non_interactive: bool,
    /// Print planned files without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct QuickstartArgs {
    /// Quickstart recipe to compose.
    #[arg(value_enum)]
    pub recipe: QuickstartRecipeArg,
    /// Project name. Required in non-interactive mode.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// Cluster target for the generated next steps.
    #[arg(long, value_enum, default_value_t = ClusterArg::Localnet)]
    pub cluster: ClusterArg,
    /// Disable prompts and require flag-based input.
    #[arg(long, default_value_t = false)]
    pub non_interactive: bool,
    /// Frontend flavor to scaffold.
    #[arg(long, value_enum, default_value_t = Frontend::Vite)]
    pub frontend: Frontend,
    /// Output directory. Defaults to `./<name>`.
    #[arg(long, value_name = "DIR")]
    pub path: Option<PathBuf>,
    /// Print planned operations without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum QuickstartRecipeArg {
    Token,
    Nft,
    Dao,
    Blog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClusterArg {
    Localnet,
    Devnet,
    Mainnet,
}

impl ClusterArg {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Localnet => "localnet",
            Self::Devnet => "devnet",
            Self::Mainnet => "mainnet",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum WalletCmd {
    /// Create a new Solana keypair.
    New(WalletNewArgs),
    /// List known local wallets.
    List,
    /// Request a cluster airdrop.
    Airdrop(WalletAirdropArgs),
    /// Print a wallet balance.
    Balance(WalletBalanceArgs),
    /// Set the default wallet path in sunscreen.yml.
    SetDefault(WalletSetDefaultArgs),
}

#[derive(Debug, Args)]
pub struct WalletNewArgs {
    /// Friendly wallet name. Used for `.sunscreen/wallets/<name>.json` when --out is omitted.
    pub name: Option<String>,
    /// Output keypair path.
    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,
    /// Pass through to `solana-keygen new`.
    #[arg(long, default_value_t = false)]
    pub no_bip39_passphrase: bool,
    /// Print the command without creating a keypair.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct WalletAirdropArgs {
    /// Amount of SOL to request.
    #[arg(default_value_t = 1.0)]
    pub amount: f64,
    /// Cluster to use.
    #[arg(long, value_enum, default_value_t = ClusterArg::Devnet)]
    pub cluster: ClusterArg,
    /// Recipient pubkey. Defaults to the Solana CLI default keypair.
    #[arg(long, value_name = "PUBKEY")]
    pub to: Option<String>,
    /// Print the command without requesting an airdrop.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct WalletBalanceArgs {
    /// Cluster to use.
    #[arg(long, value_enum, default_value_t = ClusterArg::Devnet)]
    pub cluster: ClusterArg,
    /// Address or keypair path. Defaults to Solana CLI default.
    pub address: Option<String>,
}

#[derive(Debug, Args)]
pub struct WalletSetDefaultArgs {
    /// Wallet name under `.sunscreen/wallets/` or a direct path.
    pub name: String,
    /// Cluster config to update.
    #[arg(long, value_enum, default_value_t = ClusterArg::Localnet)]
    pub cluster: ClusterArg,
}

#[derive(Debug, Args)]
pub struct DeployArgs {
    /// Deployment target.
    #[arg(value_enum)]
    pub target: ClusterArg,
    /// Optional program name to pass through to Anchor.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Run `anchor verify` after deploy when supported.
    #[arg(long, default_value_t = false)]
    pub verify: bool,
    /// Required for mainnet deployments.
    #[arg(long, default_value_t = false)]
    pub yes_i_understand_cost: bool,
    /// Print deployment plan without running Anchor.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct LearnArgs {
    /// Topic to render. Omit to list available topics.
    pub topic: Option<String>,
}

pub fn run_init(args: &InitArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::wizard::run(args, json)
}

pub fn run_examples(cmd: &ExamplesCmd, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::examples::run(cmd, json)
}

pub fn run_quickstart(args: &QuickstartArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::recipes::run(args, json)
}

pub fn run_wallet(cmd: &WalletCmd, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::wallet::run(cmd, json)
}

pub fn run_deploy(args: &DeployArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::deploy::run(args, json)
}

pub fn run_learn(args: &LearnArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::learn::run(args, json)
}
