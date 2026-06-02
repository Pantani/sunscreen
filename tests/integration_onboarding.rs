#![cfg(feature = "onboarding")]

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
    assert_eq!(examples_report["ok"], true);
    let examples = examples_report["examples"].as_array().unwrap();
    let example_names = examples
        .iter()
        .map(|example| example["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        example_names,
        [
            "token-faucet",
            "nft-collection",
            "escrow",
            "voting-dao",
            "blog-crud"
        ]
    );

    let mut describe = env.sunscreen();
    describe.args(["--json", "examples", "describe", "blog-crud"]);
    let describe_report = env.json_ok("examples describe", &mut describe);
    assert_eq!(describe_report["ok"], true);
    assert_eq!(describe_report["example"]["name"], "blog-crud");
    assert!(describe_report["readme"]
        .as_str()
        .unwrap()
        .contains("sunscreen quickstart blog"));

    let example_dest = env.path("dry-example");
    let mut example_dry = env.sunscreen();
    example_dry
        .args(["--json", "examples", "use", "blog-crud", "--dry-run"])
        .arg(&example_dest);
    let example_dry_report = env.json_ok("examples use dry-run", &mut example_dry);
    assert_eq!(example_dry_report["command"], "examples_use");
    assert_eq!(example_dry_report["dry_run"], true);
    assert_eq!(example_dry_report["written"], 0);
    assert!(!example_dest.exists());

    let mut learn = env.sunscreen();
    learn.args(["--json", "learn"]);
    let learn_list = env.json_ok("learn list", &mut learn);
    let topics = learn_list["topics"].as_array().unwrap();
    assert_eq!(topics.len(), 5);
    assert!(topics
        .iter()
        .any(|topic| topic["topic"] == "anchor-vs-native"));

    let mut learn = env.sunscreen();
    learn.args(["--json", "learn", "pda"]);
    let learn_report = env.json_ok("learn pda", &mut learn);
    assert_eq!(learn_report["ok"], true);
    assert_eq!(learn_report["topic"], "pda");
    assert!(learn_report["body"].as_str().unwrap().contains("PDA"));

    let mut missing_learn = env.sunscreen();
    missing_learn.args(["--json", "learn", "missing-topic"]);
    let missing_learn_report = env.json_err("learn missing topic", &mut missing_learn, 4);
    assert_eq!(missing_learn_report["kind"], "user_input");

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
    assert_eq!(quickstart_report["ok"], true);
    assert_eq!(quickstart_report["command"], "quickstart");
    assert_eq!(quickstart_report["recipe"], "nft");
    assert_eq!(quickstart_report["resource"], "NftCollection");
    assert_eq!(quickstart_report["cluster"], "devnet");
    assert_eq!(quickstart_report["frontend"], "none");
    assert_eq!(quickstart_report["dry_run"], false);
    assert!(quickstart_report["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op.as_str().unwrap().contains("scaffold metaplex-nft")));
    assert!(quickstart_report["next_steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step.as_str().unwrap().contains("sunscreen wallet new")));
    assert!(workspace
        .join("programs/onboard_walk/src/state/nft_collection.rs")
        .exists());

    let wallet = env.path("wallet.json");
    let mut wallet_new = env.sunscreen_fake_only_in(&workspace);
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
    assert_eq!(wallet_report["command"], "wallet_new");
    assert_eq!(wallet_report["dry_run"], false);
    assert!(wallet_report["stdout"]
        .as_str()
        .unwrap()
        .contains("Fake111111111111111111111111111111111111111"));
    assert_eq!(wallet_report["stderr"], "");
    assert!(wallet.exists());

    let mut named_wallet = env.sunscreen_fake_only_in(&workspace);
    named_wallet.args([
        "--json",
        "wallet",
        "new",
        "treasury",
        "--no-bip39-passphrase",
    ]);
    let named_wallet_report = env.json_ok("wallet new named", &mut named_wallet);
    assert_eq!(named_wallet_report["command"], "wallet_new");
    assert!(workspace.join(".sunscreen/wallets/treasury.json").exists());

    let mut wallet_list = env.sunscreen_in(&workspace);
    wallet_list.args(["--json", "wallet", "list"]);
    let wallet_list_report = env.json_ok("wallet list", &mut wallet_list);
    let wallet_names = wallet_list_report["wallets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|wallet| wallet["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(wallet_names.contains(&"solana-default"));
    assert!(wallet_names.contains(&"treasury"));

    let mut set_default = env.sunscreen_in(&workspace);
    set_default.args([
        "--json",
        "wallet",
        "set-default",
        "treasury",
        "--cluster",
        "devnet",
    ]);
    let set_default_report = env.json_ok("wallet set-default", &mut set_default);
    assert_eq!(set_default_report["command"], "wallet_set_default");
    assert_eq!(set_default_report["cluster"], "devnet");
    assert!(set_default_report["wallet"]
        .as_str()
        .unwrap()
        .ends_with(".sunscreen/wallets/treasury.json"));

    let mut balance = env.sunscreen_fake_only_in(&workspace);
    balance.args([
        "--json",
        "wallet",
        "balance",
        "--cluster",
        "devnet",
        "FakePubkey",
    ]);
    let balance_report = env.json_ok("wallet balance", &mut balance);
    assert_eq!(balance_report["command"], "wallet_balance");
    assert_eq!(balance_report["cluster"], "devnet");
    assert_eq!(balance_report["stdout"], "1 SOL\n");

    let mut airdrop_dry = env.sunscreen_fake_only_in(&workspace);
    airdrop_dry.args([
        "--json",
        "wallet",
        "airdrop",
        "2",
        "--cluster",
        "devnet",
        "--to",
        "FakePubkey",
        "--dry-run",
    ]);
    let airdrop_dry_report = env.json_ok("wallet airdrop dry-run", &mut airdrop_dry);
    assert_eq!(airdrop_dry_report["command"], "wallet_airdrop");
    assert_eq!(airdrop_dry_report["dry_run"], true);
    assert_eq!(
        airdrop_dry_report["argv"],
        serde_json::json!([
            "solana",
            "airdrop",
            "2",
            "--url",
            "https://api.devnet.solana.com",
            "FakePubkey"
        ])
    );

    let mut deploy_mainnet_guard = env.sunscreen_in(&workspace);
    deploy_mainnet_guard.args(["--json", "deploy", "mainnet", "--program", "onboard_walk"]);
    let deploy_mainnet_error = env.json_err(
        "deploy mainnet requires confirmation",
        &mut deploy_mainnet_guard,
        4,
    );
    assert_eq!(deploy_mainnet_error["kind"], "user_input");

    let mut deploy_verify_guard = env.sunscreen_in(&workspace);
    deploy_verify_guard.args(["--json", "deploy", "devnet", "--verify"]);
    let deploy_verify_error = env.json_err(
        "deploy verify requires program",
        &mut deploy_verify_guard,
        4,
    );
    assert_eq!(deploy_verify_error["kind"], "user_input");

    let mut deploy_dry = env.sunscreen_in(&workspace);
    deploy_dry.args([
        "--json",
        "deploy",
        "mainnet",
        "--program",
        "onboard_walk",
        "--yes-i-understand-cost",
        "--dry-run",
    ]);
    let deploy_dry_report = env.json_ok("deploy mainnet dry-run", &mut deploy_dry);
    assert_eq!(deploy_dry_report["dry_run"], true);
    assert_eq!(
        deploy_dry_report["argv"],
        serde_json::json!([
            "anchor",
            "deploy",
            "--provider.cluster",
            "mainnet",
            "--program-name",
            "onboard_walk"
        ])
    );

    let mut deploy = env.sunscreen_fake_only_in(&workspace);
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
    assert_eq!(deploy_report["command"], "deploy");
    assert_eq!(deploy_report["target"], "devnet");
    assert_eq!(deploy_report["program"], "onboard_walk");
    assert_eq!(deploy_report["verify"], true);
    assert!(deploy_report["deploy"]["stdout"]
        .as_str()
        .unwrap()
        .contains("fake anchor deploy"));
    assert!(deploy_report["verify_output"]["stdout"]
        .as_str()
        .unwrap()
        .contains("fake anchor verify onboard_walk"));

    let log_lines = env.fake_log_lines();
    assert!(log_lines
        .iter()
        .any(|line| line.starts_with("solana-keygen new --outfile ")));
    assert!(log_lines
        .iter()
        .any(|line| line == "solana balance --url https://api.devnet.solana.com FakePubkey"));
    assert!(log_lines.iter().any(|line| {
        line == "anchor deploy --provider.cluster devnet --program-name onboard_walk"
    }));
    assert!(log_lines
        .iter()
        .any(|line| line == "anchor verify onboard_walk --provider.cluster devnet"));
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
    assert!(payload["error"]
        .as_str()
        .unwrap()
        .contains("destination already exists and is not empty"));
}
