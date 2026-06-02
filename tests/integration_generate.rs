mod support;

use support::CliEnv;

#[test]
fn generate_group_exports_idl_hooks_and_clients_command_by_command() {
    let env = CliEnv::new();
    env.install_fake_pnpm_success();

    let workspace = env.chain_new("generate_walk", "next");
    env.write_target_idl(&workspace, "generate_walk");

    let mut idl = env.sunscreen_in(&workspace);
    idl.args(["--json", "generate", "idl"]);
    let idl_report = env.json_ok("generate idl", &mut idl);
    assert_eq!(idl_report["command"], "generate_idl");
    assert_eq!(idl_report["changed_files"].as_array().unwrap().len(), 1);
    assert!(workspace.join("clients/idl/generate_walk.json").exists());

    let mut hooks = env.sunscreen_in(&workspace);
    hooks.args(["--json", "generate", "frontend-hooks"]);
    let hooks_report = env.json_ok("generate frontend-hooks", &mut hooks);
    assert_eq!(hooks_report["command"], "generate_frontend_hooks");
    assert!(workspace
        .join("app/src/generated/sunscreen/react.ts")
        .exists());

    let mut clients = env.sunscreen_in(&workspace);
    clients.args(["--json", "generate", "clients"]);
    let clients_report = env.json_ok("generate clients", &mut clients);
    assert_eq!(clients_report["command"], "generate_clients");
    assert_eq!(clients_report["ok"], true);
    assert!(workspace.join("codama.json").exists());

    let log = env.fake_log();
    assert!(
        log.contains("pnpm exec codama run --all --config codama.json"),
        "{log}"
    );
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
    assert_eq!(
        payload["next_step"],
        "run `sunscreen init <name>` or change into a sunscreen workspace"
    );
}
