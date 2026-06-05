#![cfg(feature = "onboarding")]

use std::path::Path;
use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

fn run_ok(args: &[&str], path_arg: Option<&Path>) -> std::process::Output {
    let mut cmd = Command::new(sunscreen_bin());
    cmd.env("SUNSCREEN_SKIP_PREFLIGHT", "1").args(args);
    if let Some(path) = path_arg {
        cmd.arg(path);
    }
    let out = cmd.output().expect("invoke sunscreen");
    assert!(
        out.status.success(),
        "exit={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

#[test]
fn init_non_interactive_reuses_chain_new_workspace_path() {
    let tmp = tempfile::tempdir().unwrap();
    let chain_path = tmp.path().join("chain");
    let init_path = tmp.path().join("init");

    run_ok(
        &["chain", "new", "demo_app", "--frontend", "none", "--path"],
        Some(&chain_path),
    );
    let out = run_ok(
        &[
            "--json",
            "init",
            "demo_app",
            "--non-interactive",
            "--from-preset",
            "empty",
            "--frontend",
            "none",
            "--path",
        ],
        Some(&init_path),
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["command"], "init");
    assert_eq!(payload["project"], "demo_app");

    for rel in [
        "Anchor.toml",
        "Cargo.toml",
        "sunscreen.yml",
        "programs/demo_app/src/lib.rs",
        "programs/demo_app/src/instructions/mod.rs",
    ] {
        let chain = std::fs::read_to_string(chain_path.join(rel)).unwrap();
        let init = std::fs::read_to_string(init_path.join(rel)).unwrap();
        assert_eq!(chain, init, "{rel}");
    }
}

#[test]
fn init_non_interactive_requires_name_and_dry_run_writes_nothing() {
    let out = Command::new(sunscreen_bin())
        .args(["--json", "init", "--non-interactive"])
        .output()
        .expect("invoke init");
    assert_eq!(out.status.code(), Some(4));
    let payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(payload["kind"], "user_input");

    let tmp = tempfile::tempdir().unwrap();
    let dry_dest = tmp.path().join("dry_plan");
    run_ok(
        &[
            "init",
            "dry_plan",
            "--non-interactive",
            "--frontend",
            "none",
            "--path",
        ],
        Some(&dry_dest),
    );
    assert!(dry_dest.exists());

    let real_dry_dest = tmp.path().join("dry_run");
    run_ok(
        &[
            "init",
            "dry_run",
            "--non-interactive",
            "--frontend",
            "none",
            "--dry-run",
            "--path",
        ],
        Some(&real_dry_dest),
    );
    assert!(!real_dry_dest.exists());
}

#[test]
fn init_from_preset_applies_corresponding_recipe() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("blog_init");
    let out = run_ok(
        &[
            "--json",
            "init",
            "blog_init",
            "--non-interactive",
            "--from-preset",
            "blog",
            "--frontend",
            "none",
            "--path",
        ],
        Some(&dest),
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["preset"], "blog");
    assert_eq!(payload["preset_applied"], true);
    assert!(dest.join("programs/blog_init/src/state/post.rs").exists());
    assert!(dest
        .join("programs/blog_init/src/instructions/create_post.rs")
        .exists());
}

#[test]
fn quickstart_recipes_compose_phase_5_scaffolders() {
    let tmp = tempfile::tempdir().unwrap();
    for (recipe, name, expected) in [
        (
            "token",
            "token_app",
            "programs/token_app/src/state/token_vault.rs",
        ),
        (
            "nft",
            "nft_app",
            "programs/nft_app/src/state/nft_collection.rs",
        ),
        ("dao", "dao_app", "programs/dao_app/src/state/proposal.rs"),
        ("blog", "blog_app", "programs/blog_app/src/state/post.rs"),
    ] {
        let dest = tmp.path().join(name);
        let out = run_ok(
            &[
                "--json",
                "quickstart",
                recipe,
                "--name",
                name,
                "--cluster",
                "localnet",
                "--non-interactive",
                "--frontend",
                "none",
                "--path",
            ],
            Some(&dest),
        );
        let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(payload["command"], "quickstart");
        assert_eq!(payload["recipe"], recipe);
        assert!(dest
            .join(
                expected
                    .strip_prefix(&format!("{name}/"))
                    .unwrap_or(expected)
            )
            .exists());
        assert!(dest.join(format!("programs/{name}/src/lib.rs")).exists());
    }
}

#[test]
fn quickstart_devnet_next_steps_build_before_deploy() {
    let out = run_ok(
        &[
            "--json",
            "quickstart",
            "nft",
            "--name",
            "my-first-nft",
            "--cluster",
            "devnet",
            "--non-interactive",
            "--frontend",
            "none",
            "--dry-run",
        ],
        None,
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let next_steps = payload["next_steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step.as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(next_steps[0], "cd my-first-nft");
    assert_eq!(next_steps[1], "sunscreen chain build --headless");
    assert_eq!(
        next_steps[2],
        "sunscreen wallet new --out ~/.config/solana/id.json"
    );
    assert_eq!(
        next_steps[3],
        "sunscreen deploy devnet --program my_first_nft"
    );
    assert_eq!(next_steps[4], "sunscreen chain serve --headless");
}

#[test]
fn quickstart_reports_path_conflict_as_exit_7() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("occupied.txt"), "busy").unwrap();
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args([
            "--json",
            "quickstart",
            "nft",
            "--name",
            "nft_app",
            "--non-interactive",
            "--path",
        ])
        .arg(tmp.path())
        .output()
        .expect("invoke quickstart");
    assert_eq!(out.status.code(), Some(7));
    let payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(payload["kind"], "path_conflict");
}
