//! End-to-end tests for `sunscreen chain serve` command boundaries.

use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

#[test]
fn chain_serve_help_exposes_headless_watch_flags() {
    let out = Command::new(sunscreen_bin())
        .args(["chain", "serve", "--help"])
        .output()
        .expect("invoke sunscreen chain serve help");

    assert!(
        out.status.success(),
        "chain serve --help failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--headless"), "{stdout}");
    assert!(stdout.contains("--debounce-ms"), "{stdout}");
    assert!(stdout.contains("--no-codama"), "{stdout}");
    assert!(stdout.contains("--no-frontend"), "{stdout}");
    assert!(stdout.contains("--runtime"), "{stdout}");
}

#[test]
fn chain_serve_headless_missing_workspace_exits_5() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(sunscreen_bin())
        .current_dir(tmp.path())
        .args(["--json", "chain", "serve", "--headless"])
        .output()
        .expect("invoke sunscreen chain serve");

    assert_eq!(out.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("workspace_missing"), "{stderr}");
}
