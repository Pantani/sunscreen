//! Integration tests for `sunscreen app` — declarative plugin lifecycle.
//!
//! These tests drive the compiled binary through `tests/support` so the
//! exit-code / stdout / `sunscreen.yml` contract is exercised end-to-end.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use support::CliEnv;

const MIN_YML: &str = "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
";

fn seed_workspace(root: &Path) {
    fs::write(root.join("sunscreen.yml"), MIN_YML).expect("write sunscreen.yml");
    let prog = root.join("programs/demo/src/instructions");
    fs::create_dir_all(&prog).expect("create program dir");
    fs::write(prog.join("mod.rs"), "// noop\n").expect("write mod.rs");
    fs::write(prog.parent().unwrap().join("lib.rs"), "// noop\n").expect("write lib.rs");
}

fn seeded(env: &CliEnv, sub: &str) -> PathBuf {
    let ws = env.path(sub);
    fs::create_dir_all(&ws).expect("create workspace dir");
    seed_workspace(&ws);
    ws
}

fn read_yml(ws: &Path) -> String {
    fs::read_to_string(ws.join("sunscreen.yml")).expect("read sunscreen.yml")
}

#[test]
fn app_help_lists_subcommands_without_todo() {
    let env = CliEnv::new();
    let mut cmd = env.sunscreen();
    cmd.args(["app", "--help"]);
    let out = env.ok("app --help", &mut cmd);
    let text = String::from_utf8_lossy(&out.stdout);
    for sub in ["install", "uninstall", "list", "describe", "update"] {
        assert!(
            text.contains(sub),
            "app --help missing subcommand `{sub}`:\n{text}"
        );
    }
    assert!(
        !text.contains("TODO"),
        "app --help still prints TODO:\n{text}"
    );
}

#[test]
fn list_in_empty_workspace_returns_empty_apps() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-list-empty");
    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "list"]);
    let payload = env.json_ok("app list --json", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "app list");
    assert!(
        payload["apps"]
            .as_array()
            .map(Vec::is_empty)
            .unwrap_or(false),
        "expected empty apps array, got {payload}"
    );
    assert_eq!(payload["changed"], false);
    assert_eq!(payload["dry_run"], false);
}

#[test]
fn install_writes_plugins_and_is_idempotent() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-install");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "install", "github.com/org/foo.git@1.2.3"]);
    let payload = env.json_ok("install", &mut cmd);
    assert_eq!(payload["changed"], true);
    assert_eq!(payload["dry_run"], false);
    assert_eq!(payload["app"]["status"], "declared");
    assert_eq!(payload["app"]["name"], "foo");
    assert_eq!(payload["app"]["version"], "1.2.3");

    let yml = read_yml(&ws);
    assert!(
        yml.contains("github.com/org/foo.git"),
        "plugins block missing in sunscreen.yml:\n{yml}"
    );

    // Idempotent re-run: same source@version → no change.
    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "install", "github.com/org/foo.git@1.2.3"]);
    let payload = env.json_ok("install (idempotent)", &mut cmd);
    assert_eq!(payload["changed"], false);
}

#[test]
fn install_dry_run_does_not_modify_disk() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-install-dry");
    let before = read_yml(&ws);

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args([
        "--json",
        "app",
        "install",
        "github.com/org/foo.git",
        "--version",
        "v0.1.0",
        "--dry-run",
    ]);
    let payload = env.json_ok("install --dry-run", &mut cmd);
    assert_eq!(payload["dry_run"], true);
    assert_eq!(payload["changed"], true);
    assert_eq!(read_yml(&ws), before, "dry-run mutated sunscreen.yml");
}

#[test]
fn describe_matches_by_name_and_by_source() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-describe");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["app", "install", "github.com/org/foo.git@1.0.0"]);
    env.ok("seed install", &mut cmd);

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "describe", "foo"]);
    let payload = env.json_ok("describe by basename", &mut cmd);
    assert_eq!(payload["app"]["source"], "github.com/org/foo.git");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "describe", "github.com/org/foo.git"]);
    let payload = env.json_ok("describe by source", &mut cmd);
    assert_eq!(payload["app"]["name"], "foo");
    assert_eq!(payload["app"]["version"], "1.0.0");
    assert_eq!(payload["app"]["status"], "declared");
}

#[test]
fn uninstall_removes_entry() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-uninstall");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["app", "install", "github.com/org/foo.git@1.0.0"]);
    env.ok("seed install", &mut cmd);

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "uninstall", "foo"]);
    let payload = env.json_ok("uninstall", &mut cmd);
    assert_eq!(payload["changed"], true);

    let yml = read_yml(&ws);
    assert!(
        !yml.contains("github.com/org/foo.git"),
        "uninstall left source behind:\n{yml}"
    );
}

#[test]
fn update_version_changes_pinned_version() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-update");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["app", "install", "github.com/org/foo.git@1.0.0"]);
    env.ok("seed install", &mut cmd);

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "update", "foo", "--version", "v2.0.0"]);
    let payload = env.json_ok("update", &mut cmd);
    assert_eq!(payload["changed"], true);
    assert_eq!(payload["app"]["version"], "v2.0.0");
}

#[test]
fn update_without_version_flag_exits_user_input() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-update-noversion");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["app", "install", "github.com/org/foo.git@1.0.0"]);
    env.ok("seed install", &mut cmd);

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "update", "foo"]);
    let payload = env.json_err("update missing --version", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
}

#[test]
fn workspace_missing_exits_5() {
    let env = CliEnv::new();
    let empty = env.path("no-workspace");
    fs::create_dir_all(&empty).expect("mkdir");
    let mut cmd = env.sunscreen_in(&empty);
    cmd.args(["--json", "app", "list"]);
    let payload = env.json_err("list without workspace", &mut cmd, 5);
    assert_eq!(payload["kind"], "workspace_missing");
}

#[test]
fn invalid_config_exits_3() {
    let env = CliEnv::new();
    let ws = env.path("ws-invalid");
    fs::create_dir_all(&ws).expect("mkdir");
    // Duplicate plugin source triggers Config::validate semantic error.
    let yml = "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: github.com/org/foo.git
  - source: github.com/org/foo.git
";
    fs::write(ws.join("sunscreen.yml"), yml).expect("write yml");
    let prog = ws.join("programs/demo/src/instructions");
    fs::create_dir_all(&prog).expect("create program dir");
    fs::write(prog.join("mod.rs"), "// noop\n").unwrap();
    fs::write(prog.parent().unwrap().join("lib.rs"), "// noop\n").unwrap();

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "list"]);
    let payload = env.json_err("list with invalid config", &mut cmd, 3);
    assert_eq!(payload["kind"], "config_invalid");
}

#[test]
fn install_basename_collision_with_different_source_exits_4() {
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-collision");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["app", "install", "github.com/org/foo.git@1.0.0"]);
    env.ok("seed install", &mut cmd);

    // Different source, same basename `foo`.
    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "install", "gitlab.com/other/foo@1.0.0"]);
    let payload = env.json_err("collision", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
}

#[test]
fn ambiguous_describe_exits_4() {
    let env = CliEnv::new();
    let ws = env.path("ws-ambiguous");
    fs::create_dir_all(&ws).expect("mkdir");
    // Two distinct sources whose basenames both normalize to `foo`.
    let yml = "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: github.com/org/foo.git
  - source: gitlab.com/other/foo
";
    fs::write(ws.join("sunscreen.yml"), yml).expect("write yml");
    let prog = ws.join("programs/demo/src/instructions");
    fs::create_dir_all(&prog).expect("create prog dir");
    fs::write(prog.join("mod.rs"), "// noop\n").unwrap();
    fs::write(prog.parent().unwrap().join("lib.rs"), "// noop\n").unwrap();

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "describe", "foo"]);
    let payload = env.json_err("ambiguous describe", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
}

#[test]
fn env_overlay_does_not_leak_into_persisted_manifest() {
    // Regression: an `app install` invoked with a transient
    // `SUNSCREEN_PROJECT__NAME=...` overlay must edit only `plugins[]` and
    // leave every other field exactly as it was on disk. Without this
    // guarantee, env-overlay values would silently rewrite the manifest.
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-overlay-leak");
    let mut cmd = env.sunscreen_in(&ws);
    cmd.env("SUNSCREEN_PROJECT__NAME", "ci-demo-overlay");
    cmd.args(["app", "install", "github.com/org/foo.git@1.2.3"]);
    env.ok("install with overlay", &mut cmd);

    let after = read_yml(&ws);
    assert!(
        after.contains("github.com/org/foo.git"),
        "install did not record the plugin:\n{after}"
    );
    assert!(
        !after.contains("ci-demo-overlay"),
        "env overlay leaked into sunscreen.yml:\n{after}"
    );
    // Re-parse and assert the project name is still the on-disk value, not
    // the overlay value — serde_yaml may normalize whitespace/indentation
    // on round-trip, so we compare semantic fields rather than raw bytes.
    let reparsed: serde_yaml::Value =
        serde_yaml::from_str(&after).expect("post-install yml parses");
    assert_eq!(
        reparsed["project"]["name"].as_str(),
        Some("demo"),
        "env overlay overwrote project.name:\n{after}"
    );
}

#[test]
fn bare_reinstall_preserves_existing_pinned_version() {
    // Regression: `install <source>` with no `@version` / `--version` must
    // NOT silently unpin a previously pinned entry.
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-reinstall-bare");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["app", "install", "github.com/org/foo.git@1.2.3"]);
    env.ok("seed install", &mut cmd);

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "install", "github.com/org/foo.git"]);
    let payload = env.json_ok("bare reinstall", &mut cmd);
    assert_eq!(payload["changed"], false);
    assert_eq!(payload["app"]["version"], "1.2.3");

    let yml = read_yml(&ws);
    assert!(
        yml.contains("1.2.3"),
        "bare reinstall unpinned the existing entry:\n{yml}"
    );
}

#[test]
fn install_of_exact_source_succeeds_even_with_basename_neighbour() {
    // Regression: a same-basename neighbour must not shadow the exact-source
    // match. Re-installing the exact source of an already-declared plugin is
    // idempotent even when another entry shares its normalized basename.
    let env = CliEnv::new();
    let ws = env.path("ws-exact-vs-basename");
    fs::create_dir_all(&ws).expect("mkdir");
    // Seed two distinct sources whose basenames both normalize to `foo`.
    let yml = "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: gitlab.com/other/foo
    version: 0.1.0
  - source: github.com/org/foo.git
    version: 1.2.3
";
    fs::write(ws.join("sunscreen.yml"), yml).expect("write yml");
    let prog = ws.join("programs/demo/src/instructions");
    fs::create_dir_all(&prog).expect("create prog dir");
    fs::write(prog.join("mod.rs"), "// noop\n").unwrap();
    fs::write(prog.parent().unwrap().join("lib.rs"), "// noop\n").unwrap();

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "install", "github.com/org/foo.git@1.2.3"]);
    let payload = env.json_ok("exact reinstall", &mut cmd);
    assert_eq!(payload["changed"], false);
    assert_eq!(payload["app"]["version"], "1.2.3");
}

#[test]
fn no_app_subcommand_executes_external_process() {
    // The fake-toolchain log captures every external tool invocation. None
    // of the `app` subcommands may shell out — they are pure declaration
    // mutators on `sunscreen.yml`. If anything else were to run, it would
    // be recorded under `SUNSCREEN_FAKE_LOG`.
    let env = CliEnv::new();
    let ws = seeded(&env, "ws-no-exec");

    for (label, args) in [
        ("install", vec!["app", "install", "foo@1.2.3"]),
        ("list", vec!["app", "list"]),
        ("describe", vec!["app", "describe", "foo"]),
        ("update", vec!["app", "update", "foo", "--version", "1.2.4"]),
        ("uninstall", vec!["app", "uninstall", "foo"]),
    ] {
        let mut cmd = env.sunscreen_in(&ws);
        cmd.args(&args);
        env.ok(label, &mut cmd);
    }

    let log_path = env.path("fake-toolchain.log");
    if log_path.exists() {
        let body = fs::read_to_string(&log_path).expect("read fake log");
        assert!(
            body.trim().is_empty(),
            "app subcommands invoked external process(es):\n{body}"
        );
    }
}
