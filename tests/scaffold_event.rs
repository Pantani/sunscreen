//! End-to-end tests for `sunscreen scaffold event`.

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
fn scaffold_event_creates_events_rs_and_appends() {
    let (_tmp, ws, program_name) = event_workspace();

    scaffold_deposited_event(&ws, &program_name);
    let events_rs = ws
        .join("programs")
        .join(&program_name)
        .join("src/events.rs");
    assert_events_file_created(&events_rs);
    assert_lib_declares_events(&ws, &program_name);

    append_withdrawn_event(&ws, &program_name, &events_rs);
    assert_deposited_rerun(&ws, &program_name);
    assert_event_conflict(&ws, &program_name);
    assert_event_dry_run(&ws, &program_name, &events_rs);
}

fn event_workspace() -> (tempfile::TempDir, std::path::PathBuf, String) {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("evt_app");
    run_chain_new(&ws, "evt_app");
    let program_name = discover_program(&ws);
    (tmp, ws, program_name)
}

fn scaffold_deposited_event(ws: &Path, program_name: &str) {
    let out = run_scaffold(
        ws,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            program_name,
            "--fields",
            "amount:u64",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "scaffold event failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn assert_events_file_created(events_rs: &Path) {
    assert!(events_rs.exists());
    let contents = std::fs::read_to_string(events_rs).unwrap();
    assert!(contents.contains("#[event]"));
    assert!(contents.contains("pub struct Deposited"));
    assert!(contents.contains("pub amount: u64"));
    assert!(contents.contains("segment=events"));
}

fn assert_lib_declares_events(ws: &Path, program_name: &str) {
    let lib_rs = ws.join("programs").join(program_name).join("src/lib.rs");
    let lib_contents = std::fs::read_to_string(&lib_rs).unwrap();
    assert!(
        lib_contents.lines().any(|l| l.trim() == "pub mod events;"),
        "expected `pub mod events;` in lib.rs, got:\n{lib_contents}"
    );
}

fn append_withdrawn_event(ws: &Path, program_name: &str, events_rs: &Path) {
    let out2 = run_scaffold(
        ws,
        &[
            "scaffold",
            "event",
            "Withdrawn",
            "--program",
            program_name,
            "--fields",
            "amount:u64,user:Pubkey",
            "--json",
        ],
    );
    assert!(out2.status.success());
    let c2 = std::fs::read_to_string(events_rs).unwrap();
    assert!(c2.contains("pub struct Deposited"));
    assert!(c2.contains("pub struct Withdrawn"));
}

fn assert_deposited_rerun(ws: &Path, program_name: &str) {
    let again = run_scaffold(
        ws,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            program_name,
            "--fields",
            "amount:u64",
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
}

fn assert_event_conflict(ws: &Path, program_name: &str) {
    let conflict = run_scaffold(
        ws,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            program_name,
            "--fields",
            "amount:u128",
        ],
    );
    assert_eq!(conflict.status.code(), Some(4));
}

fn assert_event_dry_run(ws: &Path, program_name: &str, events_rs: &Path) {
    let before = std::fs::read_to_string(events_rs).unwrap();
    let dry = run_scaffold(
        ws,
        &[
            "scaffold",
            "event",
            "Paused",
            "--program",
            program_name,
            "--dry-run",
        ],
    );
    assert!(dry.status.success());
    let after = std::fs::read_to_string(events_rs).unwrap();
    assert_eq!(before, after);
}
