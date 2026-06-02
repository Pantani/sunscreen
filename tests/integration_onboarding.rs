mod support;

use support::CliEnv;

#[test]
fn onboarding_group_runs_examples_learn_quickstart_wallet_and_deploy() {
    let env = CliEnv::new();
    env.install_fake_anchor_success();
    env.install_fake_solana_success();

    let mut examples = env.sunscreen();
    examples.args(["--json", "examples", "list"]);
    let examples_report = env.json_ok("examples list", &mut examples);
    assert!(examples_report["examples"].as_array().unwrap().len() >= 3);

    let mut describe = env.sunscreen();
    describe.args(["--json", "examples", "describe", "blog-crud"]);
    let describe_report = env.json_ok("examples describe", &mut describe);
    assert_eq!(describe_report["example"]["name"], "blog-crud");

    let mut learn = env.sunscreen();
    learn.args(["--json", "learn", "pda"]);
    let learn_report = env.json_ok("learn pda", &mut learn);
    assert_eq!(learn_report["topic"], "pda");

    let workspace = env.path("onboard_walk");
    let mut quickstart = env.sunscreen();
    quickstart
        .args([
            "--json",
            "quickstart",
            "nft",
            "--name",
            "onboard_walk",
            "--cluster",
            "devnet",
            "--non-interactive",
            "--frontend",
            "none",
            "--path",
        ])
        .arg(&workspace);
    let quickstart_report = env.json_ok("quickstart nft", &mut quickstart);
    assert_eq!(quickstart_report["command"], "quickstart");
    assert_eq!(quickstart_report["recipe"], "nft");
    assert!(workspace
        .join("programs/onboard_walk/src/state/nft_collection.rs")
        .exists());

    let wallet = env.path("wallet.json");
    let mut wallet_new = env.sunscreen_in(&workspace);
    wallet_new.args([
        "--json",
        "wallet",
        "new",
        "--out",
        wallet.to_str().unwrap(),
        "--no-bip39-passphrase",
    ]);
    let wallet_report = env.json_ok("wallet new", &mut wallet_new);
    assert_eq!(wallet_report["ok"], true);
    assert!(wallet.exists());

    let mut deploy = env.sunscreen_in(&workspace);
    deploy.args([
        "--json",
        "deploy",
        "devnet",
        "--program",
        "onboard_walk",
        "--verify",
    ]);
    let deploy_report = env.json_ok("deploy devnet", &mut deploy);
    assert_eq!(deploy_report["ok"], true);

    let log = env.fake_log();
    assert!(log.contains("solana-keygen new --outfile"), "{log}");
    assert!(log.contains("anchor deploy"), "{log}");
    assert!(log.contains("anchor verify onboard_walk"), "{log}");
}

#[test]
fn onboarding_group_keeps_path_conflict_exit_code_stable() {
    let env = CliEnv::new();
    let occupied = env.path("occupied");
    std::fs::create_dir_all(&occupied).unwrap();
    std::fs::write(occupied.join("busy.txt"), "busy").unwrap();

    let mut cmd = env.sunscreen();
    cmd.args([
        "--json",
        "quickstart",
        "blog",
        "--name",
        "occupied",
        "--non-interactive",
        "--path",
    ])
    .arg(&occupied);
    let payload = env.json_err("quickstart path conflict", &mut cmd, 7);
    assert_eq!(payload["kind"], "path_conflict");
    assert_eq!(payload["exit_code"], 7);
}
