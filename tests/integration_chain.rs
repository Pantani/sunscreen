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
    assert_eq!(report["drift_count"], 0);

    let mut build = env.sunscreen_in(&nested);
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
    assert_eq!(events.last().unwrap()["status"], "ok");
    assert!(workspace.join("codama.json").exists());

    let log = env.fake_log();
    assert!(log.contains("anchor build"), "{log}");
    assert!(
        log.contains("pnpm exec codama run --all --config codama.json"),
        "{log}"
    );
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
}
