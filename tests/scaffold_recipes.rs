//! End-to-end tests for Phase 5 composite scaffold recipes.

use std::path::Path;
use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

fn run_chain_new(out_path: &Path, name: &str, frontend: &str) {
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(["chain", "new", name, "--frontend", frontend, "--path"])
        .arg(out_path)
        .output()
        .expect("invoke sunscreen chain new");
    assert!(
        out.status.success(),
        "chain new failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run_scaffold(workspace: &Path, args: &[&str]) -> std::process::Output {
    Command::new(sunscreen_bin())
        .current_dir(workspace)
        .env_remove("SUNSCREEN_SKIP_PREFLIGHT")
        .args(args)
        .output()
        .expect("invoke sunscreen scaffold")
}

fn discover_program(ws: &Path) -> String {
    let programs_dir = ws.join("programs");
    let entries: Vec<_> = std::fs::read_dir(&programs_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "expected exactly one program dir");
    entries[0].file_name().to_string_lossy().into_owned()
}

fn generated_corpus(root: &Path, paths: &[&str]) -> String {
    let mut out = String::new();
    for rel in paths {
        let contents = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|err| panic!("read generated file {rel}: {err}"));
        out.push_str("=== ");
        out.push_str(rel);
        out.push_str(" ===\n");
        out.push_str(&contents);
        if !contents.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn assert_recipe_snapshot(name: &str, contents: String) {
    insta::with_settings!({ snapshot_path => "golden/snapshots" }, {
        insta::assert_snapshot!(name, contents);
    });
}

#[test]
fn scaffold_crud_composes_resource_slice_and_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("blog_app");
    run_chain_new(&ws, "blog_app", "none");
    let program_name = discover_program(&ws);

    let out = run_scaffold(
        &ws,
        &[
            "scaffold",
            "crud",
            "Post",
            "--program",
            &program_name,
            "--fields",
            "authority:Pubkey,title:String,body:String",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "scaffold crud failed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not valid JSON ({e}): {stdout}"));
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["recipe"], "crud");
    assert_eq!(payload["resource"], "post");
    assert_eq!(payload["unchanged"], false);

    let program_dir = ws.join("programs").join(&program_name);
    let state = std::fs::read_to_string(program_dir.join("src/state/post.rs")).unwrap();
    assert!(state.contains("pub struct Post"));
    assert!(state.contains("pub authority: Pubkey,"));
    assert!(state.contains("pub title: String,"));

    for ix in ["create_post", "read_post", "update_post", "delete_post"] {
        assert!(
            program_dir
                .join("src/instructions")
                .join(format!("{ix}.rs"))
                .exists(),
            "missing instruction {ix}"
        );
    }
    let lib_rs = std::fs::read_to_string(program_dir.join("src/lib.rs")).unwrap();
    for wrapper in ["create_post", "read_post", "update_post", "delete_post"] {
        assert!(lib_rs.contains(&format!("pub fn {wrapper}(")));
    }

    let events = std::fs::read_to_string(program_dir.join("src/events.rs")).unwrap();
    for event in ["PostCreated", "PostUpdated", "PostDeleted"] {
        assert!(events.contains(&format!("pub struct {event}")));
    }
    let errors = std::fs::read_to_string(program_dir.join("src/errors.rs")).unwrap();
    assert!(errors.contains("PostNotFound,"));
    assert!(errors.contains("PostUnauthorized,"));

    let recipe_test = ws.join("tests").join(&program_name).join("post.test.ts");
    assert!(recipe_test.exists(), "recipe TS test missing");
    let recipe_test_contents = std::fs::read_to_string(&recipe_test).unwrap();
    assert!(recipe_test_contents.contains("createPost"));
    assert!(recipe_test_contents.contains("deletePost"));
    assert_recipe_snapshot(
        "scaffold_recipes_crud_full_slice",
        generated_corpus(
            &ws,
            &[
                "programs/blog_app/src/state/post.rs",
                "programs/blog_app/src/instructions/create_post.rs",
                "programs/blog_app/src/instructions/read_post.rs",
                "programs/blog_app/src/instructions/update_post.rs",
                "programs/blog_app/src/instructions/delete_post.rs",
                "programs/blog_app/src/events.rs",
                "programs/blog_app/src/errors.rs",
                "programs/blog_app/src/lib.rs",
                "tests/blog_app/post.test.ts",
            ],
        ),
    );

    let again = run_scaffold(
        &ws,
        &[
            "scaffold",
            "crud",
            "Post",
            "--program",
            &program_name,
            "--fields",
            "authority:Pubkey,title:String,body:String",
            "--json",
        ],
    );
    assert_eq!(
        again.status.code(),
        Some(0),
        "re-run should be idempotent; stderr={}",
        String::from_utf8_lossy(&again.stderr)
    );
    let stdout = String::from_utf8_lossy(&again.stdout);
    let payload: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not valid JSON ({e}): {stdout}"));
    assert_eq!(payload["unchanged"], true);
}

#[test]
fn scaffold_crud_writes_frontend_hook_and_honors_feature_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("blog_frontend_app");
    run_chain_new(&ws, "blog_frontend_app", "vite");
    let program_name = discover_program(&ws);

    let out = run_scaffold(
        &ws,
        &[
            "scaffold",
            "crud",
            "Post",
            "--program",
            &program_name,
            "--fields",
            "authority:Pubkey,title:String",
            "--no-delete",
            "--no-events",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "scaffold crud failed: exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    let program_dir = ws.join("programs").join(&program_name);
    assert!(program_dir.join("src/instructions/create_post.rs").exists());
    assert!(program_dir.join("src/instructions/read_post.rs").exists());
    assert!(program_dir.join("src/instructions/update_post.rs").exists());
    assert!(!program_dir.join("src/instructions/delete_post.rs").exists());
    assert!(
        !program_dir.join("src/events.rs").exists(),
        "--no-events should skip event scaffolding"
    );

    let hook = ws.join("app/src/hooks/use-post.ts");
    assert!(hook.exists(), "frontend recipe hook missing");
    let hook_contents = std::fs::read_to_string(&hook).unwrap();
    assert!(hook_contents.contains("usePost"));
    assert!(hook_contents.contains("useCreatePost"));
    assert!(hook_contents.contains("useUpdatePost"));
    assert!(!hook_contents.contains("useDeletePost"));
    assert_recipe_snapshot(
        "scaffold_recipes_crud_frontend_no_delete_no_events",
        generated_corpus(
            &ws,
            &[
                "programs/blog_frontend_app/src/state/post.rs",
                "programs/blog_frontend_app/src/instructions/create_post.rs",
                "programs/blog_frontend_app/src/instructions/read_post.rs",
                "programs/blog_frontend_app/src/instructions/update_post.rs",
                "programs/blog_frontend_app/src/errors.rs",
                "programs/blog_frontend_app/src/lib.rs",
                "app/src/hooks/use-post.ts",
            ],
        ),
    );
}

#[test]
fn scaffold_spl_token_and_metaplex_nft_create_recipe_slices() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("asset_app");
    run_chain_new(&ws, "asset_app", "none");
    let program_name = discover_program(&ws);

    let spl = run_scaffold(
        &ws,
        &[
            "scaffold",
            "spl-token",
            "TokenVault",
            "--program",
            &program_name,
            "--json",
        ],
    );
    assert!(
        spl.status.success(),
        "scaffold spl-token failed: exit={:?} stderr={}",
        spl.status.code(),
        String::from_utf8_lossy(&spl.stderr)
    );

    let nft = run_scaffold(
        &ws,
        &[
            "scaffold",
            "metaplex-nft",
            "NftCollection",
            "--program",
            &program_name,
            "--json",
        ],
    );
    assert!(
        nft.status.success(),
        "scaffold metaplex-nft failed: exit={:?} stderr={}",
        nft.status.code(),
        String::from_utf8_lossy(&nft.stderr)
    );

    let program_dir = ws.join("programs").join(&program_name);
    assert!(program_dir.join("src/state/token_vault.rs").exists());
    assert!(program_dir.join("src/state/nft_collection.rs").exists());

    for ix in [
        "initialize_token_vault",
        "mint_token_vault",
        "transfer_token_vault",
        "create_nft_collection",
        "mint_nft_collection",
        "verify_nft_collection",
    ] {
        assert!(
            program_dir
                .join("src/instructions")
                .join(format!("{ix}.rs"))
                .exists(),
            "missing recipe instruction {ix}"
        );
    }

    let events = std::fs::read_to_string(program_dir.join("src/events.rs")).unwrap();
    for event in [
        "TokenVaultInitialized",
        "TokenVaultMinted",
        "NftCollectionCreated",
        "NftCollectionMinted",
    ] {
        assert!(events.contains(&format!("pub struct {event}")));
    }
    let errors = std::fs::read_to_string(program_dir.join("src/errors.rs")).unwrap();
    for variant in [
        "InvalidMint",
        "TokenVaultUnauthorized",
        "InvalidMetadata",
        "NftCollectionUnauthorized",
    ] {
        assert!(errors.contains(&format!("{variant},")));
    }
    assert_recipe_snapshot(
        "scaffold_recipes_spl_token_and_metaplex_nft",
        generated_corpus(
            &ws,
            &[
                "programs/asset_app/src/state/token_vault.rs",
                "programs/asset_app/src/state/nft_collection.rs",
                "programs/asset_app/src/instructions/initialize_token_vault.rs",
                "programs/asset_app/src/instructions/mint_token_vault.rs",
                "programs/asset_app/src/instructions/transfer_token_vault.rs",
                "programs/asset_app/src/instructions/create_nft_collection.rs",
                "programs/asset_app/src/instructions/mint_nft_collection.rs",
                "programs/asset_app/src/instructions/verify_nft_collection.rs",
                "programs/asset_app/src/events.rs",
                "programs/asset_app/src/errors.rs",
                "programs/asset_app/src/lib.rs",
            ],
        ),
    );
}

#[test]
fn scaffold_recipe_invalid_field_exit_4() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run_scaffold(
        tmp.path(),
        &[
            "scaffold",
            "crud",
            "Post",
            "--program",
            "any",
            "--fields",
            "noType",
        ],
    );
    assert_eq!(out.status.code(), Some(4));
    assert_recipe_snapshot(
        "scaffold_recipes_invalid_field_error",
        String::from_utf8_lossy(&out.stderr).into_owned(),
    );
}
