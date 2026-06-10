//! End-to-end tests for `sunscreen scaffold program`.

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

fn run(ws: &Path, args: &[&str]) -> std::process::Output {
    Command::new(sunscreen_bin())
        .current_dir(ws)
        .args(args)
        .output()
        .expect("invoke sunscreen")
}

fn strip_auto_segment(source: &str, segment: &str) -> String {
    let line_ending = detect_line_ending(source);
    let begin = format!("sunscreen:auto-generated:begin segment={segment}");
    let end = format!("sunscreen:auto-generated:end segment={segment}");
    let mut out = Vec::new();
    let mut skipping = false;
    for line in source.lines() {
        if line.contains(&begin) {
            skipping = true;
            continue;
        }
        if skipping && line.contains(&end) {
            skipping = false;
            continue;
        }
        if !skipping {
            out.push(line);
        }
    }
    let mut joined = out.join(line_ending);
    joined.push_str(line_ending);
    joined
}

fn remove_auto_marker_lines(source: &str, segment: &str) -> String {
    let line_ending = detect_line_ending(source);
    let begin = format!("sunscreen:auto-generated:begin segment={segment}");
    let end = format!("sunscreen:auto-generated:end segment={segment}");
    let mut joined = source
        .lines()
        .filter(|line| !line.contains(&begin) && !line.contains(&end))
        .collect::<Vec<_>>()
        .join(line_ending);
    joined.push_str(line_ending);
    joined
}

fn detect_line_ending(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

#[test]
fn strip_auto_segment_preserves_crlf_line_endings() {
    let source = concat!(
        "alpha\r\n",
        "// sunscreen:auto-generated:begin segment=dispatch version=1\r\n",
        "generated\r\n",
        "// sunscreen:auto-generated:end segment=dispatch\r\n",
        "omega\r\n",
    );

    assert_eq!(strip_auto_segment(source, "dispatch"), "alpha\r\nomega\r\n");
}

#[test]
fn remove_auto_marker_lines_preserves_crlf_line_endings() {
    let source = concat!(
        "alpha\r\n",
        "// sunscreen:auto-generated:begin segment=dispatch version=1\r\n",
        "generated\r\n",
        "// sunscreen:auto-generated:end segment=dispatch\r\n",
        "omega\r\n",
    );

    assert_eq!(
        remove_auto_marker_lines(source, "dispatch"),
        "alpha\r\ngenerated\r\nomega\r\n"
    );
}

#[test]
fn scaffold_program_creates_crate_and_patches_manifests() {
    let (_tmp, ws) = chain_workspace("multi_app");

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

    assert_extra_program_files(&ws);
    assert_extra_program_anchor_toml(&ws);
    assert_extra_program_manifest(&ws);
}

fn chain_workspace(name: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join(name);
    run_chain_new(&ws, name);
    (tmp, ws)
}

fn assert_extra_program_files(ws: &Path) {
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
}

fn assert_extra_program_anchor_toml(ws: &Path) {
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
}

fn assert_extra_program_manifest(ws: &Path) {
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

#[test]
fn chain_doctor_fix_markers_rebuilds_dispatch_inside_program_module() {
    let (_tmp, ws) = chain_workspace("dispatch_repair_app");
    let (program_name, program_dir) = single_program(&ws);
    scaffold_deposit_instruction(&ws, &program_name);

    let lib_rs = program_dir.join("src/lib.rs");
    remove_dispatch_segment(&lib_rs);
    assert_doctor_fix_succeeds(&ws, "dispatch repair should succeed");
    assert_dispatch_repaired(&lib_rs);
}

fn single_program(ws: &Path) -> (String, PathBuf) {
    let entries: Vec<_> = std::fs::read_dir(ws.join("programs"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    (
        entries[0].file_name().to_string_lossy().into_owned(),
        entries[0].path(),
    )
}

fn scaffold_deposit_instruction(ws: &Path, program_name: &str) {
    let scaffold = run(
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
    assert!(
        scaffold.status.success(),
        "scaffold instruction failed: stderr={}",
        String::from_utf8_lossy(&scaffold.stderr)
    );
}

fn remove_dispatch_segment(lib_rs: &Path) {
    let original = std::fs::read_to_string(lib_rs).unwrap();
    assert!(original.contains("sunscreen:auto-generated:begin segment=dispatch"));
    assert!(original.contains("pub fn deposit(ctx: Context<Deposit>, amount: u64)"));

    let scrubbed = strip_auto_segment(&original, "dispatch");
    assert!(!scrubbed.contains("sunscreen:auto-generated:begin segment=dispatch"));
    assert!(!scrubbed.contains("pub fn deposit(ctx: Context<Deposit>, amount: u64)"));
    std::fs::write(lib_rs, scrubbed).unwrap();
}

fn assert_doctor_fix_succeeds(ws: &Path, message: &str) {
    let report = run(ws, &["chain", "doctor", "--json"]);
    assert_eq!(report.status.code(), Some(6));

    let fix = run(ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(0),
        "{message}; stdout={} stderr={}",
        String::from_utf8_lossy(&fix.stdout),
        String::from_utf8_lossy(&fix.stderr)
    );
}

fn assert_dispatch_repaired(lib_rs: &Path) {
    let after = std::fs::read_to_string(lib_rs).unwrap();
    assert!(after.contains("sunscreen:auto-generated:begin segment=dispatch"));
    assert!(after.contains("pub fn deposit(ctx: Context<Deposit>, amount: u64)"));
    assert!(after.contains("instructions::deposit::handler(ctx, amount)"));
}

#[test]
fn chain_doctor_fix_markers_preserves_no_arg_handlers_and_skips_helper_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("dispatch_helper_app");
    run_chain_new(&ws, "dispatch_helper_app");

    let entries: Vec<_> = std::fs::read_dir(ws.join("programs"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    let program_name = entries[0].file_name().to_string_lossy().into_owned();
    let program_dir = entries[0].path();

    let scaffold = run(
        &ws,
        &[
            "scaffold",
            "instruction",
            "ping",
            "--program",
            &program_name,
            "--json",
        ],
    );
    assert!(
        scaffold.status.success(),
        "scaffold instruction failed: stderr={}",
        String::from_utf8_lossy(&scaffold.stderr)
    );

    let helper_rs = program_dir.join("src/instructions/helpers.rs");
    std::fs::write(
        helper_rs,
        "use super::*;\n\npub fn normalize_amount(amount: u64) -> u64 {\n    amount\n}\n",
    )
    .unwrap();

    let lib_rs = program_dir.join("src/lib.rs");
    let original = std::fs::read_to_string(&lib_rs).unwrap();
    let scrubbed = strip_auto_segment(&original, "dispatch");
    std::fs::write(&lib_rs, scrubbed).unwrap();

    let fix = run(&ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(0),
        "dispatch repair should succeed; stdout={} stderr={}",
        String::from_utf8_lossy(&fix.stdout),
        String::from_utf8_lossy(&fix.stderr)
    );

    let after = std::fs::read_to_string(&lib_rs).unwrap();
    assert!(after.contains("pub fn ping(ctx: Context<Ping>)"));
    assert!(after.contains("instructions::ping::handler(ctx)"));
    assert!(!after.contains("pub fn helpers("));
    assert!(!after.contains("instructions::helpers::handler"));
}

#[test]
fn chain_doctor_fix_markers_refuses_dispatch_when_wrappers_remain_without_markers() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("dispatch_duplicate_app");
    run_chain_new(&ws, "dispatch_duplicate_app");

    let entries: Vec<_> = std::fs::read_dir(ws.join("programs"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    let program_name = entries[0].file_name().to_string_lossy().into_owned();
    let program_dir = entries[0].path();

    let scaffold = run(
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
        scaffold.status.success(),
        "scaffold instruction failed: stderr={}",
        String::from_utf8_lossy(&scaffold.stderr)
    );

    let lib_rs = program_dir.join("src/lib.rs");
    let original = std::fs::read_to_string(&lib_rs).unwrap();
    let scrubbed = remove_auto_marker_lines(&original, "dispatch");
    assert!(scrubbed.contains("pub fn deposit(ctx: Context<Deposit>, amount: u64)"));
    std::fs::write(&lib_rs, &scrubbed).unwrap();

    let fix = run(&ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(6),
        "dispatch repair should refuse ambiguous existing wrappers; stdout={} stderr={}",
        String::from_utf8_lossy(&fix.stdout),
        String::from_utf8_lossy(&fix.stderr)
    );

    let after = std::fs::read_to_string(&lib_rs).unwrap();
    assert_eq!(after.matches("pub fn deposit(").count(), 1);
    assert_eq!(after, scrubbed);
}

#[test]
fn chain_doctor_fix_markers_repairs_empty_error_variants_enum() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("empty_error_repair_app");
    run_chain_new(&ws, "empty_error_repair_app");

    let entries: Vec<_> = std::fs::read_dir(ws.join("programs"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    let program_name = entries[0].file_name().to_string_lossy().into_owned();
    let program_dir = entries[0].path();

    let scaffold = run(
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
    assert!(
        scaffold.status.success(),
        "scaffold error failed: stderr={}",
        String::from_utf8_lossy(&scaffold.stderr)
    );

    let errors_rs = program_dir.join("src/errors.rs");
    let original = std::fs::read_to_string(errors_rs.as_path()).unwrap();
    let scrubbed = strip_auto_segment(&original, "error_variants");
    assert!(!scrubbed.contains("Unauthorized,"));
    std::fs::write(&errors_rs, scrubbed).unwrap();

    let fix = run(&ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(0),
        "empty error enum repair should succeed; stdout={} stderr={}",
        String::from_utf8_lossy(&fix.stdout),
        String::from_utf8_lossy(&fix.stderr)
    );

    let after = std::fs::read_to_string(errors_rs).unwrap();
    assert!(after.contains("sunscreen:auto-generated:begin segment=error_variants"));
    assert!(after.contains("sunscreen:auto-generated:end segment=error_variants"));
}

#[test]
fn chain_doctor_fix_markers_refuses_ambiguous_error_variants_body() {
    let (_tmp, ws) = chain_workspace("error_repair_app");
    let (program_name, program_dir) = single_program(&ws);
    scaffold_unauthorized_error(&ws, &program_name);

    let errors_rs = program_dir.join("src/errors.rs");
    let scrubbed = remove_error_variant_markers(&errors_rs);
    assert_ambiguous_error_repair_stays_unresolved(&ws, &errors_rs, &scrubbed);
}

fn scaffold_unauthorized_error(ws: &Path, program_name: &str) {
    let scaffold = run(
        ws,
        &[
            "scaffold",
            "error",
            "Unauthorized",
            "--program",
            program_name,
            "--msg",
            "caller is not authorized } today",
            "--json",
        ],
    );
    assert!(
        scaffold.status.success(),
        "scaffold error failed: stderr={}",
        String::from_utf8_lossy(&scaffold.stderr)
    );
}

fn remove_error_variant_markers(errors_rs: &Path) -> String {
    let original = std::fs::read_to_string(errors_rs).unwrap();
    assert!(original.contains("sunscreen:auto-generated:begin segment=error_variants"));
    assert!(original.contains("Unauthorized,"));

    let scrubbed = remove_auto_marker_lines(&original, "error_variants");
    assert!(!scrubbed.contains("sunscreen:auto-generated:begin segment=error_variants"));
    assert!(scrubbed.contains("Unauthorized,"));
    std::fs::write(errors_rs, &scrubbed).unwrap();
    scrubbed
}

fn assert_ambiguous_error_repair_stays_unresolved(ws: &Path, errors_rs: &Path, scrubbed: &str) {
    let report = run(ws, &["chain", "doctor", "--json"]);
    assert_eq!(report.status.code(), Some(6));

    let fix = run(ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(6),
        "ambiguous error variant repair should remain unresolved; stdout={} stderr={}",
        String::from_utf8_lossy(&fix.stdout),
        String::from_utf8_lossy(&fix.stderr)
    );

    let after = std::fs::read_to_string(errors_rs).unwrap();
    assert_eq!(after, scrubbed);
    assert!(!after.contains("sunscreen:auto-generated:begin segment=error_variants"));
    assert!(after.contains("Unauthorized,"));
}

#[test]
fn chain_doctor_fix_markers_refuses_single_line_error_enum() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("single_line_error_app");
    run_chain_new(&ws, "single_line_error_app");

    let entries: Vec<_> = std::fs::read_dir(ws.join("programs"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    let program_dir = entries[0].path();

    let errors_rs = program_dir.join("src/errors.rs");
    let source = "use anchor_lang::prelude::*;\n\n#[error_code]\npub enum SingleLineError {}\n";
    std::fs::write(&errors_rs, source).unwrap();

    let fix = run(&ws, &["chain", "doctor", "--fix-markers", "--json"]);
    assert_eq!(
        fix.status.code(),
        Some(6),
        "single-line enum repair should remain unresolved; stdout={} stderr={}",
        String::from_utf8_lossy(&fix.stdout),
        String::from_utf8_lossy(&fix.stderr)
    );

    let after = std::fs::read_to_string(&errors_rs).unwrap();
    assert_eq!(after, source);
}
