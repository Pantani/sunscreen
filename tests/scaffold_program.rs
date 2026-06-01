//! End-to-end tests for `sunscreen scaffold program`.

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

fn run(ws: &Path, args: &[&str]) -> std::process::Output {
    Command::new(sunscreen_bin())
        .current_dir(ws)
        .args(args)
        .output()
        .expect("invoke sunscreen")
}

#[test]
fn scaffold_program_creates_crate_and_patches_manifests() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("multi_app");
    run_chain_new(&ws, "multi_app");

    let out = run(&ws, &["scaffold", "program", "extra-prog", "--json"]);
    assert!(
        out.status.success(),
        "scaffold program failed: code={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json payload");
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        payload.get("name").and_then(|v| v.as_str()),
        Some("extra-prog")
    );

    // Files on disk.
    let prog_dir = ws.join("programs/extra_prog");
    assert!(prog_dir.is_dir(), "program directory missing");
    let lib_rs = prog_dir.join("src/lib.rs");
    assert!(lib_rs.is_file(), "lib.rs missing");
    let lib = std::fs::read_to_string(&lib_rs).unwrap();
    assert!(
        lib.contains("segment=dispatch version=1"),
        "lib.rs missing segment=dispatch; got:\n{lib}"
    );
    let mod_rs = prog_dir.join("src/instructions/mod.rs");
    let mod_contents = std::fs::read_to_string(&mod_rs).unwrap();
    assert!(mod_contents.contains("segment=instructions"));

    // Anchor.toml patched on both clusters.
    let anchor_toml = std::fs::read_to_string(ws.join("Anchor.toml")).unwrap();
    assert!(
        anchor_toml.contains("[programs.localnet]"),
        "Anchor.toml missing localnet section"
    );
    let localnet_block = anchor_toml.split("[programs.devnet]").next().unwrap();
    assert!(
        localnet_block.contains("extra_prog ="),
        "localnet entry missing: {anchor_toml}"
    );
    let after_devnet = anchor_toml.split("[programs.devnet]").nth(1).unwrap();
    assert!(
        after_devnet.contains("extra_prog ="),
        "devnet entry missing: {anchor_toml}"
    );

    // sunscreen.yml has the new program entry.
    let cfg = std::fs::read_to_string(ws.join("sunscreen.yml")).unwrap();
    assert!(
        cfg.contains("name: extra-prog"),
        "sunscreen.yml not patched:\n{cfg}"
    );
    assert!(cfg.contains("path: programs/extra_prog"));
}

#[test]
fn scaffold_program_idempotent_conflict_exits_4() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("dup_app");
    run_chain_new(&ws, "dup_app");

    let first = run(&ws, &["scaffold", "program", "twin"]);
    assert!(first.status.success(), "first scaffold failed");

    let second = run(&ws, &["scaffold", "program", "twin"]);
    assert_eq!(second.status.code(), Some(4));
}

#[test]
fn scaffold_program_dry_run_touches_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("dry_app");
    run_chain_new(&ws, "dry_app");
    let anchor_before = std::fs::read_to_string(ws.join("Anchor.toml")).unwrap();
    let cfg_before = std::fs::read_to_string(ws.join("sunscreen.yml")).unwrap();

    let out = run(
        &ws,
        &["scaffold", "program", "ghost", "--dry-run", "--json"],
    );
    assert!(out.status.success());
    assert!(!ws.join("programs/ghost").exists());
    assert_eq!(
        std::fs::read_to_string(ws.join("Anchor.toml")).unwrap(),
        anchor_before
    );
    assert_eq!(
        std::fs::read_to_string(ws.join("sunscreen.yml")).unwrap(),
        cfg_before
    );
}

#[test]
fn scaffold_program_rejects_invalid_pubkey() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("badid_app");
    run_chain_new(&ws, "badid_app");
    // Too short to be a base58 32-byte pubkey.
    let out = run(&ws, &["scaffold", "program", "good", "--id", "short"]);
    assert_eq!(out.status.code(), Some(4));
}

#[test]
fn scaffold_program_with_custom_id_propagates_to_lib_rs() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("idapp");
    run_chain_new(&ws, "idapp");
    let custom_id = "11111111111111111111111111111111";
    let out = run(&ws, &["scaffold", "program", "tagged", "--id", custom_id]);
    assert!(
        out.status.success(),
        "scaffold failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lib = std::fs::read_to_string(ws.join("programs/tagged/src/lib.rs")).unwrap();
    assert!(
        lib.contains(&format!("declare_id!(\"{custom_id}\")")),
        "lib.rs declare_id mismatch; got:\n{lib}"
    );
    let anchor = std::fs::read_to_string(ws.join("Anchor.toml")).unwrap();
    assert!(
        anchor.contains(&format!("tagged = \"{custom_id}\"")),
        "Anchor.toml missing custom id; got:\n{anchor}"
    );
}

#[test]
fn chain_doctor_clean_workspace_reports_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("doc_app");
    run_chain_new(&ws, "doc_app");
    let out = run(&ws, &["chain", "doctor", "--json"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "doctor on fresh workspace should be clean; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim()).expect("json");
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(payload.get("drift_count").and_then(|v| v.as_u64()), Some(0));
}

#[test]
fn chain_doctor_fix_markers_repairs_drift() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("drift_app");
    run_chain_new(&ws, "drift_app");

    // Discover the program crate directory.
    let entries: Vec<_> = std::fs::read_dir(ws.join("programs"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    let program_dir = entries[0].path();
    let mod_rs = program_dir.join("src/instructions/mod.rs");

    // Strip the markers from instructions/mod.rs to simulate drift.
    let original = std::fs::read_to_string(&mod_rs).unwrap();
    let scrubbed: String = original
        .lines()
        .filter(|l| !l.contains("sunscreen:auto-generated"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&mod_rs, &scrubbed).unwrap();

    // Report-only: should exit 6 with drift>0.
    let report = run(&ws, &["chain", "doctor", "--json"]);
    assert_eq!(report.status.code(), Some(6));

    // Fix: should repair and exit 0.
    let fix = run(&ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(0),
        "fix should succeed; stderr={}",
        String::from_utf8_lossy(&fix.stderr)
    );

    let after = std::fs::read_to_string(&mod_rs).unwrap();
    assert!(after.contains("sunscreen:auto-generated:begin segment=instructions"));
}
