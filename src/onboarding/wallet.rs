//! Friendly wallet wrappers around the Solana CLI.

use std::path::{Path, PathBuf};

use crate::cli::onboarding::{
    ClusterArg, WalletAirdropArgs, WalletBalanceArgs, WalletCmd, WalletNewArgs,
    WalletSetDefaultArgs,
};
use crate::error::SunscreenError;
use crate::runtime::subprocess::{
    CommandOutput, CommandSpec, ProcessError, ProcessRunner, SubprocessRunner,
};
use crate::workspace;

pub fn run(cmd: &WalletCmd, json: bool) -> Result<i32, SunscreenError> {
    match cmd {
        WalletCmd::New(args) => run_new(args, json, &SubprocessRunner),
        WalletCmd::List => run_list(json),
        WalletCmd::Airdrop(args) => run_airdrop(args, json, &SubprocessRunner),
        WalletCmd::Balance(args) => run_balance(args, json, &SubprocessRunner),
        WalletCmd::SetDefault(args) => run_set_default(args, json),
    }
}

fn run_new<R: ProcessRunner>(
    args: &WalletNewArgs,
    json: bool,
    runner: &R,
) -> Result<i32, SunscreenError> {
    let out = wallet_output_path(args);
    if args.dry_run {
        emit_wallet_new(json, &out, true, None);
        return Ok(0);
    }
    if out.exists() {
        return Err(SunscreenError::PathConflict(format!(
            "wallet already exists: {}",
            out.display()
        )));
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            SunscreenError::Other(anyhow::anyhow!(
                "create wallet dir {}: {err}",
                parent.display()
            ))
        })?;
    }
    let mut spec = CommandSpec::new("solana-keygen")
        .arg("new")
        .arg("--outfile")
        .arg(out.as_os_str());
    if args.no_bip39_passphrase {
        spec = spec.arg("--no-bip39-passphrase");
    }
    let output = runner
        .run(spec)
        .map_err(map_process_missing("solana-keygen"))?;
    if !output.success() {
        return Err(SunscreenError::Other(anyhow::anyhow!(
            "solana-keygen failed with exit {}: {}",
            output.exit_code,
            output.stderr
        )));
    }
    emit_wallet_new(json, &out, false, Some(&output));
    Ok(0)
}

fn run_list(json: bool) -> Result<i32, SunscreenError> {
    let mut wallets = Vec::new();
    let default = default_wallet_path();
    wallets.push(wallet_entry("solana-default", &default, true));
    if let Ok(ws) = workspace::find_root(None) {
        let dir = ws.root.join(".sunscreen/wallets");
        if dir.is_dir() {
            let mut entries = std::fs::read_dir(&dir)
                .map_err(|err| {
                    SunscreenError::Other(anyhow::anyhow!("read {}: {err}", dir.display()))
                })?
                .flatten()
                .filter(|entry| entry.path().is_file())
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.path());
            for entry in entries {
                let name = entry
                    .path()
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("wallet")
                    .to_string();
                wallets.push(wallet_entry(&name, &entry.path(), false));
            }
        }
    }
    if json {
        println!("{}", serde_json::json!({ "ok": true, "wallets": wallets }));
    } else {
        for wallet in wallets {
            println!(
                "{}\t{}\t{}",
                wallet["name"].as_str().unwrap_or("wallet"),
                wallet["path"].as_str().unwrap_or(""),
                if wallet["exists"].as_bool().unwrap_or(false) {
                    "exists"
                } else {
                    "missing"
                }
            );
        }
    }
    Ok(0)
}

fn run_airdrop<R: ProcessRunner>(
    args: &WalletAirdropArgs,
    json: bool,
    runner: &R,
) -> Result<i32, SunscreenError> {
    if args.amount <= 0.0 {
        return Err(SunscreenError::UserInput(
            "airdrop amount must be greater than zero".into(),
        ));
    }
    let mut spec = CommandSpec::new("solana")
        .arg("airdrop")
        .arg(args.amount.to_string())
        .arg("--url")
        .arg(cluster_url(args.cluster));
    if let Some(to) = &args.to {
        spec = spec.arg(to);
    }
    if args.dry_run {
        emit_command_plan(json, "wallet_airdrop", &spec, args.cluster);
        return Ok(0);
    }
    let output = runner.run(spec).map_err(map_process_missing("solana"))?;
    if !output.success() {
        return Err(SunscreenError::Network(format!(
            "airdrop failed with exit {}: {}",
            output.exit_code, output.stderr
        )));
    }
    emit_command_output(json, "wallet_airdrop", args.cluster, &output);
    Ok(0)
}

fn run_balance<R: ProcessRunner>(
    args: &WalletBalanceArgs,
    json: bool,
    runner: &R,
) -> Result<i32, SunscreenError> {
    let mut spec = CommandSpec::new("solana")
        .arg("balance")
        .arg("--url")
        .arg(cluster_url(args.cluster));
    if let Some(address) = &args.address {
        spec = spec.arg(address);
    }
    let output = runner.run(spec).map_err(map_process_missing("solana"))?;
    if !output.success() {
        return Err(SunscreenError::Network(format!(
            "balance failed with exit {}: {}",
            output.exit_code, output.stderr
        )));
    }
    emit_command_output(json, "wallet_balance", args.cluster, &output);
    Ok(0)
}

fn run_set_default(args: &WalletSetDefaultArgs, json: bool) -> Result<i32, SunscreenError> {
    let ws = workspace::find_root(None)?;
    let wallet = resolve_wallet_reference(&ws.root, &args.name);
    if !wallet.exists() {
        return Err(SunscreenError::UserInput(format!(
            "wallet `{}` does not exist at {}; run `sunscreen wallet new {}` first",
            args.name,
            wallet.display(),
            args.name
        )));
    }
    let mut cfg = ws.config.clone();
    match args.cluster {
        ClusterArg::Localnet => cfg.clusters.localnet.wallet = wallet.display().to_string(),
        ClusterArg::Devnet => cfg.clusters.devnet.wallet = wallet.display().to_string(),
        ClusterArg::Mainnet => cfg.clusters.mainnet.wallet = wallet.display().to_string(),
    }
    let rendered = serde_yaml::to_string(&cfg)
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("serialize config: {err}")))?;
    std::fs::write(&ws.config_path, rendered).map_err(|err| {
        SunscreenError::Other(anyhow::anyhow!("write {}: {err}", ws.config_path.display()))
    })?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "wallet_set_default",
                "cluster": args.cluster.as_str(),
                "wallet": wallet.display().to_string(),
                "config": ws.config_path.display().to_string(),
            })
        );
    } else {
        println!(
            "set {} wallet to {}",
            args.cluster.as_str(),
            wallet.display()
        );
    }
    Ok(0)
}

fn wallet_output_path(args: &WalletNewArgs) -> PathBuf {
    if let Some(out) = &args.out {
        return expand_home(out);
    }
    if let Some(name) = args.name.as_ref().filter(|value| !value.trim().is_empty()) {
        return PathBuf::from(".sunscreen/wallets").join(format!("{name}.json"));
    }
    default_wallet_path()
}

fn default_wallet_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/solana/id.json")
}

fn resolve_wallet_reference(workspace_root: &Path, name: &str) -> PathBuf {
    let path = expand_home(Path::new(name));
    if path.components().count() > 1 || path.extension().is_some() {
        path
    } else {
        workspace_root
            .join(".sunscreen/wallets")
            .join(format!("{name}.json"))
    }
}

fn expand_home(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if raw == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from("."));
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir().unwrap_or_else(|| PathBuf::from(".")).join(rest);
    }
    path.to_path_buf()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn cluster_url(cluster: ClusterArg) -> &'static str {
    match cluster {
        ClusterArg::Localnet => "http://127.0.0.1:8899",
        ClusterArg::Devnet => "https://api.devnet.solana.com",
        ClusterArg::Mainnet => "https://api.mainnet-beta.solana.com",
    }
}

fn wallet_entry(name: &str, path: &Path, default: bool) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "path": path.display().to_string(),
        "default": default,
        "exists": path.exists(),
    })
}

fn emit_wallet_new(json: bool, out: &Path, dry_run: bool, output: Option<&CommandOutput>) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "wallet_new",
                "path": out.display().to_string(),
                "dry_run": dry_run,
                "stdout": output.map(|value| value.stdout.clone()),
                "stderr": output.map(|value| value.stderr.clone()),
                "next_step": "sunscreen wallet airdrop 1 --cluster devnet",
            })
        );
    } else if dry_run {
        println!("dry-run: would create wallet at {}", out.display());
    } else {
        println!("created wallet at {}", out.display());
    }
}

fn emit_command_plan(json: bool, command: &str, spec: &CommandSpec, cluster: ClusterArg) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": command,
                "cluster": cluster.as_str(),
                "dry_run": true,
                "argv": spec.display_argv(),
            })
        );
    } else {
        println!("dry-run: {}", spec.display_argv().join(" "));
    }
}

fn emit_command_output(json: bool, command: &str, cluster: ClusterArg, output: &CommandOutput) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": command,
                "cluster": cluster.as_str(),
                "stdout": output.stdout,
                "stderr": output.stderr,
            })
        );
    } else if output.stdout.trim().is_empty() {
        println!("{command}: ok");
    } else {
        print!("{}", output.stdout);
    }
}

fn map_process_missing(tool: &'static str) -> impl FnOnce(ProcessError) -> SunscreenError {
    move |err| {
        if err.is_not_found() {
            SunscreenError::ToolchainMissing(format!(
                "{tool} not found on PATH; install Solana CLI tooling"
            ))
        } else {
            SunscreenError::Other(anyhow::anyhow!("{tool}: {err}"))
        }
    }
}
