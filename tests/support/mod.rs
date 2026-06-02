#![allow(dead_code)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct CliEnv {
    temp: tempfile::TempDir,
    home: PathBuf,
    config_home: PathBuf,
    fake_bin: PathBuf,
    fake_log: PathBuf,
}

impl CliEnv {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let home = temp.path().join("home");
        let config_home = temp.path().join("config-home");
        let fake_bin = temp.path().join("bin");
        let fake_log = temp.path().join("fake-toolchain.log");
        std::fs::create_dir_all(&home).expect("create fake home");
        std::fs::create_dir_all(&config_home).expect("create fake config home");
        std::fs::create_dir_all(&fake_bin).expect("create fake bin");
        Self {
            temp,
            home,
            config_home,
            fake_bin,
            fake_log,
        }
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.temp.path().join(rel)
    }

    pub fn sunscreen(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sunscreen"));
        cmd.env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("SUNSCREEN_FAKE_LOG", &self.fake_log)
            .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
            .env("NO_COLOR", "1")
            .env("PATH", self.path_with_fake_bin());
        cmd
    }

    pub fn sunscreen_in(&self, cwd: &Path) -> Command {
        let mut cmd = self.sunscreen();
        cmd.current_dir(cwd);
        cmd
    }

    pub fn sunscreen_fake_only(&self) -> Command {
        let mut cmd = self.sunscreen();
        cmd.env("PATH", self.fake_only_path());
        cmd
    }

    pub fn sunscreen_fake_only_in(&self, cwd: &Path) -> Command {
        let mut cmd = self.sunscreen_fake_only();
        cmd.current_dir(cwd);
        cmd
    }

    pub fn ok(&self, label: &str, cmd: &mut Command) -> Output {
        let out = cmd.output().unwrap_or_else(|err| panic!("{label}: {err}"));
        assert!(
            out.status.success(),
            "{label}: expected success, got code={:?}\nstdout={}\nstderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    pub fn err(&self, label: &str, cmd: &mut Command, expected_code: i32) -> Output {
        let out = cmd.output().unwrap_or_else(|err| panic!("{label}: {err}"));
        assert_eq!(
            out.status.code(),
            Some(expected_code),
            "{label}: unexpected exit code\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }

    pub fn json_ok(&self, label: &str, cmd: &mut Command) -> serde_json::Value {
        let out = self.ok(label, cmd);
        parse_json(label, &out.stdout, "stdout")
    }

    pub fn json_err(
        &self,
        label: &str,
        cmd: &mut Command,
        expected_code: i32,
    ) -> serde_json::Value {
        let out = self.err(label, cmd, expected_code);
        parse_json(label, &out.stderr, "stderr")
    }

    pub fn ndjson_ok(&self, label: &str, cmd: &mut Command) -> Vec<serde_json::Value> {
        let out = self.ok(label, cmd);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let events = stdout
            .lines()
            .enumerate()
            .map(|(index, line)| {
                serde_json::from_str(line).unwrap_or_else(|err| {
                    panic!("{label}: invalid NDJSON line {index}: {err}\nline={line}")
                })
            })
            .collect::<Vec<_>>();
        assert!(!events.is_empty(), "{label}: expected NDJSON events");
        events
    }

    pub fn chain_new(&self, name: &str, frontend: &str) -> PathBuf {
        let workspace = self.path(name);
        let mut cmd = self.sunscreen();
        cmd.args(["chain", "new", name, "--frontend", frontend, "--path"])
            .arg(&workspace);
        self.ok("chain new", &mut cmd);
        workspace
    }

    pub fn write_target_idl(&self, workspace: &Path, program: &str) -> PathBuf {
        let idl_dir = workspace.join("target/idl");
        std::fs::create_dir_all(&idl_dir).expect("create target/idl");
        let path = idl_dir.join(format!("{program}.json"));
        std::fs::write(&path, sample_idl(program)).expect("write target idl");
        path
    }

    pub fn install_fake_anchor_success(&self) {
        self.write_fake_command(
            "anchor",
            r#"#!/bin/sh
echo "anchor $@" >> "$SUNSCREEN_FAKE_LOG"
if [ "${1:-}" = "build" ]; then
  mkdir -p target/idl
  printf '%s\n' '{"metadata":{"name":"fake_app"},"instructions":[],"accounts":[],"errors":[],"types":[]}' > target/idl/fake_app.json
fi
echo "fake anchor $@"
exit 0
"#,
            r#"@echo off
echo anchor %*>> "%SUNSCREEN_FAKE_LOG%"
if "%1"=="build" (
  if not exist target\idl mkdir target\idl
  > target\idl\fake_app.json echo {"metadata":{"name":"fake_app"},"instructions":[],"accounts":[],"errors":[],"types":[]}
)
echo fake anchor %*
exit /b 0
"#,
        );
    }

    pub fn install_fake_pnpm_success(&self) {
        self.write_fake_command(
            "pnpm",
            r#"#!/bin/sh
echo "pnpm $@" >> "$SUNSCREEN_FAKE_LOG"
echo "fake pnpm $@"
exit 0
"#,
            r#"@echo off
echo pnpm %*>> "%SUNSCREEN_FAKE_LOG%"
echo fake pnpm %*
exit /b 0
"#,
        );
    }

    pub fn install_fake_solana_success(&self) {
        self.write_fake_command(
            "solana",
            r#"#!/bin/sh
echo "solana $@" >> "$SUNSCREEN_FAKE_LOG"
echo "1 SOL"
exit 0
"#,
            r#"@echo off
echo solana %*>> "%SUNSCREEN_FAKE_LOG%"
echo 1 SOL
exit /b 0
"#,
        );
        self.write_fake_command(
            "solana-keygen",
            r#"#!/bin/sh
echo "solana-keygen $@" >> "$SUNSCREEN_FAKE_LOG"
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--outfile" ]; then
    shift
    out="$1"
  fi
  shift
done
if [ -n "$out" ]; then
  mkdir -p "$(dirname "$out")"
  printf '[1,2,3,4]\n' > "$out"
fi
echo "pubkey: Fake111111111111111111111111111111111111111"
exit 0
"#,
            r#"@echo off
echo solana-keygen %*>> "%SUNSCREEN_FAKE_LOG%"
set OUT=
:loop
if "%~1"=="" goto done
if "%~1"=="--outfile" (
  shift
  set OUT=%~1
)
shift
goto loop
:done
if not "%OUT%"=="" (
  for %%I in ("%OUT%") do if not exist "%%~dpI" mkdir "%%~dpI"
  > "%OUT%" echo [1,2,3,4]
)
echo pubkey: Fake111111111111111111111111111111111111111
exit /b 0
"#,
        );
    }

    pub fn fake_log(&self) -> String {
        std::fs::read_to_string(&self.fake_log).unwrap_or_default()
    }

    pub fn fake_log_lines(&self) -> Vec<String> {
        self.fake_log()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>()
    }

    fn path_with_fake_bin(&self) -> OsString {
        let mut paths = vec![self.fake_bin.clone()];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        std::env::join_paths(paths).expect("join PATH")
    }

    fn fake_only_path(&self) -> OsString {
        let mut paths = vec![self.fake_bin.clone()];
        #[cfg(unix)]
        {
            paths.push(PathBuf::from("/bin"));
            paths.push(PathBuf::from("/usr/bin"));
        }
        std::env::join_paths(paths).expect("join fake-only PATH")
    }

    fn write_fake_command(&self, name: &str, unix: &str, windows: &str) -> PathBuf {
        let path = self.fake_bin.join(executable_name(name));
        let contents = if cfg!(windows) { windows } else { unix };
        std::fs::write(&path, contents).expect("write fake command");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&path)
                .expect("fake command metadata")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).expect("chmod fake command");
        }
        path
    }
}

fn parse_json(label: &str, bytes: &[u8], stream: &str) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|err| {
        panic!(
            "{label}: invalid JSON in {stream}: {err}\n{}",
            String::from_utf8_lossy(bytes)
        )
    })
}

fn sample_idl(program: &str) -> String {
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
      "name": "initializeVault",
      "accounts": [],
      "args": [
        {{ "name": "amount", "type": "u64" }}
      ]
    }}
  ],
  "accounts": [],
  "errors": [],
  "types": []
}}"#
    )
}

#[cfg(windows)]
fn executable_name(name: &str) -> String {
    format!("{name}.cmd")
}

#[cfg(not(windows))]
fn executable_name(name: &str) -> String {
    name.to_string()
}
