//! End-to-end tests for `sunscreen chain build`.

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
        "chain new failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run_chain_new_pinocchio(out_path: &Path, name: &str) {
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args([
            "chain",
            "new",
            name,
            "--framework",
            "pinocchio",
            "--frontend",
            "none",
            "--path",
        ])
        .arg(out_path)
        .output()
        .expect("invoke sunscreen chain new");
    assert!(
        out.status.success(),
        "chain new failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn parse_ndjson(stdout: &[u8]) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("invalid json line {line:?}: {e}"))
        })
        .collect()
}

fn prepend_path(bin_dir: &Path) -> std::ffi::OsString {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("join PATH")
}

#[cfg(unix)]
fn write_fake_anchor(bin_dir: &Path, exit_code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = bin_dir.join("anchor");
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$PWD" > "$ANCHOR_CWD_FILE"
printf '%s\n' "$*" > "$ANCHOR_ARGS_FILE"
mkdir -p target/idl
printf '{{"metadata":{{"name":"fake"}}}}\n' > target/idl/fake.json
echo "fake anchor $*"
echo "fake anchor stderr" >&2
exit {exit_code}
"#
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn write_fake_pnpm(bin_dir: &Path, exit_code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = bin_dir.join("pnpm");
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$PWD" > "$PNPM_CWD_FILE"
printf '%s\n' "$*" > "$PNPM_ARGS_FILE"
echo "fake pnpm $*"
exit {exit_code}
"#
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn write_fake_cargo(bin_dir: &Path, exit_code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = bin_dir.join("cargo");
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$PWD" > "$CARGO_CWD_FILE"
printf '%s\n' "$*" > "$CARGO_ARGS_FILE"
echo "fake cargo $*"
exit {exit_code}
"#
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(windows)]
fn write_fake_anchor(bin_dir: &Path, exit_code: i32) -> PathBuf {
    let path = bin_dir.join("anchor.bat");
    std::fs::write(
        &path,
        format!(
            r#"@echo off
cd > "%ANCHOR_CWD_FILE%"
echo %* > "%ANCHOR_ARGS_FILE%"
mkdir target\idl 2>NUL
echo {{"metadata":{{"name":"fake"}}}} > target\idl\fake.json
echo fake anchor %*
echo fake anchor stderr 1>&2
exit /b {exit_code}
"#
        ),
    )
    .unwrap();
    path
}

#[cfg(windows)]
fn write_fake_pnpm(bin_dir: &Path, exit_code: i32) -> PathBuf {
    let path = bin_dir.join("pnpm.bat");
    std::fs::write(
        &path,
        format!(
            r#"@echo off
cd > "%PNPM_CWD_FILE%"
echo %* > "%PNPM_ARGS_FILE%"
echo fake pnpm %*
exit /b {exit_code}
"#
        ),
    )
    .unwrap();
    path
}

#[cfg(windows)]
fn write_fake_cargo(bin_dir: &Path, exit_code: i32) -> PathBuf {
    let path = bin_dir.join("cargo.bat");
    std::fs::write(
        &path,
        format!(
            r#"@echo off
cd > "%CARGO_CWD_FILE%"
echo %* > "%CARGO_ARGS_FILE%"
echo fake cargo %*
exit /b {exit_code}
"#
        ),
    )
    .unwrap();
    path
}

#[test]
fn chain_build_headless_runs_anchor_then_codama_in_workspace_root_and_emits_ndjson() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("build_app");
    run_chain_new(&ws, "build_app");

    let fake_bin = tmp.path().join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    write_fake_anchor(&fake_bin, 0);
    write_fake_pnpm(&fake_bin, 0);
    let cwd_file = tmp.path().join("anchor.cwd");
    let args_file = tmp.path().join("anchor.args");
    let pnpm_cwd_file = tmp.path().join("pnpm.cwd");
    let pnpm_args_file = tmp.path().join("pnpm.args");
    let nested_dir = ws.join("programs/build_app/src");

    let out = Command::new(sunscreen_bin())
        .current_dir(&nested_dir)
        .env("PATH", prepend_path(&fake_bin))
        .env("ANCHOR_CWD_FILE", &cwd_file)
        .env("ANCHOR_ARGS_FILE", &args_file)
        .env("PNPM_CWD_FILE", &pnpm_cwd_file)
        .env("PNPM_ARGS_FILE", &pnpm_args_file)
        .args(["--json", "chain", "build", "--headless"])
        .output()
        .expect("invoke sunscreen chain build");

    assert!(
        out.status.success(),
        "chain build failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let recorded_cwd = PathBuf::from(std::fs::read_to_string(&cwd_file).unwrap().trim());
    assert_eq!(
        recorded_cwd.canonicalize().unwrap(),
        ws.canonicalize().unwrap()
    );
    assert_eq!(std::fs::read_to_string(&args_file).unwrap().trim(), "build");
    let recorded_pnpm_cwd = PathBuf::from(std::fs::read_to_string(&pnpm_cwd_file).unwrap().trim());
    assert_eq!(
        recorded_pnpm_cwd.canonicalize().unwrap(),
        ws.canonicalize().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(&pnpm_args_file).unwrap().trim(),
        "exec codama run --all --config codama.json"
    );
    assert!(ws.join("codama.json").exists());

    let events = parse_ndjson(&out.stdout);
    let names: Vec<_> = events
        .iter()
        .map(|event| event.get("event").and_then(|v| v.as_str()).unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "chain_build_started",
            "command_started",
            "command_finished",
            "command_started",
            "command_finished",
            "chain_build_finished"
        ]
    );
    let steps: Vec<_> = events
        .iter()
        .filter_map(|event| event.get("step").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(
        steps,
        ["anchor_build", "anchor_build", "codama_run", "codama_run"]
    );
    assert_eq!(
        events
            .last()
            .unwrap()
            .get("status")
            .and_then(|v| v.as_str()),
        Some("ok")
    );
}

#[test]
fn chain_build_pinocchio_runs_cargo_build_sbf_and_skips_codama() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("pinocchio_build_app");
    run_chain_new_pinocchio(&ws, "pinocchio_build_app");

    let fake_bin = tmp.path().join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    write_fake_cargo(&fake_bin, 0);
    let cargo_cwd_file = tmp.path().join("cargo.cwd");
    let cargo_args_file = tmp.path().join("cargo.args");

    let out = Command::new(sunscreen_bin())
        .current_dir(&ws)
        .env("PATH", prepend_path(&fake_bin))
        .env("CARGO_CWD_FILE", &cargo_cwd_file)
        .env("CARGO_ARGS_FILE", &cargo_args_file)
        .args(["--json", "chain", "build", "--headless"])
        .output()
        .expect("invoke sunscreen chain build");

    assert!(
        out.status.success(),
        "chain build failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let recorded_cwd = PathBuf::from(std::fs::read_to_string(&cargo_cwd_file).unwrap().trim());
    assert_eq!(
        recorded_cwd.canonicalize().unwrap(),
        ws.canonicalize().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(&cargo_args_file).unwrap().trim(),
        "build-sbf"
    );

    let events = parse_ndjson(&out.stdout);
    assert_eq!(events[0]["framework"], "pinocchio");
    assert_eq!(events[0]["codama"], false);
    let steps: Vec<_> = events
        .iter()
        .filter_map(|event| event.get("step").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(steps, ["pinocchio_build", "pinocchio_build"]);
    assert!(
        !ws.join("codama.json").exists(),
        "Pinocchio build should not write Codama config"
    );
}

#[test]
fn chain_build_missing_workspace_exits_5() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(sunscreen_bin())
        .current_dir(tmp.path())
        .args(["--json", "chain", "build", "--headless"])
        .output()
        .expect("invoke sunscreen chain build");

    assert_eq!(out.status.code(), Some(5));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let payload: serde_json::Value = serde_json::from_str(stderr.trim()).expect("json stderr");
    assert_eq!(
        payload.get("kind").and_then(|v| v.as_str()),
        Some("workspace_missing")
    );
}

#[test]
fn chain_build_no_codama_skips_pnpm() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("no_codama_app");
    run_chain_new(&ws, "no_codama_app");

    let fake_bin = tmp.path().join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    write_fake_anchor(&fake_bin, 0);
    let cwd_file = tmp.path().join("anchor.cwd");
    let args_file = tmp.path().join("anchor.args");

    let out = Command::new(sunscreen_bin())
        .current_dir(&ws)
        .env("PATH", prepend_path(&fake_bin))
        .env("ANCHOR_CWD_FILE", &cwd_file)
        .env("ANCHOR_ARGS_FILE", &args_file)
        .args(["--json", "chain", "build", "--headless", "--no-codama"])
        .output()
        .expect("invoke sunscreen chain build");

    assert!(
        out.status.success(),
        "chain build failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let events = parse_ndjson(&out.stdout);
    let steps: Vec<_> = events
        .iter()
        .filter_map(|event| event.get("step").and_then(|v| v.as_str()))
        .collect();
    assert_eq!(steps, ["anchor_build", "anchor_build"]);
}

#[test]
fn chain_build_missing_pnpm_exits_2_after_successful_anchor_build() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("missing_pnpm_app");
    run_chain_new(&ws, "missing_pnpm_app");

    let fake_bin = tmp.path().join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    write_fake_anchor(&fake_bin, 0);
    let cwd_file = tmp.path().join("anchor.cwd");
    let args_file = tmp.path().join("anchor.args");

    let out = Command::new(sunscreen_bin())
        .current_dir(&ws)
        .env("PATH", prepend_path(&fake_bin))
        .env("ANCHOR_CWD_FILE", &cwd_file)
        .env("ANCHOR_ARGS_FILE", &args_file)
        .args(["--json", "chain", "build", "--headless"])
        .output()
        .expect("invoke sunscreen chain build");

    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let payload: serde_json::Value = serde_json::from_str(stderr.trim()).expect("json stderr");
    assert_eq!(
        payload.get("kind").and_then(|v| v.as_str()),
        Some("toolchain_missing")
    );
}

#[test]
fn chain_build_missing_anchor_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("missing_anchor_app");
    run_chain_new(&ws, "missing_anchor_app");
    let empty_path = tmp.path().join("empty-bin");
    std::fs::create_dir_all(&empty_path).unwrap();

    let out = Command::new(sunscreen_bin())
        .current_dir(&ws)
        .env("PATH", &empty_path)
        .args(["--json", "chain", "build", "--headless"])
        .output()
        .expect("invoke sunscreen chain build");

    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let payload: serde_json::Value = serde_json::from_str(stderr.trim()).expect("json stderr");
    assert_eq!(
        payload.get("kind").and_then(|v| v.as_str()),
        Some("toolchain_missing")
    );
}

#[test]
fn chain_build_returns_anchor_exit_code_on_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("failing_anchor_app");
    run_chain_new(&ws, "failing_anchor_app");

    let fake_bin = tmp.path().join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    write_fake_anchor(&fake_bin, 17);
    let cwd_file = tmp.path().join("anchor.cwd");
    let args_file = tmp.path().join("anchor.args");

    let out = Command::new(sunscreen_bin())
        .current_dir(&ws)
        .env("PATH", prepend_path(&fake_bin))
        .env("ANCHOR_CWD_FILE", &cwd_file)
        .env("ANCHOR_ARGS_FILE", &args_file)
        .args(["--json", "chain", "build", "--headless"])
        .output()
        .expect("invoke sunscreen chain build");

    assert_eq!(out.status.code(), Some(17));
    let events = parse_ndjson(&out.stdout);
    assert_eq!(
        events
            .last()
            .unwrap()
            .get("status")
            .and_then(|v| v.as_str()),
        Some("failed")
    );
    assert_eq!(
        events
            .last()
            .unwrap()
            .get("exit_code")
            .and_then(|v| v.as_i64()),
        Some(17)
    );
}
