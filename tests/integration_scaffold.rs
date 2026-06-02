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
    env.json_ok("scaffold event", &mut event);

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
    env.json_ok("scaffold error", &mut error);

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
    env.json_ok("scaffold instruction", &mut instruction);

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
    assert_eq!(recipe_report["recipe"], "crud");
    assert!(program_dir.join("state/post.rs").exists());
    assert!(program_dir.join("instructions/create_post.rs").exists());

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
}
