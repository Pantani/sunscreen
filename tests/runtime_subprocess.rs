//! Tests for the Phase 3 subprocess runner.

use std::path::{Path, PathBuf};

use sunscreen::process::{CommandSpec, ProcessRunner, SubprocessRunner};

#[cfg(unix)]
fn write_script(bin_dir: &Path, name: &str, exit_code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = bin_dir.join(name);
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/sh
printf 'cwd=%s\n' "$PWD"
printf 'args=%s\n' "$*"
printf 'env=%s\n' "$SUNSCREEN_TEST_ENV"
printf 'stderr-line\n' >&2
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
fn write_script(bin_dir: &Path, name: &str, exit_code: i32) -> PathBuf {
    let path = bin_dir.join(format!("{name}.bat"));
    std::fs::write(
        &path,
        format!(
            r#"@echo off
echo cwd=%CD%
echo args=%*
echo env=%SUNSCREEN_TEST_ENV%
echo stderr-line 1>&2
exit /b {exit_code}
"#
        ),
    )
    .unwrap();
    path
}

#[test]
fn subprocess_runner_captures_output_exit_code_cwd_args_and_env() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let script = write_script(&bin_dir, "fake-command", 23);
    let cwd = tmp.path().join("workspace");
    std::fs::create_dir_all(&cwd).unwrap();

    let runner = SubprocessRunner;
    let output = runner
        .run(
            CommandSpec::new(script)
                .arg("alpha")
                .arg("beta")
                .cwd(&cwd)
                .env("SUNSCREEN_TEST_ENV", "present"),
        )
        .expect("run subprocess");

    assert_eq!(output.exit_code, 23);
    assert!(
        output.stdout.contains("args=alpha beta"),
        "{}",
        output.stdout
    );
    assert!(output.stdout.contains("env=present"), "{}", output.stdout);
    assert!(output.stderr.contains("stderr-line"), "{}", output.stderr);

    let cwd_line = output
        .stdout
        .lines()
        .find(|line| line.starts_with("cwd="))
        .expect("cwd line");
    let recorded = PathBuf::from(cwd_line.trim_start_matches("cwd="));
    assert_eq!(
        recorded.canonicalize().unwrap(),
        cwd.canonicalize().unwrap()
    );
}

#[test]
fn subprocess_runner_reports_missing_binary_as_spawn_error() {
    let runner = SubprocessRunner;
    let err = runner
        .run(CommandSpec::new("__sunscreen_missing_binary_for_test__"))
        .expect_err("missing binary should fail before producing output");

    assert!(err.is_not_found(), "{err}");
}
