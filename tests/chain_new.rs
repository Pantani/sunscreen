//! End-to-end tests for `sunscreen chain new`.
//!
//! These exercise the public CLI surface via the library entry point
//! (`cli::root` parses argv from `std::env`, so we drive it through
//! `cargo run --bin sunscreen` for the integration path) plus the
//! internal helpers directly.

use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

#[test]
fn chain_new_dry_run_lists_planned_files() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("demo");

    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args([
            "chain",
            "new",
            "demo",
            "--framework",
            "anchor",
            "--frontend",
            "none",
            "--path",
        ])
        .arg(&out_path)
        .arg("--dry-run")
        .output()
        .expect("invoke sunscreen");

    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dry-run"));
    assert!(stdout.contains("Anchor.toml"));
    assert!(stdout.contains("Cargo.toml"));
    assert!(stdout.contains("sunscreen.yml"));
    assert!(stdout.contains("programs/demo/"));
    // Nothing was actually written.
    assert!(!out_path.exists());
}

#[test]
fn chain_new_writes_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("demo_app");

    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args([
            "chain",
            "new",
            "demo_app",
            "--framework",
            "anchor",
            "--frontend",
            "none",
            "--path",
        ])
        .arg(&out_path)
        .output()
        .expect("invoke sunscreen");

    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    // Minimum structure we expect.
    for rel in [
        "Anchor.toml",
        "Cargo.toml",
        "sunscreen.yml",
        "programs/demo_app/Cargo.toml",
        "programs/demo_app/src/lib.rs",
    ] {
        let p = out_path.join(rel);
        assert!(p.exists(), "missing file: {}", p.display());
    }

    // sunscreen.yml must parse via the loader (validates the v1 schema).
    let cfg_path = out_path.join("sunscreen.yml");
    let cfg = sunscreen::config::load(Some(&cfg_path)).expect("loader");
    assert_eq!(cfg.version, 1);
    assert!(!cfg.project.name.is_empty());
    cfg.validate().expect("generated sunscreen.yml validates");
}

#[test]
fn chain_new_refuses_invalid_name() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(["chain", "new", "1bad", "--path"])
        .arg(tmp.path())
        .arg("--dry-run")
        .output()
        .expect("invoke sunscreen");
    assert_eq!(out.status.code(), Some(4));
}
