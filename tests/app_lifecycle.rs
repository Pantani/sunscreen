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

fn seed_workspace_with_config(root: &Path, config: &str) {
    fs::write(root.join("sunscreen.yml"), config).expect("write sunscreen.yml");
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
    for sub in [
        "install",
        "uninstall",
        "list",
        "describe",
        "update",
        "commands",
        "run",
        "hook",
        "marketplace",
    ] {
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

#[cfg(unix)]
fn write_stdio_plugin_script(path: &Path, body_json: &str) {
    use std::os::unix::fs::PermissionsExt;

    let response = format!("Content-Length: {}\r\n\r\n{}", body_json.len(), body_json);
    fs::write(
        path,
        format!(
            "#!/bin/sh\ncat >/tmp/sunscreen-plugin-stdin.$$ || true\nprintf '%s' '{}'\n",
            response.replace('\'', "'\\''")
        ),
    )
    .expect("write plugin script");
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod plugin script");
}

#[cfg(unix)]
fn seed_stdio_plugin_workspace(env: &CliEnv, sub: &str) -> PathBuf {
    let ws = env.path(sub);
    fs::create_dir_all(ws.join("plugins/hello")).expect("create plugin dir");
    write_stdio_plugin_script(
        &ws.join("plugins/hello/plugin.sh"),
        r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true,"message":"hello from plugin","files":[{"path":"programs/demo/src/hello.rs","action":"would_write"}]}}"#,
    );
    fs::write(
        ws.join("plugins/hello/sunscreen-plugin.json"),
        r#"{
  "name": "hello",
  "version": "0.1.0",
  "transport": "stdio-jsonrpc",
  "entrypoint": ["./plugin.sh"],
  "commands": [
    { "name": "hello", "kind": "app", "summary": "Run hello" },
    { "name": "indexer", "kind": "scaffold", "summary": "Scaffold an indexer" }
  ],
  "hooks": ["post-codama"],
  "capabilities": { "network": true, "filesystem": ["workspace", "scratch"] }
}
"#,
    )
    .expect("write plugin manifest");
    seed_workspace_with_config(
        &ws,
        "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: plugins/hello
    version: 0.1.0
    manifest: plugins/hello/sunscreen-plugin.json
",
    );
    ws
}

#[cfg(unix)]
fn seed_bad_plugin_workspace(
    env: &CliEnv,
    sub: &str,
    manifest_entrypoint: &str,
    script: &str,
) -> PathBuf {
    let ws = env.path(sub);
    fs::create_dir_all(ws.join("plugins/hello")).expect("create plugin dir");
    fs::write(ws.join("plugins/hello/plugin.sh"), script).expect("write plugin script");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(ws.join("plugins/hello/plugin.sh"))
            .expect("metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(ws.join("plugins/hello/plugin.sh"), perms).expect("chmod");
    }
    fs::write(
        ws.join("plugins/hello/sunscreen-plugin.json"),
        format!(
            r#"{{
  "name": "hello",
  "version": "0.1.0",
  "transport": "stdio-jsonrpc",
  "entrypoint": [{manifest_entrypoint}],
  "commands": [{{ "name": "hello", "kind": "app" }}],
  "capabilities": {{ "network": true, "filesystem": ["workspace", "scratch"] }}
}}
"#
        ),
    )
    .expect("write plugin manifest");
    seed_workspace_with_config(
        &ws,
        "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: plugins/hello
    version: 0.1.0
    manifest: plugins/hello/sunscreen-plugin.json
",
    );
    ws
}

#[cfg(unix)]
fn seed_plugin_workspace_with_capabilities(
    env: &CliEnv,
    sub: &str,
    capabilities: &str,
    script: &str,
) -> PathBuf {
    let ws = env.path(sub);
    fs::create_dir_all(ws.join("plugins/hello")).expect("create plugin dir");
    fs::write(ws.join("plugins/hello/plugin.sh"), script).expect("write plugin script");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(ws.join("plugins/hello/plugin.sh"))
            .expect("metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(ws.join("plugins/hello/plugin.sh"), perms).expect("chmod");
    }
    fs::write(
        ws.join("plugins/hello/sunscreen-plugin.json"),
        format!(
            r#"{{
  "name": "hello",
  "version": "0.1.0",
  "transport": "stdio-jsonrpc",
  "entrypoint": ["./plugin.sh"],
  "commands": [{{ "name": "hello", "kind": "app" }}],
  "capabilities": {capabilities}
}}
"#
        ),
    )
    .expect("write plugin manifest");
    seed_workspace_with_config(
        &ws,
        "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: plugins/hello
    version: 0.1.0
    manifest: plugins/hello/sunscreen-plugin.json
",
    );
    ws
}

#[cfg(unix)]
fn seed_ambiguous_stdio_plugins_workspace(env: &CliEnv, sub: &str) -> PathBuf {
    let ws = env.path(sub);
    for (dir, message) in [
        ("plugins/alpha", "alpha plugin"),
        ("plugins/beta", "beta plugin"),
    ] {
        fs::create_dir_all(ws.join(dir)).expect("create plugin dir");
        write_stdio_plugin_script(
            &ws.join(dir).join("plugin.sh"),
            &format!(r#"{{"jsonrpc":"2.0","id":1,"result":{{"message":"{message}"}}}}"#),
        );
        fs::write(
            ws.join(dir).join("sunscreen-plugin.json"),
            r#"{
  "name": "hello",
  "version": "0.1.0",
  "transport": "stdio-jsonrpc",
  "entrypoint": ["./plugin.sh"],
  "commands": [{ "name": "hello", "kind": "app" }],
  "capabilities": { "network": true, "filesystem": ["workspace", "scratch"] }
}
"#,
        )
        .expect("write plugin manifest");
    }
    seed_workspace_with_config(
        &ws,
        "version: 1
project:
  name: demo
  framework: anchor
programs:
  - name: demo
    path: programs/demo
plugins:
  - source: plugins/alpha
    version: 0.1.0
    manifest: plugins/alpha/sunscreen-plugin.json
  - source: plugins/beta
    version: 0.1.0
    manifest: plugins/beta/sunscreen-plugin.json
",
    );
    ws
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

#[cfg(unix)]
#[test]
fn commands_lists_manifest_declared_dynamic_commands() {
    let env = CliEnv::new();
    let ws = seed_stdio_plugin_workspace(&env, "ws-plugin-commands");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "commands"]);
    let payload = env.json_ok("app commands", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "app commands");
    let commands = payload["commands"].as_array().expect("commands array");
    assert!(
        commands.iter().any(|cmd| {
            cmd["plugin"] == "hello"
                && cmd["name"] == "hello"
                && cmd["kind"] == "app"
                && cmd["transport"] == "stdio-jsonrpc"
        }),
        "missing app command in payload: {payload}"
    );
    assert!(
        commands.iter().any(|cmd| cmd["plugin"] == "hello"
            && cmd["name"] == "indexer"
            && cmd["kind"] == "scaffold"),
        "missing scaffold command in payload: {payload}"
    );
}

#[cfg(unix)]
#[test]
fn run_executes_stdio_jsonrpc_plugin_command() {
    let env = CliEnv::new();
    let ws = seed_stdio_plugin_workspace(&env, "ws-plugin-run");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "hello", "hello", "--", "Alice"]);
    let payload = env.json_ok("app run", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "app run");
    assert_eq!(payload["plugin"], "hello");
    assert_eq!(payload["plugin_command"], "hello");
    assert_eq!(payload["transport"], "stdio-jsonrpc");
    assert_eq!(payload["result"]["message"], "hello from plugin");
}

#[cfg(unix)]
#[test]
fn run_rejects_plugin_without_workspace_capability_before_spawn() {
    let env = CliEnv::new();
    let ws = seed_plugin_workspace_with_capabilities(
        &env,
        "ws-plugin-scratch-only",
        r#"{ "network": false, "filesystem": ["scratch"] }"#,
        "#!/bin/sh\ntouch spawned.txt\nprintf 'Content-Length: 51\\r\\n\\r\\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}'\n",
    );

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "hello", "hello"]);
    let payload = env.json_err("app run scratch-only", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
    assert!(
        payload["error"].as_str().unwrap().contains("workspace"),
        "unexpected error payload: {payload}"
    );
    assert!(
        !ws.join("plugins/hello/spawned.txt").exists(),
        "plugin was spawned before capability enforcement"
    );
}

#[cfg(unix)]
#[test]
fn run_rejects_stdio_plugin_without_network_capability_before_spawn() {
    let env = CliEnv::new();
    let ws = seed_plugin_workspace_with_capabilities(
        &env,
        "ws-plugin-network-denied",
        r#"{ "network": false, "filesystem": ["workspace", "scratch"] }"#,
        "#!/bin/sh\ntouch spawned.txt\nprintf 'Content-Length: 51\\r\\n\\r\\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}'\n",
    );

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "hello", "hello"]);
    let payload = env.json_err("app run network-denied", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
    assert!(
        payload["error"].as_str().unwrap().contains("network"),
        "unexpected error payload: {payload}"
    );
    assert!(
        !ws.join("plugins/hello/spawned.txt").exists(),
        "plugin was spawned before capability enforcement"
    );
}

#[cfg(unix)]
#[test]
fn run_rejects_ambiguous_plugin_alias_before_dispatch() {
    let env = CliEnv::new();
    let ws = seed_ambiguous_stdio_plugins_workspace(&env, "ws-plugin-run-ambiguous");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "hello", "hello"]);
    let payload = env.json_err("app run ambiguous target", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
    assert!(
        payload["error"].as_str().unwrap().contains("matches 2"),
        "unexpected error payload: {payload}"
    );
}

#[cfg(unix)]
#[test]
fn run_allows_exact_source_to_disambiguate_plugin_alias() {
    let env = CliEnv::new();
    let ws = seed_ambiguous_stdio_plugins_workspace(&env, "ws-plugin-run-exact-source");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "plugins/beta", "hello"]);
    let payload = env.json_ok("app run exact source", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["plugin"], "hello");
    assert_eq!(payload["result"]["message"], "beta plugin");
}

#[cfg(unix)]
#[test]
fn hook_executes_declared_stdio_plugin_hook() {
    let env = CliEnv::new();
    let ws = seed_stdio_plugin_workspace(&env, "ws-plugin-hook");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args([
        "--json",
        "app",
        "hook",
        "post-codama",
        "--",
        "clients/idl/demo.json",
    ]);
    let payload = env.json_ok("app hook", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "app hook");
    assert_eq!(payload["hook"], "post-codama");
    assert_eq!(payload["reports"][0]["plugin"], "hello");
    assert_eq!(
        payload["reports"][0]["result"]["message"],
        "hello from plugin"
    );
}

#[cfg(unix)]
#[test]
fn run_rejects_entrypoint_that_escapes_plugin_directory() {
    let env = CliEnv::new();
    let ws = seed_bad_plugin_workspace(
        &env,
        "ws-plugin-path-traversal",
        r#""../evil.sh""#,
        "#!/bin/sh\nexit 0\n",
    );

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "hello", "hello"]);
    let payload = env.json_err("app run traversal", &mut cmd, 9);
    assert_eq!(payload["kind"], "plugin_runtime");
}

#[cfg(unix)]
#[test]
fn run_maps_nonzero_plugin_exit_to_plugin_runtime() {
    let env = CliEnv::new();
    let ws = seed_bad_plugin_workspace(
        &env,
        "ws-plugin-nonzero",
        r#""./plugin.sh""#,
        "#!/bin/sh\necho plugin failed >&2\nexit 23\n",
    );

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args(["--json", "app", "run", "hello", "hello"]);
    let payload = env.json_err("app run nonzero", &mut cmd, 9);
    assert_eq!(payload["kind"], "plugin_runtime");
    assert_eq!(payload["exit_code"], 9);
}

#[cfg(unix)]
#[test]
fn scaffold_external_subcommand_routes_to_plugin_runtime() {
    let env = CliEnv::new();
    let ws = seed_stdio_plugin_workspace(&env, "ws-plugin-scaffold");

    let mut cmd = env.sunscreen_in(&ws);
    cmd.args([
        "--json",
        "scaffold",
        "indexer",
        "--",
        "--cluster",
        "localnet",
    ]);
    let payload = env.json_ok("plugin scaffold", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "scaffold indexer");
    assert_eq!(payload["plugin"], "hello");
    assert_eq!(payload["transport"], "stdio-jsonrpc");
    assert_eq!(payload["result"]["message"], "hello from plugin");
}

#[test]
fn marketplace_lists_reference_plugins() {
    let env = CliEnv::new();
    let mut cmd = env.sunscreen();
    cmd.args(["--json", "app", "marketplace"]);
    let payload = env.json_ok("app marketplace", &mut cmd);
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["command"], "app marketplace");
    let plugins = payload["plugins"].as_array().expect("plugins array");
    assert!(
        plugins
            .iter()
            .any(|plugin| plugin["name"] == "spl-token-2022" && plugin["transport"] == "grpc"),
        "missing spl-token-2022 reference plugin: {payload}"
    );
    assert!(
        plugins.iter().any(|plugin| {
            plugin["name"] == "yellowstone-indexer" && plugin["transport"] == "stdio-jsonrpc"
        }),
        "missing yellowstone-indexer reference plugin: {payload}"
    );
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
