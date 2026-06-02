#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

#[cfg(unix)]
struct FakeBins {
    dir: tempfile::TempDir,
    log: PathBuf,
}

#[cfg(unix)]
impl FakeBins {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("argv.log");
        write_fake(
            dir.path().join("solana-keygen"),
            r#"#!/bin/sh
echo "solana-keygen $@" >> "$SUNSCREEN_FAKE_LOG"
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--outfile" ]; then
    shift
    out="$1"
  fi
  shift
done
mkdir -p "$(dirname "$out")"
printf '[1,2,3,4]\n' > "$out"
echo "pubkey: Fake111111111111111111111111111111111111111"
"#,
        );
        write_fake(
            dir.path().join("solana"),
            r#"#!/bin/sh
echo "solana $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "$SUNSCREEN_FAKE_FAIL" = "network" ]; then
  echo "rate limited" >&2
  exit 42
fi
echo "1 SOL"
"#,
        );
        write_fake(
            dir.path().join("anchor"),
            r#"#!/bin/sh
echo "anchor $@" >> "$SUNSCREEN_FAKE_LOG"
echo "anchor ok"
"#,
        );
        Self { dir, log }
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(sunscreen_bin());
        cmd.env("SUNSCREEN_FAKE_LOG", &self.log);
        let old_path = std::env::var_os("PATH").unwrap_or_default();
        let path = format!(
            "{}:{}",
            self.dir.path().display(),
            old_path.to_string_lossy()
        );
        cmd.env("PATH", path);
        cmd
    }

    fn log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }
}

#[cfg(unix)]
fn write_fake(path: PathBuf, body: &str) {
    std::fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
    }
}

#[cfg(unix)]
fn chain_new(path: &Path, name: &str) {
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(["chain", "new", name, "--frontend", "none", "--path"])
        .arg(path)
        .output()
        .expect("chain new");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[cfg(unix)]
fn normalize_json_strings(value: &mut serde_json::Value, from: &str, to: &str) {
    match value {
        serde_json::Value::String(text) => {
            *text = text.replace(from, to);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_json_strings(item, from, to);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                normalize_json_strings(item, from, to);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

#[cfg(unix)]
#[test]
fn wallet_new_uses_solana_keygen_boundary() {
    let fake = FakeBins::new();
    let tmp = tempfile::tempdir().unwrap();
    let wallet = tmp.path().join("id.json");
    let out = fake
        .command()
        .args([
            "--json",
            "wallet",
            "new",
            "--out",
            wallet.to_str().unwrap(),
            "--no-bip39-passphrase",
        ])
        .output()
        .expect("wallet new");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(wallet.exists());
    let mut payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    normalize_json_strings(&mut payload, &wallet.display().to_string(), "[WALLET_PATH]");
    insta::assert_json_snapshot!("wallet_new_outfile", payload);
    assert!(fake.log().contains("solana-keygen new --outfile"));
}

#[cfg(unix)]
#[test]
fn wallet_new_named_wallet_is_workspace_rooted() {
    let fake = FakeBins::new();
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("wallet_app");
    chain_new(&ws, "wallet_app");
    let subdir = ws.join("programs/wallet_app/src");
    let expected = ws.join(".sunscreen/wallets/treasury.json");

    let out = fake
        .command()
        .current_dir(&subdir)
        .args([
            "--json",
            "wallet",
            "new",
            "treasury",
            "--no-bip39-passphrase",
        ])
        .output()
        .expect("wallet new --name");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(expected.exists());
    let mut payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    payload["path"] =
        serde_json::Value::String("[WORKSPACE]/.sunscreen/wallets/treasury.json".to_string());
    insta::assert_json_snapshot!("wallet_new_named_workspace_rooted", payload);
}

#[cfg(unix)]
#[test]
fn wallet_airdrop_maps_rpc_failure_to_network_exit_8() {
    let fake = FakeBins::new();
    let out = fake
        .command()
        .env("SUNSCREEN_FAKE_FAIL", "network")
        .args(["--json", "wallet", "airdrop", "1", "--cluster", "devnet"])
        .output()
        .expect("wallet airdrop");
    assert_eq!(out.status.code(), Some(8));
    let payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    insta::assert_json_snapshot!("wallet_airdrop_network_error", payload);
}

#[cfg(unix)]
#[test]
fn deploy_guards_mainnet_and_uses_anchor_boundary() {
    let fake = FakeBins::new();
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("deploy_app");
    chain_new(&ws, "deploy_app");

    let denied = fake
        .command()
        .current_dir(&ws)
        .args(["--json", "deploy", "mainnet", "--program", "deploy_app"])
        .output()
        .expect("deploy mainnet");
    assert_eq!(denied.status.code(), Some(4));
    let denied_payload: serde_json::Value = serde_json::from_slice(&denied.stderr).unwrap();
    insta::assert_json_snapshot!("deploy_mainnet_requires_confirmation_error", denied_payload);

    let mainnet_plan = fake
        .command()
        .current_dir(&ws)
        .args([
            "--json",
            "deploy",
            "mainnet",
            "--program",
            "deploy_app",
            "--yes-i-understand-cost",
            "--dry-run",
        ])
        .output()
        .expect("deploy mainnet dry-run");
    assert!(
        mainnet_plan.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&mainnet_plan.stderr)
    );
    let mainnet_payload: serde_json::Value = serde_json::from_slice(&mainnet_plan.stdout).unwrap();
    insta::assert_json_snapshot!(
        "deploy_mainnet_dry_run_uses_anchor_moniker",
        mainnet_payload
    );

    let ok = fake
        .command()
        .current_dir(&ws)
        .args([
            "--json",
            "deploy",
            "devnet",
            "--program",
            "deploy_app",
            "--verify",
        ])
        .output()
        .expect("deploy devnet");
    assert!(
        ok.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&ok.stdout).unwrap();
    insta::assert_json_snapshot!("deploy_devnet_verify", payload);
    let log = fake.log();
    assert!(log.contains("anchor deploy"));
    assert!(log.contains("anchor verify deploy_app"));
}
