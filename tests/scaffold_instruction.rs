//! End-to-end tests for `sunscreen scaffold instruction`.
//!
//! Workflow exercised:
//! 1. Bootstrap a workspace via `chain new`.
//! 2. Run `scaffold instruction <name> --program <p> --args ...`.
//! 3. Validate the generated files + that `mod.rs` was patched.
//! 4. Re-run the same command (should be a no-op, idempotent).

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

#[test]
fn scaffold_instruction_creates_file_and_patches_mod() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("demo_app");
    run_chain_new(&ws, "demo_app");

    // The program path in config is `programs/demo-app` (kebab-case
    // normalisation), but the on-disk crate folder is whatever `chain new`
    // emitted. Inspect the layout to discover the actual program name.
    let programs_dir = ws.join("programs");
    let entries: Vec<_> = std::fs::read_dir(&programs_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "expected exactly one program dir");
    let program_name = entries[0].file_name().to_string_lossy().into_owned();

    // 1) Scaffold an instruction.
    let out = run_scaffold(
        &ws,
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            &program_name,
            "--args",
            "amount:u64",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "scaffold failed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    let program_dir = programs_dir.join(&program_name);
    let ix_file = program_dir.join("src/instructions/deposit.rs");
    let mod_file = program_dir.join("src/instructions/mod.rs");
    assert!(ix_file.exists(), "instruction file missing");
    assert!(mod_file.exists(), "mod.rs missing");

    let mod_contents = std::fs::read_to_string(&mod_file).unwrap();
    assert!(mod_contents.contains("pub mod deposit;"));
    assert!(mod_contents.contains("pub use deposit::*;"));
    assert!(mod_contents.contains("sunscreen:auto-generated:begin segment=instructions"));

    let ix_contents = std::fs::read_to_string(&ix_file).unwrap();
    assert!(ix_contents.contains("Deposit"));
    assert!(ix_contents.contains("amount: u64"));

    // 2) Re-run with identical args: idempotent no-op (exit 0, unchanged=true).
    let again = run_scaffold(
        &ws,
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            &program_name,
            "--args",
            "amount:u64",
            "--json",
        ],
    );
    assert_eq!(
        again.status.code(),
        Some(0),
        "re-run should be idempotent; stderr={}",
        String::from_utf8_lossy(&again.stderr)
    );
    let stdout = String::from_utf8_lossy(&again.stdout);
    assert!(
        stdout.contains("\"unchanged\":true") || stdout.contains("\"unchanged\": true"),
        "expected unchanged=true in JSON output, got: {stdout}"
    );

    // 3) Files on disk are byte-stable across the re-run.
    let snapshot = std::fs::read_to_string(&mod_file).unwrap();
    assert_eq!(snapshot, mod_contents);
    let ix_snapshot = std::fs::read_to_string(&ix_file).unwrap();
    assert_eq!(ix_snapshot, ix_contents);

    // 4) User-region edits are PRESERVED across re-runs (not treated as drift).
    let user_edited = ix_contents.replace(
        "let _ = ctx;",
        "let _ = ctx;\n    msg!(\"custom user logic\");",
    );
    assert_ne!(user_edited, ix_contents, "test setup: edit must apply");
    std::fs::write(&ix_file, &user_edited).unwrap();
    let preserved = run_scaffold(
        &ws,
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            &program_name,
            "--args",
            "amount:u64",
        ],
    );
    assert_eq!(
        preserved.status.code(),
        Some(0),
        "user-region edit must NOT drift; stderr={}",
        String::from_utf8_lossy(&preserved.stderr)
    );
    let after = std::fs::read_to_string(&ix_file).unwrap();
    assert!(
        after.contains("msg!(\"custom user logic\");"),
        "user-region content must be preserved across re-run, got: {after}"
    );
}

#[test]
fn scaffold_instruction_outside_workspace_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(sunscreen_bin())
        .current_dir(tmp.path())
        .args(["scaffold", "instruction", "foo", "--program", "anything"])
        .output()
        .expect("invoke sunscreen");
    // Exit code 5 = workspace missing.
    assert_eq!(out.status.code(), Some(5));
}

#[test]
fn scaffold_instruction_invalid_args_exit_4() {
    // args/accounts parsing happens before workspace discovery, so a
    // malformed --args value yields exit 4 even outside a real workspace.
    let tmp = tempfile::tempdir().unwrap();
    let out = run_scaffold(
        tmp.path(),
        &[
            "scaffold",
            "instruction",
            "foo",
            "--program",
            "any",
            "--args",
            "noType",
        ],
    );
    assert_eq!(out.status.code(), Some(4));
}
