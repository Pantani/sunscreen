//! End-to-end tests for `sunscreen scaffold instruction`.
//!
//! Workflow exercised:
//! 1. Bootstrap a workspace via `chain new`.
//! 2. Run `scaffold instruction <name> --program <p> --args ...`.
//! 3. Validate the generated files + that `mod.rs` was patched.
//! 4. Re-run the same command (should be a no-op, idempotent).

use std::path::{Path, PathBuf};
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
    let (_tmp, ws, program_name) = instruction_workspace("demo_app");
    scaffold_deposit_instruction(&ws, &program_name, true);

    let program_dir = ws.join("programs").join(&program_name);
    let ix_file = program_dir.join("src/instructions/deposit.rs");
    let mod_file = program_dir.join("src/instructions/mod.rs");
    assert_instruction_outputs(&ix_file, &mod_file);

    let mod_contents = std::fs::read_to_string(mod_file.as_path()).unwrap();
    let ix_contents = std::fs::read_to_string(ix_file.as_path()).unwrap();
    assert_instruction_rerun(
        &ws,
        &program_name,
        &mod_file,
        &mod_contents,
        &ix_file,
        &ix_contents,
    );
    assert_user_region_preserved(&ws, &program_name, &ix_file, &ix_contents);
}

fn instruction_workspace(name: &str) -> (tempfile::TempDir, PathBuf, String) {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join(name);
    run_chain_new(&ws, name);
    let program_name = discover_program(&ws);
    (tmp, ws, program_name)
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

fn scaffold_deposit_instruction(ws: &Path, program_name: &str, json: bool) {
    let mut args = vec![
        "scaffold",
        "instruction",
        "deposit",
        "--program",
        program_name,
        "--args",
        "amount:u64",
    ];
    if json {
        args.push("--json");
    }
    let out = run_scaffold(ws, &args);
    assert!(
        out.status.success(),
        "scaffold failed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_instruction_outputs(ix_file: &Path, mod_file: &Path) {
    assert!(ix_file.exists(), "instruction file missing");
    assert!(mod_file.exists(), "mod.rs missing");

    let mod_contents = std::fs::read_to_string(mod_file).unwrap();
    assert!(mod_contents.contains("pub mod deposit;"));
    assert!(mod_contents.contains("pub use deposit::*;"));
    assert!(mod_contents.contains("sunscreen:auto-generated:begin segment=instructions"));

    let ix_contents = std::fs::read_to_string(ix_file).unwrap();
    assert!(ix_contents.contains("Deposit"));
    assert!(ix_contents.contains("amount: u64"));
}

fn assert_instruction_rerun(
    ws: &Path,
    program_name: &str,
    mod_file: &Path,
    mod_contents: &str,
    ix_file: &Path,
    ix_contents: &str,
) {
    let again = run_scaffold(
        ws,
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            program_name,
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
    let snapshot = std::fs::read_to_string(mod_file).unwrap();
    assert_eq!(snapshot, mod_contents);
    let ix_snapshot = std::fs::read_to_string(ix_file).unwrap();
    assert_eq!(ix_snapshot, ix_contents);
}

fn assert_user_region_preserved(ws: &Path, program_name: &str, ix_file: &Path, ix_contents: &str) {
    let user_edited = ix_contents.replace(
        "let _ = ctx;",
        "let _ = ctx;\n    msg!(\"custom user logic\");",
    );
    assert_ne!(user_edited, ix_contents, "test setup: edit must apply");
    std::fs::write(ix_file, &user_edited).unwrap();
    let preserved = run_scaffold(
        ws,
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            program_name,
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
    let after = std::fs::read_to_string(ix_file).unwrap();
    insta::assert_snapshot!("user_region_preserved_after_rescaffold", after);
}

#[test]
fn scaffold_instruction_patches_lib_rs_dispatch_segment() {
    // Phase 2 R2 carry-over: a freshly scaffolded workspace must ship with
    // the `segment=dispatch` markers in lib.rs so the first
    // `scaffold instruction` actually patches them (instead of warning
    // "no dispatch segment marker — skipped").
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("dispatch_app");
    run_chain_new(&ws, "dispatch_app");

    let programs_dir = ws.join("programs");
    let entries: Vec<_> = std::fs::read_dir(&programs_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one program directory under programs/, found {}",
        entries.len()
    );
    let program_name = entries[0].file_name().to_string_lossy().into_owned();

    // Sanity: the seeded lib.rs already has the dispatch markers.
    let lib_rs = programs_dir.join(&program_name).join("src/lib.rs");
    let lib_before = std::fs::read_to_string(&lib_rs).unwrap();
    assert!(
        lib_before.contains("sunscreen:auto-generated:begin segment=dispatch"),
        "seeded lib.rs should ship with dispatch markers, got:\n{lib_before}"
    );
    // Sanity: the seeded instructions/mod.rs exists with markers.
    let mod_rs = programs_dir
        .join(&program_name)
        .join("src/instructions/mod.rs");
    let mod_before = std::fs::read_to_string(&mod_rs).unwrap();
    assert!(
        mod_before.contains("sunscreen:auto-generated:begin segment=instructions"),
        "seeded instructions/mod.rs should ship with instructions markers"
    );

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
        "scaffold failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not valid JSON ({e}): {stdout}"));
    assert_eq!(
        payload.get("lib_rs_patched").and_then(|v| v.as_bool()),
        Some(true),
        "expected lib_rs_patched=true; payload={payload}"
    );

    // The patched dispatch segment should now contain the new wrapper.
    // Snapshot the full file so format/marker drift is caught loudly.
    let lib_after = std::fs::read_to_string(&lib_rs).unwrap();
    insta::assert_snapshot!("lib_rs_after_first_scaffold_deposit", lib_after);
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
