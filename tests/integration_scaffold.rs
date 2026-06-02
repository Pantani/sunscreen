mod support;

use support::CliEnv;

#[test]
fn scaffold_group_runs_primitives_and_recipe_command_by_command() {
    let env = CliEnv::new();
    let workspace = env.chain_new("scaffold_walk", "vite");

    let mut account = env.sunscreen_in(&workspace);
    account.args([
        "--json",
        "scaffold",
        "account",
        "Vault",
        "--program",
        "scaffold_walk",
        "--fields",
        "owner:Pubkey,total:u64",
    ]);
    let account_report = env.json_ok("scaffold account", &mut account);
    assert_eq!(account_report["ok"], true);
    assert_eq!(account_report["noun"], "account");
    assert_eq!(account_report["name"], "Vault");
    assert_eq!(account_report["program"], "scaffold_walk");
    assert_eq!(account_report["account_file"], "created");
    assert_eq!(account_report["mod_file"], "created");
    assert_eq!(
        account_report["segments_patched"],
        serde_json::json!(["accounts"])
    );

    let mut event = env.sunscreen_in(&workspace);
    event.args([
        "--json",
        "scaffold",
        "event",
        "VaultDeposited",
        "--program",
        "scaffold_walk",
        "--fields",
        "amount:u64",
    ]);
    let event_report = env.json_ok("scaffold event", &mut event);
    assert_eq!(event_report["noun"], "event");
    assert_eq!(event_report["name"], "VaultDeposited");
    assert_eq!(event_report["events_file"], "created");
    assert_eq!(
        event_report["segments_patched"],
        serde_json::json!(["events"])
    );

    let mut error = env.sunscreen_in(&workspace);
    error.args([
        "--json",
        "scaffold",
        "error",
        "InsufficientFunds",
        "--program",
        "scaffold_walk",
        "--msg",
        "not enough lamports",
    ]);
    let error_report = env.json_ok("scaffold error", &mut error);
    assert_eq!(error_report["noun"], "error");
    assert_eq!(error_report["name"], "InsufficientFunds");
    assert_eq!(error_report["errors_file"], "created");
    assert_eq!(
        error_report["segments_patched"],
        serde_json::json!(["error_variants"])
    );

    let mut instruction = env.sunscreen_in(&workspace);
    instruction.args([
        "--json",
        "scaffold",
        "instruction",
        "deposit",
        "--program",
        "scaffold_walk",
        "--args",
        "amount:u64",
        "--accounts",
        "vault:mut,user:signer",
        "--emit",
        "VaultDeposited",
    ]);
    let instruction_report = env.json_ok("scaffold instruction", &mut instruction);
    assert_eq!(instruction_report["instruction"], "deposit");
    assert_eq!(instruction_report["program"], "scaffold_walk");
    assert_eq!(instruction_report["instruction_file"], "created");
    assert_eq!(instruction_report["mod_file"], "updated");
    assert_eq!(instruction_report["lib_file"], "updated");
    assert_eq!(instruction_report["lib_rs_patched"], true);

    let program_dir = workspace.join("programs/scaffold_walk/src");
    for rel in [
        "state/vault.rs",
        "events.rs",
        "errors.rs",
        "instructions/deposit.rs",
    ] {
        assert!(program_dir.join(rel).exists(), "missing {rel}");
    }
    let lib_rs = std::fs::read_to_string(program_dir.join("lib.rs")).unwrap();
    assert!(lib_rs.contains("pub fn deposit("));
    assert!(lib_rs.contains("pub mod events;"));
    assert!(lib_rs.contains("pub mod errors;"));
    assert!(lib_rs.contains("pub mod state;"));
    let vault_rs = std::fs::read_to_string(program_dir.join("state/vault.rs")).unwrap();
    assert!(vault_rs.contains("pub owner: Pubkey,"));
    assert!(vault_rs.contains("pub total: u64,"));
    let event_rs = std::fs::read_to_string(program_dir.join("events.rs")).unwrap();
    assert!(event_rs.contains("pub struct VaultDeposited"));
    assert!(event_rs.contains("pub amount: u64,"));
    let error_rs = std::fs::read_to_string(program_dir.join("errors.rs")).unwrap();
    assert!(error_rs.contains("InsufficientFunds,"));
    let deposit_rs = std::fs::read_to_string(program_dir.join("instructions/deposit.rs")).unwrap();
    assert!(deposit_rs.contains("emit!(VaultDeposited"));
    assert!(deposit_rs.contains("amount: u64"));

    let mut recipe = env.sunscreen_in(&workspace);
    recipe.args([
        "--json",
        "scaffold",
        "crud",
        "Post",
        "--program",
        "scaffold_walk",
        "--fields",
        "authority:Pubkey,title:String",
        "--no-events",
        "--no-frontend",
    ]);
    let recipe_report = env.json_ok("scaffold crud", &mut recipe);
    assert_eq!(recipe_report["ok"], true);
    assert_eq!(recipe_report["recipe"], "crud");
    assert_eq!(recipe_report["resource"], "post");
    assert_eq!(recipe_report["program"], "scaffold_walk");
    assert_eq!(recipe_report["dry_run"], false);
    assert_eq!(recipe_report["unchanged"], false);
    assert!(
        recipe_report["steps"].as_u64().unwrap() >= 4,
        "recipe should report multiple primitive steps: {recipe_report:#?}"
    );
    assert!(program_dir.join("state/post.rs").exists());
    assert!(program_dir.join("instructions/create_post.rs").exists());
    assert!(program_dir.join("instructions/read_post.rs").exists());
    assert!(program_dir.join("instructions/update_post.rs").exists());
    assert!(program_dir.join("instructions/delete_post.rs").exists());

    let mut rerun = env.sunscreen_in(&workspace);
    rerun.args([
        "--json",
        "scaffold",
        "account",
        "Vault",
        "--program",
        "scaffold_walk",
        "--fields",
        "owner:Pubkey,total:u64",
    ]);
    let rerun_report = env.json_ok("scaffold account rerun", &mut rerun);
    assert_eq!(rerun_report["unchanged"], true);
    assert_eq!(rerun_report["account_file"], "unchanged");
    assert_eq!(rerun_report["mod_file"], "unchanged");
    assert_eq!(rerun_report["segments_patched"], serde_json::json!([]));
}

#[test]
fn scaffold_group_reports_invalid_field_as_json_error() {
    let env = CliEnv::new();
    let workspace = env.chain_new("scaffold_error_walk", "none");

    let mut cmd = env.sunscreen_in(&workspace);
    cmd.args([
        "--json",
        "scaffold",
        "account",
        "Broken",
        "--program",
        "scaffold_error_walk",
        "--fields",
        "badfield",
    ]);
    let payload = env.json_err("scaffold invalid field", &mut cmd, 4);
    assert_eq!(payload["kind"], "user_input");
    assert_eq!(payload["exit_code"], 4);
    assert!(payload["error"]
        .as_str()
        .unwrap()
        .contains("invalid --fields entry"));
}

#[test]
fn scaffold_group_covers_program_and_builtin_recipes() {
    let env = CliEnv::new();
    let workspace = env.chain_new("asset_walk", "none");
    let program_dir = workspace.join("programs/asset_walk/src");

    let mut program = env.sunscreen_in(&workspace);
    program.args(["--json", "scaffold", "program", "Rewards"]);
    let program_report = env.json_ok("scaffold program", &mut program);
    assert_eq!(program_report["ok"], true);
    assert_eq!(program_report["noun"], "program");
    assert_eq!(program_report["name"], "rewards");
    assert_eq!(program_report["anchor_toml_patched"], true);
    assert!(workspace.join("programs/rewards/src/lib.rs").exists());

    let mut dry = env.sunscreen_in(&workspace);
    dry.args([
        "--json",
        "scaffold",
        "spl-token",
        "PreviewVault",
        "--program",
        "asset_walk",
        "--dry-run",
    ]);
    let dry_report = env.json_ok("scaffold spl-token dry-run", &mut dry);
    assert_eq!(dry_report["recipe"], "spl-token");
    assert_eq!(dry_report["resource"], "preview_vault");
    assert_eq!(dry_report["dry_run"], true);
    assert_eq!(dry_report["written"], 0);
    assert!(!program_dir.join("state/preview_vault.rs").exists());

    let mut spl = env.sunscreen_in(&workspace);
    spl.args([
        "--json",
        "scaffold",
        "spl-token",
        "TokenVault",
        "--program",
        "asset_walk",
    ]);
    let spl_report = env.json_ok("scaffold spl-token", &mut spl);
    assert_eq!(spl_report["recipe"], "spl-token");
    assert_eq!(spl_report["resource"], "token_vault");
    assert_eq!(spl_report["program"], "asset_walk");
    assert_eq!(spl_report["unchanged"], false);
    assert!(program_dir.join("state/token_vault.rs").exists());
    for ix in [
        "initialize_token_vault",
        "mint_token_vault",
        "transfer_token_vault",
    ] {
        assert!(program_dir.join(format!("instructions/{ix}.rs")).exists());
    }

    let mut nft = env.sunscreen_in(&workspace);
    nft.args([
        "--json",
        "scaffold",
        "metaplex-nft",
        "NftCollection",
        "--program",
        "asset_walk",
    ]);
    let nft_report = env.json_ok("scaffold metaplex-nft", &mut nft);
    assert_eq!(nft_report["recipe"], "metaplex-nft");
    assert_eq!(nft_report["resource"], "nft_collection");
    assert_eq!(nft_report["program"], "asset_walk");
    assert!(program_dir.join("state/nft_collection.rs").exists());
    for ix in [
        "create_nft_collection",
        "mint_nft_collection",
        "verify_nft_collection",
    ] {
        assert!(program_dir.join(format!("instructions/{ix}.rs")).exists());
    }

    let mut spl_again = env.sunscreen_in(&workspace);
    spl_again.args([
        "--json",
        "scaffold",
        "spl-token",
        "TokenVault",
        "--program",
        "asset_walk",
    ]);
    let spl_again_report = env.json_ok("scaffold spl-token rerun", &mut spl_again);
    assert_eq!(spl_again_report["unchanged"], true);
}
