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
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("evt_app");
    run_chain_new(&ws, "evt_app");
    let program_name = discover_program(&ws);

    // 1st event creates the file.
    let out = run_scaffold(
        &ws,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            &program_name,
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

    let events_rs = ws
        .join("programs")
        .join(&program_name)
        .join("src/events.rs");
    assert!(events_rs.exists());
    let contents = std::fs::read_to_string(&events_rs).unwrap();
    assert!(contents.contains("#[event]"));
    assert!(contents.contains("pub struct Deposited"));
    assert!(contents.contains("pub amount: u64"));
    assert!(contents.contains("segment=events"));

    // 2nd event appends.
    let out2 = run_scaffold(
        &ws,
        &[
            "scaffold",
            "event",
            "Withdrawn",
            "--program",
            &program_name,
            "--fields",
            "amount:u64,user:Pubkey",
            "--json",
        ],
    );
    assert!(out2.status.success());
    let c2 = std::fs::read_to_string(&events_rs).unwrap();
    assert!(c2.contains("pub struct Deposited"));
    assert!(c2.contains("pub struct Withdrawn"));

    // 3rd: re-add Deposited → no-op.
    let again = run_scaffold(
        &ws,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            &program_name,
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

    // Different fields for existing name → error.
    let conflict = run_scaffold(
        &ws,
        &[
            "scaffold",
            "event",
            "Deposited",
            "--program",
            &program_name,
            "--fields",
            "amount:u128",
        ],
    );
    assert_eq!(conflict.status.code(), Some(4));

    // Dry-run for a new event leaves disk untouched.
    let before = std::fs::read_to_string(&events_rs).unwrap();
    let dry = run_scaffold(
        &ws,
        &[
            "scaffold",
            "event",
            "Paused",
            "--program",
            &program_name,
            "--dry-run",
        ],
    );
    assert!(dry.status.success());
    let after = std::fs::read_to_string(&events_rs).unwrap();
    assert_eq!(before, after);
}
