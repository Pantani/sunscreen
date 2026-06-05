mod support;

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use super::support::CliEnv;

    fn write_exe(dir: &Path, name: &str, script: &str) {
        let path = dir.join(name);
        fs::write(&path, script).expect("write fake executable");
        let mut permissions = fs::metadata(&path)
            .expect("fake executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("chmod fake executable");
    }

    fn write_pnpm_that_installs_codama(fake_bin: &Path) {
        write_exe(
            fake_bin,
            "pnpm",
            r#"#!/bin/sh
echo "pnpm $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "${1:-}" = "add" ] && [ "${2:-}" = "--global" ] && [ "${3:-}" = "codama" ]; then
  cat > "$SUNSCREEN_FAKE_BIN/codama" <<'CODAMA'
#!/bin/sh
echo "codama $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "${1:-}" = "--version" ]; then
  echo "0.1.0"
else
  echo "fake codama $@"
fi
CODAMA
  chmod +x "$SUNSCREEN_FAKE_BIN/codama"
fi
exit 0
"#,
        );
    }

    #[test]
    fn doctor_fix_component_anchor_installs_anchor_through_avm() {
        let env = CliEnv::new();
        let fake_bin = env.path("bin");
        write_exe(
            &fake_bin,
            "cargo",
            r#"#!/bin/sh
echo "cargo $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "${1:-}" = "install" ]; then
  cat > "$SUNSCREEN_FAKE_BIN/avm" <<'AVM'
#!/bin/sh
echo "avm $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "${1:-}" = "use" ]; then
  cat > "$SUNSCREEN_FAKE_BIN/anchor" <<'ANCHOR'
#!/bin/sh
echo "anchor $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "${1:-}" = "--version" ]; then
  echo "anchor-cli 1.0.2"
else
  echo "fake anchor $@"
fi
ANCHOR
  chmod +x "$SUNSCREEN_FAKE_BIN/anchor"
fi
exit 0
AVM
  chmod +x "$SUNSCREEN_FAKE_BIN/avm"
fi
exit 0
"#,
        );

        let mut cmd = env.sunscreen();
        cmd.env("SUNSCREEN_FAKE_BIN", &fake_bin);
        cmd.args(["--json", "doctor", "--component", "anchor", "--fix"]);
        let payload = env.json_ok("doctor --fix anchor", &mut cmd);

        assert_eq!(payload["ok_after"], true);
        assert_eq!(payload["reports_after"][0]["name"], "anchor");
        assert_eq!(payload["reports_after"][0]["available"], true);
        assert_eq!(payload["fixes"][0]["name"], "anchor");
        assert_eq!(payload["fixes"][0]["status"], "fixed");
        assert_eq!(
            payload["fixes"][0]["commands"],
            serde_json::json!([
                [
                    "cargo",
                    "install",
                    "--git",
                    "https://github.com/solana-foundation/anchor",
                    "avm",
                    "--force"
                ],
                ["avm", "install", "latest"],
                ["avm", "use", "latest"]
            ])
        );
        assert_eq!(
            env.fake_log_lines(),
            [
                "cargo install --git https://github.com/solana-foundation/anchor avm --force",
                "avm install latest",
                "avm use latest",
                "anchor --version"
            ]
        );
    }

    #[test]
    fn doctor_fix_anchor_unparseable_after_repair_needs_inspection() {
        let env = CliEnv::new();
        let fake_bin = env.path("bin");
        write_exe(
            &fake_bin,
            "cargo",
            r#"#!/bin/sh
if [ "${1:-}" = "install" ]; then
  cat > "$SUNSCREEN_FAKE_BIN/avm" <<'AVM'
#!/bin/sh
if [ "${1:-}" = "use" ]; then
  cat > "$SUNSCREEN_FAKE_BIN/anchor" <<'ANCHOR'
#!/bin/sh
if [ "${1:-}" = "--version" ]; then
  echo "anchor from a custom wrapper"
fi
ANCHOR
  chmod +x "$SUNSCREEN_FAKE_BIN/anchor"
fi
exit 0
AVM
  chmod +x "$SUNSCREEN_FAKE_BIN/avm"
fi
exit 0
"#,
        );

        let mut cmd = env.sunscreen();
        cmd.env("SUNSCREEN_FAKE_BIN", &fake_bin);
        cmd.args(["--json", "doctor", "--component", "anchor", "--fix"]);
        let out = env.err("doctor --fix anchor unparseable", &mut cmd, 2);
        let stdout: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("stdout should remain JSON");
        let stderr = String::from_utf8_lossy(&out.stderr);

        assert_eq!(stdout["fixes"][0]["status"], "needs_inspection");
        assert!(stdout["fixes"][0]["message"]
            .as_str()
            .unwrap()
            .contains("anchor --version"));
        assert!(stderr.contains("doctor fix: anchor needs_inspection"));
    }

    #[test]
    fn doctor_fix_component_codama_repairs_optional_tool_when_targeted() {
        let env = CliEnv::new();
        let fake_bin = env.path("bin");
        write_pnpm_that_installs_codama(&fake_bin);

        let mut cmd = env.sunscreen();
        cmd.env("SUNSCREEN_FAKE_BIN", &fake_bin);
        cmd.args(["--json", "doctor", "--component", "codama", "--fix"]);
        let payload = env.json_ok("doctor --fix codama", &mut cmd);

        assert_eq!(payload["reports_before"][0]["name"], "codama");
        assert_eq!(payload["reports_before"][0]["available"], false);
        assert_eq!(payload["reports_after"][0]["available"], true);
        assert_eq!(payload["fixes"][0]["status"], "fixed");
        assert_eq!(
            payload["fixes"][0]["commands"],
            serde_json::json!([["pnpm", "add", "--global", "codama"]])
        );
        assert_eq!(
            env.fake_log_lines(),
            ["pnpm add --global codama", "codama --version"]
        );
    }

    #[test]
    fn doctor_fix_logs_each_step_to_stderr_without_polluting_json_stdout() {
        let env = CliEnv::new();
        let fake_bin = env.path("bin");
        write_pnpm_that_installs_codama(&fake_bin);

        let mut cmd = env.sunscreen();
        cmd.env("SUNSCREEN_FAKE_BIN", &fake_bin);
        cmd.args(["--json", "doctor", "--component", "codama", "--fix"]);
        let out = env.ok("doctor --fix codama logs", &mut cmd);
        let stdout: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("stdout should remain JSON");
        let stderr = String::from_utf8_lossy(&out.stderr);

        assert_eq!(stdout["fixes"][0]["status"], "fixed");
        assert!(
            stderr.contains("doctor fix: scanning 1 tool"),
            "missing scan log: {stderr}"
        );
        assert!(
            stderr.contains("doctor fix: codama is missing_optional"),
            "missing per-tool status log: {stderr}"
        );
        assert!(
            stderr.contains("doctor fix: running `pnpm add --global codama`"),
            "missing command start log: {stderr}"
        );
        assert!(
            stderr.contains("doctor fix: completed `pnpm add --global codama`"),
            "missing command completion log: {stderr}"
        );
        assert!(
            stderr.contains("doctor fix: re-checking 1 tool"),
            "missing re-check log: {stderr}"
        );
        assert!(
            stderr.contains("doctor fix: codama fixed"),
            "missing final status log: {stderr}"
        );
    }

    #[test]
    fn doctor_fix_solana_reports_downloader_failure_instead_of_reload_shell() {
        let env = CliEnv::new();
        let fake_bin = env.path("bin");
        write_exe(
            &fake_bin,
            "curl",
            r#"#!/bin/sh
echo "curl $@" >> "$SUNSCREEN_FAKE_LOG"
echo "curl: (92) HTTP/2 stream 1 was not closed cleanly: INTERNAL_ERROR (err 2)" >&2
exit 92
"#,
        );

        let mut cmd = env.sunscreen();
        cmd.args(["--json", "doctor", "--component", "solana", "--fix"]);
        let out = env.err("doctor --fix solana failed curl", &mut cmd, 2);
        let stdout: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("stdout should remain JSON");
        let stderr = String::from_utf8_lossy(&out.stderr);

        assert_eq!(stdout["fixes"][0]["name"], "solana");
        assert_eq!(stdout["fixes"][0]["status"], "failed");
        assert_ne!(stdout["fixes"][0]["status"], "needs_shell_reload");
        assert_eq!(stdout["fixes"][0]["exit_code"], 92);
        assert!(stdout["fixes"][0]["message"]
            .as_str()
            .unwrap()
            .contains("HTTP/2 stream 1 was not closed cleanly"));
        assert!(stderr.contains("doctor fix: failed `sh -c"));
        assert!(stderr.contains("HTTP/2 stream 1 was not closed cleanly"));
        assert!(env.fake_log().contains("curl --http1.1"));
    }
}
