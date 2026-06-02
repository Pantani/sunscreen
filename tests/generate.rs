//! End-to-end tests for `sunscreen generate`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

fn run_chain_new(out_path: &Path, name: &str, frontend: &str) {
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(["chain", "new", name, "--frontend", frontend, "--path"])
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

fn run_generate(ws: &Path, args: &[&str]) -> std::process::Output {
    Command::new(sunscreen_bin())
        .current_dir(ws)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("invoke sunscreen {args:?}: {err}"))
}

fn parse_json(stdout: &[u8]) -> serde_json::Value {
    serde_json::from_slice(stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json stdout: {err}\n{}",
            String::from_utf8_lossy(stdout)
        )
    })
}

fn sample_idl(program: &str) -> String {
    format!(
        r#"{{
  "address": "Sunscreen1111111111111111111111111111111111",
  "metadata": {{
    "name": "{program}",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "test idl"
  }},
  "instructions": [
    {{
      "name": "initializeVault",
      "accounts": [],
      "args": [
        {{ "name": "amount", "type": "u64" }},
        {{ "name": "memo", "type": "string" }}
      ]
    }},
    {{
      "name": "closeVault",
      "accounts": [],
      "args": []
    }}
  ],
  "accounts": [],
  "errors": [],
  "types": []
}}"#
    )
}

fn write_target_idl(ws: &Path, program: &str) -> PathBuf {
    let idl_dir = ws.join("target/idl");
    std::fs::create_dir_all(&idl_dir).expect("create idl dir");
    let path = idl_dir.join(format!("{program}.json"));
    std::fs::write(&path, sample_idl(program)).expect("write idl");
    path
}

fn write_target_idl_with_instruction(ws: &Path, program: &str, instruction: &str) -> PathBuf {
    let idl_dir = ws.join("target/idl");
    std::fs::create_dir_all(&idl_dir).expect("create idl dir");
    let path = idl_dir.join(format!("{program}.json"));
    std::fs::write(
        &path,
        format!(
            r#"{{
  "address": "{program}1111111111111111111111111111111111",
  "metadata": {{
    "name": "{program}",
    "version": "0.1.0",
    "spec": "0.1.0"
  }},
  "instructions": [
    {{
      "name": "{instruction}",
      "accounts": [],
      "args": []
    }}
  ],
  "accounts": [],
  "errors": [],
  "types": []
}}"#
        ),
    )
    .expect("write idl");
    path
}

fn prepend_path(bin_dir: &Path) -> std::ffi::OsString {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("join PATH")
}

#[cfg(unix)]
fn write_fake_pnpm(bin_dir: &Path, exit_code: i32) {
    use std::os::unix::fs::PermissionsExt;

    let path = bin_dir.join("pnpm");
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$PWD" > "$PNPM_CWD_FILE"
printf '%s\n' "$*" > "$PNPM_ARGS_FILE"
echo "fake codama $*"
exit {exit_code}
"#
        ),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
}

#[cfg(windows)]
fn write_fake_pnpm(bin_dir: &Path, exit_code: i32) {
    let path = bin_dir.join("pnpm.bat");
    std::fs::write(
        &path,
        format!(
            r#"@echo off
cd > "%PNPM_CWD_FILE%"
echo %* > "%PNPM_ARGS_FILE%"
echo fake codama %*
exit /b {exit_code}
"#
        ),
    )
    .unwrap();
}

#[test]
fn generate_idl_exports_anchor_idl_and_reports_idempotence() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("idl_app");
    run_chain_new(&ws, "idl_app", "none");
    write_target_idl(&ws, "idl_app");

    let first = run_generate(&ws, &["--json", "generate", "idl"]);
    assert!(
        first.status.success(),
        "generate idl failed: code={:?}\nstdout={}\nstderr={}",
        first.status.code(),
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let first_payload = parse_json(&first.stdout);
    assert_eq!(
        first_payload.get("ok").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        first_payload
            .get("changed_files")
            .and_then(|v| v.as_array())
            .unwrap()
            .len(),
        1
    );
    assert!(ws.join("clients/idl/idl_app.json").exists());

    let second = run_generate(&ws, &["--json", "generate", "idl"]);
    assert!(second.status.success());
    let second_payload = parse_json(&second.stdout);
    assert_eq!(
        second_payload
            .get("changed_files")
            .and_then(|v| v.as_array())
            .unwrap()
            .len(),
        0,
        "second run should be byte-for-byte idempotent"
    );
}

#[test]
fn generate_clients_writes_codama_config_and_invokes_codama_all() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("clients_app");
    run_chain_new(&ws, "clients_app", "none");
    write_target_idl(&ws, "clients_app");

    let fake_bin = tmp.path().join("bin");
    std::fs::create_dir_all(&fake_bin).unwrap();
    write_fake_pnpm(&fake_bin, 0);
    let cwd_file = tmp.path().join("pnpm.cwd");
    let args_file = tmp.path().join("pnpm.args");

    let out = Command::new(sunscreen_bin())
        .current_dir(&ws)
        .env("PATH", prepend_path(&fake_bin))
        .env("PNPM_CWD_FILE", &cwd_file)
        .env("PNPM_ARGS_FILE", &args_file)
        .args(["--json", "generate", "clients"])
        .output()
        .expect("invoke generate clients");

    assert!(
        out.status.success(),
        "generate clients failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        PathBuf::from(std::fs::read_to_string(cwd_file).unwrap().trim())
            .canonicalize()
            .unwrap(),
        ws.canonicalize().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(args_file).unwrap().trim(),
        "exec codama run --all --config codama.json"
    );

    let cfg: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(ws.join("codama.json")).unwrap()).unwrap();
    assert_eq!(
        cfg.get("idl").and_then(|v| v.as_str()),
        Some("target/idl/clients_app.json")
    );
    assert_eq!(
        cfg.pointer("/scripts/js/from").and_then(|v| v.as_str()),
        Some("@codama/renderers-js")
    );

    let payload = parse_json(&out.stdout);
    assert_eq!(payload.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        payload
            .get("codama_config_changed")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(payload.get("exit_code").and_then(|v| v.as_i64()), Some(0));
}

#[test]
fn generate_idl_refuses_output_directory_that_escapes_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("escape_app");
    run_chain_new(&ws, "escape_app", "none");
    write_target_idl(&ws, "escape_app");

    let out = run_generate(
        &ws,
        &["--json", "generate", "idl", "--out-dir", "../outside"],
    );

    assert_eq!(out.status.code(), Some(4));
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("json error payload");
    assert_eq!(
        payload.get("kind").and_then(|v| v.as_str()),
        Some("user_input")
    );
    assert!(!tmp.path().join("outside").exists());
}

#[test]
fn generate_frontend_hooks_defaults_to_react_hooks_for_next() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("hooks_app");
    run_chain_new(&ws, "hooks_app", "next");
    write_target_idl(&ws, "hooks_app");
    let idl = run_generate(&ws, &["--json", "generate", "idl"]);
    assert!(idl.status.success());

    let first = run_generate(&ws, &["--json", "generate", "frontend-hooks"]);
    assert!(
        first.status.success(),
        "generate frontend-hooks failed: code={:?}\nstdout={}\nstderr={}",
        first.status.code(),
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let generated_root = ws.join("app/src/generated/sunscreen");
    for rel in ["idl.ts", "core.ts", "react.ts", "index.ts"] {
        assert!(generated_root.join(rel).exists(), "missing generated {rel}");
    }
    assert!(
        !generated_root.join("solid.ts").exists(),
        "React frontend default should not generate Solid hooks"
    );

    let react = std::fs::read_to_string(generated_root.join("react.ts")).unwrap();
    assert!(react.contains("@tanstack/react-query"));
    assert!(react.contains("useInitializeVaultMutation"));
    assert!(react.contains("useCloseVaultMutation"));
    assert!(react.contains("useProgramAccountsQuery"));

    let core = std::fs::read_to_string(generated_root.join("core.ts")).unwrap();
    assert!(core.contains("createSurfpoolRpc"));
    assert!(core.contains("http://127.0.0.1:8899"));

    let before = read_tree(&generated_root);
    let second = run_generate(&ws, &["--json", "generate", "frontend-hooks"]);
    assert!(second.status.success());
    let second_payload = parse_json(&second.stdout);
    assert_eq!(
        second_payload
            .get("changed_files")
            .and_then(|v| v.as_array())
            .unwrap()
            .len(),
        0
    );
    assert_eq!(before, read_tree(&generated_root));
}

#[test]
fn generate_frontend_hooks_target_all_emits_react_and_solid_hooks() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("hooks_all_app");
    run_chain_new(&ws, "hooks_all_app", "next");
    write_target_idl(&ws, "hooks_all_app");
    assert!(run_generate(&ws, &["--json", "generate", "idl"])
        .status
        .success());

    let out = run_generate(
        &ws,
        &["--json", "generate", "frontend-hooks", "--target", "all"],
    );

    assert!(
        out.status.success(),
        "generate frontend-hooks failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let generated_root = ws.join("app/src/generated/sunscreen");
    let react = std::fs::read_to_string(generated_root.join("react.ts")).unwrap();
    let solid = std::fs::read_to_string(generated_root.join("solid.ts")).unwrap();
    assert!(react.contains("useInitializeVaultMutation"));
    assert!(solid.contains("@tanstack/solid-query"));
    assert!(solid.contains("createInitializeVaultMutation"));
    assert!(solid.contains("createProgramAccountsQuery"));
}

#[test]
fn generate_frontend_hooks_program_ignores_stale_exported_idls() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("program_filter_app");
    run_chain_new(&ws, "program_filter_app", "next");
    write_target_idl(&ws, "program_filter_app");

    let stale_dir = ws.join("clients/idl");
    std::fs::create_dir_all(&stale_dir).unwrap();
    std::fs::write(
        stale_dir.join("stale_app.json"),
        sample_idl("stale_app").replace("initializeVault", "staleInstruction"),
    )
    .unwrap();

    let out = run_generate(
        &ws,
        &[
            "--json",
            "generate",
            "frontend-hooks",
            "--program",
            "program_filter_app",
        ],
    );

    assert!(
        out.status.success(),
        "generate frontend-hooks failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let generated_root = ws.join("app/src/generated/sunscreen");
    let idl = std::fs::read_to_string(generated_root.join("idl.ts")).unwrap();
    let react = std::fs::read_to_string(generated_root.join("react.ts")).unwrap();
    assert!(idl.contains("program_filter_app"));
    assert!(!idl.contains("stale_app"));
    assert!(!react.contains("useStaleInstructionMutation"));
}

#[test]
fn generate_frontend_hooks_namespaces_duplicate_instruction_names_across_programs() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("multi_hooks_app");
    run_chain_new(&ws, "multi_hooks_app", "next");
    write_target_idl_with_instruction(&ws, "alpha_app", "initialize");
    write_target_idl_with_instruction(&ws, "beta_app", "initialize");

    let idl = run_generate(&ws, &["--json", "generate", "idl"]);
    assert!(idl.status.success());
    let hooks = run_generate(
        &ws,
        &["--json", "generate", "frontend-hooks", "--target", "all"],
    );
    assert!(
        hooks.status.success(),
        "generate frontend-hooks failed: code={:?}\nstdout={}\nstderr={}",
        hooks.status.code(),
        String::from_utf8_lossy(&hooks.stdout),
        String::from_utf8_lossy(&hooks.stderr)
    );

    let generated_root = ws.join("app/src/generated/sunscreen");
    let react = std::fs::read_to_string(generated_root.join("react.ts")).unwrap();
    let solid = std::fs::read_to_string(generated_root.join("solid.ts")).unwrap();
    assert!(react.contains("useAlphaAppInitializeMutation"));
    assert!(react.contains("useBetaAppInitializeMutation"));
    assert!(solid.contains("createAlphaAppInitializeMutation"));
    assert!(solid.contains("createBetaAppInitializeMutation"));
    assert!(!react.contains("export type InitializeInput"));
    assert!(!solid.contains("export type InitializeInput"));
}

#[test]
fn generate_frontend_hooks_refuses_frontend_none_without_explicit_path() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("no_frontend_app");
    run_chain_new(&ws, "no_frontend_app", "none");
    write_target_idl(&ws, "no_frontend_app");
    let idl = run_generate(&ws, &["--json", "generate", "idl"]);
    assert!(idl.status.success());

    let out = run_generate(&ws, &["--json", "generate", "frontend-hooks"]);

    assert_eq!(out.status.code(), Some(4));
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("json error payload");
    assert_eq!(
        payload.get("kind").and_then(|v| v.as_str()),
        Some("user_input")
    );
    assert!(payload
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap()
        .contains("--frontend-path"));
}

#[test]
#[ignore = "requires pnpm install for a generated Next.js app"]
fn generated_frontend_hooks_typecheck_vanilla_next_project_when_dependencies_are_installed() {
    if std::env::var_os("SUNSCREEN_FRONTEND_COMPILE_TESTS").is_none() {
        eprintln!(
            "skipping frontend typecheck: set SUNSCREEN_FRONTEND_COMPILE_TESTS=1 and run with --ignored"
        );
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("next_typecheck_app");
    run_chain_new(&ws, "next_typecheck_app", "next");
    write_target_idl(&ws, "next_typecheck_app");
    assert!(run_generate(&ws, &["--json", "generate", "idl"])
        .status
        .success());
    assert!(run_generate(&ws, &["--json", "generate", "frontend-hooks"])
        .status
        .success());

    let install = Command::new("pnpm")
        .current_dir(ws.join("app"))
        .args(["install", "--frozen-lockfile=false"])
        .output()
        .expect("pnpm install");
    assert!(
        install.status.success(),
        "pnpm install failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );

    let check = Command::new("pnpm")
        .current_dir(ws.join("app"))
        .args(["exec", "tsc", "--noEmit"])
        .output()
        .expect("pnpm exec tsc");
    assert!(
        check.status.success(),
        "frontend typecheck failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

fn read_tree(root: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    read_tree_into(root, root, &mut files);
    files
}

fn read_tree_into(root: &Path, current: &Path, files: &mut BTreeMap<String, String>) {
    for entry in std::fs::read_dir(current).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            read_tree_into(root, &path, files);
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(rel, std::fs::read_to_string(path).unwrap());
        }
    }
}
