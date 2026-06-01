//! End-to-end integration tests against a real Anchor toolchain.
//!
//! These tests are **`#[ignore]`d by default** because the Anchor + Solana
//! toolchain is heavy and not present on every dev box. Run them with:
//!
//! ```bash
//! cargo test --test integration_anchor -- --ignored
//! ```
//!
//! Each test additionally probes for `anchor` (and `codama` for the regen
//! test) at runtime via `which`. If the tool is missing the test prints a
//! skip message and returns success — `--ignored` mode never fails because
//! a dev box lacks the toolchain.
//!
//! Coverage:
//!   1. `chain new` → `anchor build` succeeds.
//!   2. `chain new` + `scaffold instruction` → still builds.
//!   3. `scaffold account` → IDL contains the account type.
//!   4. `scaffold event` → IDL contains the event type.
//!   5. `codama regen` is idempotent (second run produces identical output).
//!
//! Owned by qa-integrator (R5).

use std::path::{Path, PathBuf};
use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

/// Returns `true` if `name` is on `$PATH`.
fn tool_available(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn skip_if_missing(tool: &str) -> bool {
    if !tool_available(tool) {
        eprintln!("SKIP: `{tool}` not installed — integration test skipped");
        return true;
    }
    false
}

fn chain_new(workdir: &Path, name: &str) {
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args([
            "chain",
            "new",
            name,
            "--framework",
            "anchor",
            "--frontend",
            "none",
            "--path",
        ])
        .arg(workdir.join(name))
        .output()
        .expect("invoke sunscreen chain new");
    assert!(
        out.status.success(),
        "chain new failed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn anchor_build(project_dir: &Path) -> std::process::Output {
    Command::new("anchor")
        .arg("build")
        .current_dir(project_dir)
        .output()
        .expect("invoke anchor build")
}

fn read_idl(project_dir: &Path, program: &str) -> Option<String> {
    let p = project_dir
        .join("target")
        .join("idl")
        .join(format!("{program}.json"));
    std::fs::read_to_string(p).ok()
}

fn run_scaffold(project_dir: &Path, args: &[&str]) {
    let out = Command::new(sunscreen_bin())
        .current_dir(project_dir)
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(args)
        .output()
        .expect("invoke sunscreen scaffold");
    assert!(
        out.status.success(),
        "scaffold failed (args={args:?}): exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn tmp_workdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
#[ignore]
fn anchor_build_succeeds_after_chain_new() {
    if skip_if_missing("anchor") {
        return;
    }
    let tmp = tmp_workdir();
    let name = "anchor_build_demo";
    chain_new(tmp.path(), name);

    let out = anchor_build(&tmp.path().join(name));
    assert!(
        out.status.success(),
        "anchor build failed: exit={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
#[ignore]
fn anchor_build_succeeds_after_scaffold_instruction() {
    if skip_if_missing("anchor") {
        return;
    }
    let tmp = tmp_workdir();
    let name = "instr_build_demo";
    chain_new(tmp.path(), name);
    let project = tmp.path().join(name);

    run_scaffold(
        &project,
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            name,
            "--args",
            "amount:u64",
            "--accounts",
            "vault:mut,depositor:signer|mut,system_program:system",
        ],
    );

    let out = anchor_build(&project);
    assert!(
        out.status.success(),
        "anchor build after scaffold instruction failed:\nstderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
#[ignore]
fn scaffold_account_appears_in_idl() {
    if skip_if_missing("anchor") {
        return;
    }
    let tmp = tmp_workdir();
    let name = "acct_idl_demo";
    chain_new(tmp.path(), name);
    let project = tmp.path().join(name);

    run_scaffold(
        &project,
        &[
            "scaffold",
            "account",
            "Vault",
            "--program",
            name,
            "--fields",
            "total:u64,authority:Pubkey,bump:u8",
        ],
    );

    let out = anchor_build(&project);
    assert!(
        out.status.success(),
        "anchor build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let idl = read_idl(&project, name).expect("IDL not produced by anchor build");
    assert!(
        idl.contains("\"Vault\""),
        "IDL does not mention the Vault account:\n{idl}",
    );
}

#[test]
#[ignore]
fn scaffold_event_appears_in_idl() {
    if skip_if_missing("anchor") {
        return;
    }
    let tmp = tmp_workdir();
    let name = "evt_idl_demo";
    chain_new(tmp.path(), name);
    let project = tmp.path().join(name);

    run_scaffold(
        &project,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            name,
            "--fields",
            "depositor:Pubkey,amount:u64",
        ],
    );

    let out = anchor_build(&project);
    assert!(
        out.status.success(),
        "anchor build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let idl = read_idl(&project, name).expect("IDL not produced by anchor build");
    assert!(
        idl.contains("\"Deposited\""),
        "IDL does not mention the Deposited event:\n{idl}",
    );
}

#[test]
#[ignore]
fn codama_regen_is_idempotent() {
    if skip_if_missing("anchor") || skip_if_missing("codama") {
        return;
    }
    let tmp = tmp_workdir();
    let name = "codama_idem_demo";
    chain_new(tmp.path(), name);
    let project = tmp.path().join(name);

    // Need an IDL first.
    let build = anchor_build(&project);
    assert!(
        build.status.success(),
        "anchor build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let idl_path = project
        .join("target")
        .join("idl")
        .join(format!("{name}.json"));
    assert!(idl_path.exists(), "expected IDL at {}", idl_path.display());

    // First codama regen.
    let regen = |project: &Path| -> PathBuf {
        let out_dir = project.join("clients").join("ts");
        Command::new("codama")
            .args(["run", "js"])
            .arg("--idl")
            .arg(&idl_path)
            .arg("--out")
            .arg(&out_dir)
            .current_dir(project)
            .output()
            .expect("invoke codama");
        out_dir
    };

    let out_dir = regen(&project);
    let snapshot_1 = collect_tree(&out_dir);
    let _ = regen(&project);
    let snapshot_2 = collect_tree(&out_dir);

    assert_eq!(
        snapshot_1, snapshot_2,
        "codama regen produced different output on the second run (not idempotent)",
    );
}

/// Walk `root` and return `(relative_path, contents)` pairs sorted by path.
/// Used to compare two regen passes byte-for-byte.
fn collect_tree(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(s) = std::fs::read_to_string(&path) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, s));
            }
        }
    }
    out.sort();
    out
}
