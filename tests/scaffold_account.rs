//! End-to-end tests for `sunscreen scaffold account`.

use std::path::Path;
use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

fn run_chain_new(out_path: &Path, name: &str) {
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(["chain", "new", name, "--frontend", "none", "--path"])
        .arg(out_path)
        .output()
        .expect("invoke sunscreen chain new");
    assert!(
        out.status.success(),
        "chain new failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run_scaffold(workspace: &Path, args: &[&str]) -> std::process::Output {
    Command::new(sunscreen_bin())
        .current_dir(workspace)
        .env_remove("SUNSCREEN_SKIP_PREFLIGHT")
        .args(args)
        .output()
        .expect("invoke sunscreen scaffold")
}

fn discover_program(ws: &Path) -> String {
    let programs_dir = ws.join("programs");
    let entries: Vec<_> = std::fs::read_dir(&programs_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "expected exactly one program dir");
    entries[0].file_name().to_string_lossy().into_owned()
}

#[test]
fn scaffold_account_creates_file_and_patches_mod() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("acct_app");
    run_chain_new(&ws, "acct_app");
    let program_name = discover_program(&ws);

    let out = run_scaffold(
        &ws,
        &[
            "scaffold",
            "account",
            "Vault",
            "--program",
            &program_name,
            "--fields",
            "owner:Pubkey,total:u64",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "scaffold account failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let program_dir = ws.join("programs").join(&program_name);
    let acct_file = program_dir.join("src/state/vault.rs");
    let mod_file = program_dir.join("src/state/mod.rs");
    assert!(acct_file.exists(), "account file missing");
    assert!(mod_file.exists(), "state/mod.rs missing");

    let mod_contents = std::fs::read_to_string(&mod_file).unwrap();
    assert!(mod_contents.contains("pub mod vault;"));
    assert!(mod_contents.contains("pub use vault::*;"));
    assert!(mod_contents.contains("sunscreen:auto-generated:begin segment=accounts"));

    let acct_contents = std::fs::read_to_string(&acct_file).unwrap();
    assert!(acct_contents.contains("#[account]"));
    assert!(acct_contents.contains("pub struct Vault"));
    assert!(acct_contents.contains("pub owner: Pubkey,"));
    assert!(acct_contents.contains("pub total: u64,"));

    // Re-run: idempotent no-op.
    let again = run_scaffold(
        &ws,
        &[
            "scaffold",
            "account",
            "Vault",
            "--program",
            &program_name,
            "--fields",
            "owner:Pubkey,total:u64",
            "--json",
        ],
    );
    assert_eq!(again.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&again.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not valid JSON ({e}): {stdout}"));
    assert_eq!(
        payload.get("unchanged").and_then(|v| v.as_bool()),
        Some(true)
    );

    let snap = std::fs::read_to_string(&mod_file).unwrap();
    assert_eq!(snap, mod_contents);

    // Dry-run: does not touch disk even with new args.
    let acct_before = std::fs::read_to_string(&acct_file).unwrap();
    let dry = run_scaffold(
        &ws,
        &[
            "scaffold",
            "account",
            "Treasury",
            "--program",
            &program_name,
            "--fields",
            "balance:u64",
            "--dry-run",
            "--json",
        ],
    );
    assert!(dry.status.success());
    assert!(!program_dir.join("src/state/treasury.rs").exists());
    let acct_after = std::fs::read_to_string(&acct_file).unwrap();
    assert_eq!(acct_before, acct_after);
}

#[test]
fn scaffold_account_invalid_field_exit_4() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_scaffold(
        tmp.path(),
        &[
            "scaffold",
            "account",
            "Foo",
            "--program",
            "any",
            "--fields",
            "noType",
        ],
    );
    assert_eq!(out.status.code(), Some(4));
}

#[test]
fn scaffold_account_outside_workspace_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(sunscreen_bin())
        .current_dir(tmp.path())
        .args(["scaffold", "account", "Foo", "--program", "any"])
        .output()
        .expect("invoke sunscreen");
    assert_eq!(out.status.code(), Some(5));
}
