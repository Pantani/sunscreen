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
    fn doctor_fix_component_codama_repairs_optional_tool_when_targeted() {
        let env = CliEnv::new();
        let fake_bin = env.path("bin");
        write_exe(
            &fake_bin,
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
}
