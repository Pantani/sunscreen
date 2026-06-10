mod support;

use std::path::Path;

use support::CliEnv;

#[test]
fn generate_group_exports_idl_hooks_and_clients_command_by_command() {
    let env = CliEnv::new();
    env.install_fake_pnpm_success();

    let workspace = env.chain_new("generate_walk", "next");
    env.write_target_idl(&workspace, "generate_walk");

    generate_idl(&env, &workspace);
    generate_frontend_hooks(&env, &workspace);
    generate_clients(&env, &workspace);
    assert_idl_rerun_is_unchanged(&env, &workspace);

    assert_eq!(
        env.fake_log_lines(),
        ["pnpm exec codama run --all --config codama.json"]
    );
}

fn generate_idl(env: &CliEnv, workspace: &Path) {
    let mut idl = env.sunscreen_in(workspace);
    idl.args(["--json", "generate", "idl"]);
    let idl_report = env.json_ok("generate idl", &mut idl);
    assert_eq!(idl_report["ok"], true);
    assert_eq!(idl_report["command"], "generate_idl");
    assert_eq!(
        idl_report["files"],
        serde_json::json!(["clients/idl/generate_walk.json"])
    );
    assert_eq!(
        idl_report["changed_files"],
        serde_json::json!(["clients/idl/generate_walk.json"])
    );
    let exported_idl = workspace.join("clients/idl/generate_walk.json");
    assert!(exported_idl.exists());
    assert_exported_idl(&exported_idl);
}

fn assert_exported_idl(exported_idl: &Path) {
    let exported_idl_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(exported_idl).unwrap())
            .expect("exported idl json");
    assert!(
        exported_idl_json["metadata"]["name"].is_string(),
        "exported IDL should include metadata.name: {exported_idl_json:#?}"
    );
    assert_eq!(exported_idl_json["metadata"]["name"], "generate_walk");
    let instructions = exported_idl_json["instructions"]
        .as_array()
        .expect("exported IDL instructions should be an array");
    assert!(
        !instructions.is_empty(),
        "exported IDL instructions should not be empty"
    );
    assert_eq!(instructions[0]["name"], "initializeVault");
}

fn generate_frontend_hooks(env: &CliEnv, workspace: &Path) {
    let mut hooks = env.sunscreen_in(workspace);
    hooks.args(["--json", "generate", "frontend-hooks"]);
    let hooks_report = env.json_ok("generate frontend-hooks", &mut hooks);
    assert_eq!(hooks_report["ok"], true);
    assert_eq!(hooks_report["command"], "generate_frontend_hooks");
    assert!(workspace
        .join("app/src/generated/sunscreen/react.ts")
        .exists());
    assert!(hooks_report["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "app/src/generated/sunscreen/react.ts"));
    assert!(!workspace
        .join("app/src/generated/sunscreen/solid.ts")
        .exists());
    let react_hooks =
        std::fs::read_to_string(workspace.join("app/src/generated/sunscreen/react.ts")).unwrap();
    assert!(react_hooks.contains("useInitializeVaultMutation"));
}

fn generate_clients(env: &CliEnv, workspace: &Path) {
    let mut clients = env.sunscreen_fake_only_in(workspace);
    clients.args(["--json", "generate", "clients"]);
    let clients_report = env.json_ok("generate clients", &mut clients);
    assert_eq!(clients_report["command"], "generate_clients");
    assert_eq!(clients_report["ok"], true);
    assert_eq!(clients_report["exit_code"], 0);
    assert_eq!(clients_report["codama_config"], "codama.json");
    assert_eq!(clients_report["stderr"], "");
    assert!(clients_report["stdout"]
        .as_str()
        .unwrap()
        .contains("fake pnpm exec codama run"));
    assert!(workspace.join("codama.json").exists());
    let codama_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(workspace.join("codama.json")).unwrap())
            .expect("codama config json");
    assert_eq!(codama_config["idl"], "target/idl/generate_walk.json");
    assert_eq!(
        codama_config["scripts"]["js"]["from"],
        "@codama/renderers-js"
    );
}

fn assert_idl_rerun_is_unchanged(env: &CliEnv, workspace: &Path) {
    let mut idl_again = env.sunscreen_in(workspace);
    idl_again.args(["--json", "generate", "idl"]);
    let idl_again_report = env.json_ok("generate idl idempotent rerun", &mut idl_again);
    assert_eq!(idl_again_report["changed_files"], serde_json::json!([]));
}

#[test]
fn generate_group_reports_missing_workspace_as_json_error() {
    let env = CliEnv::new();
    let empty = env.path("empty");
    std::fs::create_dir_all(&empty).unwrap();

    let mut cmd = env.sunscreen_in(&empty);
    cmd.args(["--json", "generate", "idl"]);
    let payload = env.json_err("generate idl outside workspace", &mut cmd, 5);
    assert_eq!(payload["kind"], "workspace_missing");
    assert_eq!(payload["exit_code"], 5);
    assert_eq!(
        payload["next_step"],
        "run `sunscreen init <name>` or change into a sunscreen workspace"
    );
}
