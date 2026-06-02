mod support;

use support::CliEnv;

#[test]
fn chain_group_runs_new_doctor_and_build_pipeline_through_real_binary() {
    let env = CliEnv::new();
    env.install_fake_anchor_success();
    env.install_fake_pnpm_success();

    let workspace = env.chain_new("chain_walk", "none");
    let nested = workspace.join("programs/chain_walk/src");

    let mut doctor = env.sunscreen_in(&nested);
    doctor.args(["--json", "chain", "doctor"]);
    let report = env.json_ok("chain doctor", &mut doctor);
    assert_eq!(report["ok"], true);
    assert_eq!(report["fix_markers"], false);
    assert_eq!(report["drift_count"], 0);
    assert_eq!(report["unresolved"], 0);
    let findings = report["findings"].as_array().expect("doctor findings");
    assert!(
        findings.iter().all(|finding| finding["status"] == "ok"),
        "expected all marker findings to be ok: {findings:#?}"
    );

    let mut build = env.sunscreen_fake_only_in(&nested);
    build.args(["--json", "chain", "build", "--headless"]);
    let events = env.ndjson_ok("chain build", &mut build);
    let names: Vec<_> = events
        .iter()
        .map(|event| event["event"].as_str().unwrap())
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
    assert_eq!(events[0]["codama"], true);
    assert_eq!(events[0]["programs"], serde_json::json!(["chain-walk"]));
    assert_eq!(events.last().unwrap()["status"], "ok");
    assert_eq!(events.last().unwrap()["exit_code"], 0);
    let finished: Vec<_> = events
        .iter()
        .filter(|event| event["event"] == "command_finished")
        .collect();
    assert_eq!(finished.len(), 2);
    for event in finished {
        assert_eq!(event["exit_code"], 0);
        assert_eq!(event["status"], "ok");
    }
    assert!(workspace.join("codama.json").exists());
    let codama_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(workspace.join("codama.json")).unwrap())
            .expect("codama config json");
    assert_eq!(codama_config["idl"], "target/idl/fake_app.json");
    assert!(workspace.join("target/idl/fake_app.json").exists());

    assert_eq!(
        env.fake_log_lines(),
        [
            "anchor build",
            "pnpm exec codama run --all --config codama.json"
        ]
    );
}

#[test]
fn chain_group_build_no_codama_skips_pnpm() {
    let env = CliEnv::new();
    env.install_fake_anchor_success();
    env.install_fake_pnpm_success();

    let workspace = env.chain_new("chain_no_codama", "none");
    let mut build = env.sunscreen_fake_only_in(&workspace);
    build.args(["--json", "chain", "build", "--headless", "--no-codama"]);
    let events = env.ndjson_ok("chain build --no-codama", &mut build);

    assert_eq!(events[0]["codama"], false);
    let steps: Vec<_> = events
        .iter()
        .filter_map(|event| event["step"].as_str())
        .collect();
    assert_eq!(steps, ["anchor_build", "anchor_build"]);
    assert_eq!(events.last().unwrap()["status"], "ok");
    assert_eq!(env.fake_log_lines(), ["anchor build"]);
    assert!(
        !workspace.join("codama.json").exists(),
        "--no-codama should not write a managed Codama config"
    );
}

#[test]
fn chain_group_pinocchio_bootstrap_and_anchor_only_guards() {
    let env = CliEnv::new();
    let workspace = env.path("pinocchio_walk");

    let mut new_cmd = env.sunscreen();
    new_cmd.args([
        "chain",
        "new",
        "pinocchio_walk",
        "--framework",
        "pinocchio",
        "--frontend",
        "none",
        "--path",
    ]);
    new_cmd.arg(&workspace);
    env.ok("chain new pinocchio", &mut new_cmd);

    let cfg = std::fs::read_to_string(workspace.join("sunscreen.yml")).unwrap();
    assert!(cfg.contains("framework: pinocchio"));
    assert!(!workspace.join("Anchor.toml").exists());

    let mut scaffold = env.sunscreen_in(&workspace);
    scaffold.args([
        "--json",
        "scaffold",
        "instruction",
        "deposit",
        "--program",
        "pinocchio-walk",
    ]);
    let payload = env.json_err("pinocchio scaffold guard", &mut scaffold, 4);
    assert_eq!(payload["kind"], "user_input");
    assert!(payload["error"]
        .as_str()
        .unwrap()
        .contains("built-in scaffolders currently target Anchor"));

    let mut generate = env.sunscreen_in(&workspace);
    generate.args(["--json", "generate", "idl"]);
    let payload = env.json_err("pinocchio generate guard", &mut generate, 4);
    assert_eq!(payload["kind"], "user_input");
    assert!(payload["error"]
        .as_str()
        .unwrap()
        .contains("currently consumes Anchor IDLs"));
}

#[test]
fn chain_group_reports_missing_workspace_as_json_error() {
    let env = CliEnv::new();
    let empty = env.path("empty");
    std::fs::create_dir_all(&empty).unwrap();

    let mut cmd = env.sunscreen_in(&empty);
    cmd.args(["--json", "chain", "doctor"]);
    let payload = env.json_err("chain doctor outside workspace", &mut cmd, 5);
    assert_eq!(payload["kind"], "workspace_missing");
    assert_eq!(payload["exit_code"], 5);
    assert_eq!(
        payload["next_step"],
        "run `sunscreen init <name>` or change into a sunscreen workspace"
    );
}
