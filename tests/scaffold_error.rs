//! End-to-end tests for `sunscreen scaffold error`.

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
fn scaffold_error_creates_errors_rs_and_appends() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("err_app");
    run_chain_new(&ws, "err_app");
    let program_name = discover_program(&ws);

    let out = run_scaffold(
        &ws,
        &[
            "scaffold",
            "error",
            "InsufficientFunds",
            "--program",
            &program_name,
            "--msg",
            "not enough lamports",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "scaffold error failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let errors_rs = ws
        .join("programs")
        .join(&program_name)
        .join("src/errors.rs");
    assert!(errors_rs.exists());
    let contents = std::fs::read_to_string(&errors_rs).unwrap();
    assert!(contents.contains("#[error_code]"));
    assert!(contents.contains("pub enum "));
    assert!(contents.contains("InsufficientFunds,"));
    assert!(contents.contains("#[msg(\"not enough lamports\")]"));
    assert!(contents.contains("segment=error_variants"));

    // Add second variant.
    let out2 = run_scaffold(
        &ws,
        &[
            "scaffold",
            "error",
            "Unauthorized",
            "--program",
            &program_name,
            "--msg",
            "caller is not authorized",
            "--json",
        ],
    );
    assert!(out2.status.success());
    let c2 = std::fs::read_to_string(&errors_rs).unwrap();
    assert!(c2.contains("InsufficientFunds,"));
    assert!(c2.contains("Unauthorized,"));

    // Re-add: no-op.
    let again = run_scaffold(
        &ws,
        &[
            "scaffold",
            "error",
            "InsufficientFunds",
            "--program",
            &program_name,
            "--msg",
            "not enough lamports",
            "--json",
        ],
    );
    assert_eq!(again.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&again.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        payload.get("unchanged").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Conflict: same name, different message → exit 4.
    let conflict = run_scaffold(
        &ws,
        &[
            "scaffold",
            "error",
            "InsufficientFunds",
            "--program",
            &program_name,
            "--msg",
            "different message",
        ],
    );
    assert_eq!(conflict.status.code(), Some(4));

    // Dry-run leaves disk untouched.
    let before = std::fs::read_to_string(&errors_rs).unwrap();
    let dry = run_scaffold(
        &ws,
        &[
            "scaffold",
            "error",
            "RateLimited",
            "--program",
            &program_name,
            "--msg",
            "slow down",
            "--dry-run",
        ],
    );
    assert!(dry.status.success());
    let after = std::fs::read_to_string(&errors_rs).unwrap();
    assert_eq!(before, after);
}
