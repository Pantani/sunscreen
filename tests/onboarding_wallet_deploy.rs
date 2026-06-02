use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

struct FakeBins {
    dir: tempfile::TempDir,
    log: PathBuf,
}

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

fn write_fake(path: PathBuf, body: &str) {
    std::fs::write(&path, body).unwrap();
    #[cfg(unix)]
    {
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
    }
}

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
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["command"], "wallet_new");
    assert!(fake.log().contains("solana-keygen new --outfile"));
}

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
    assert_eq!(payload["kind"], "network");
    assert_eq!(payload["exit_code"], 8);
}

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
    assert_eq!(denied_payload["kind"], "user_input");

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
    assert_eq!(payload["command"], "deploy");
    assert_eq!(payload["verify"], true);
    let log = fake.log();
    assert!(log.contains("anchor deploy"));
    assert!(log.contains("anchor verify deploy_app"));
}
