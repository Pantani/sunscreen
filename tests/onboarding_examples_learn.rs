use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

fn json_stdout(args: &[&str]) -> serde_json::Value {
    let out = Command::new(sunscreen_bin())
        .args(args)
        .output()
        .expect("invoke sunscreen");
    assert!(
        out.status.success(),
        "exit={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("stdout json")
}

#[test]
fn examples_list_describe_and_use_are_embedded_and_deterministic() {
    let list = json_stdout(&["--json", "examples", "list"]);
    let examples = list["examples"].as_array().expect("examples array");
    assert!(examples.len() >= 5);
    assert!(examples.iter().any(|item| item["name"] == "nft-collection"));

    let filtered = json_stdout(&["--json", "examples", "list", "--tag", "crud"]);
    let filtered_examples = filtered["examples"].as_array().unwrap();
    assert!(filtered_examples
        .iter()
        .any(|item| item["name"] == "blog-crud"));
    assert!(filtered_examples.iter().all(|item| item["tags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tag| tag == "crud")));

    let describe = json_stdout(&["--json", "examples", "describe", "nft-collection"]);
    assert_eq!(describe["example"]["name"], "nft-collection");
    assert!(describe["readme"]
        .as_str()
        .unwrap()
        .contains("Metaplex NFT"));

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("copied");
    let out = Command::new(sunscreen_bin())
        .env("SUNSCREEN_SKIP_PREFLIGHT", "1")
        .args(["--json", "examples", "use", "blog-crud"])
        .arg(&dest)
        .output()
        .expect("invoke examples use");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(payload["example"], "blog-crud");
    assert_eq!(
        payload["next_step"],
        format!("cd {} && sunscreen chain serve --headless", dest.display())
    );
    assert!(dest.join("README.md").exists());
    assert!(dest.join("sunscreen.yml").exists());
    assert!(dest.join("Anchor.toml").exists());
    assert!(dest.join("programs/blog_crud/src/state/post.rs").exists());
    assert!(dest.join(".sunscreen/example/sunscreen.example").exists());
}

#[test]
fn examples_use_reports_path_conflict_as_exit_7() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("already.txt"), "occupied").unwrap();
    let out = Command::new(sunscreen_bin())
        .args(["--json", "examples", "use", "blog-crud"])
        .arg(tmp.path())
        .output()
        .expect("invoke examples use");
    assert_eq!(out.status.code(), Some(7));
    let payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(payload["kind"], "path_conflict");
    assert_eq!(payload["exit_code"], 7);
}

#[test]
fn learn_lists_and_renders_topics_from_embedded_markdown() {
    let list = json_stdout(&["--json", "learn"]);
    let topics = list["topics"].as_array().expect("topics");
    assert_eq!(topics.len(), 5);
    assert!(topics.iter().any(|topic| topic["topic"] == "pda"));

    let pda = json_stdout(&["--json", "learn", "pda"]);
    assert_eq!(pda["topic"], "pda");
    assert_eq!(pda["title"], "Program Derived Addresses");
    assert!(pda["body"]
        .as_str()
        .unwrap()
        .contains("deterministic seeds"));

    let out = Command::new(sunscreen_bin())
        .args(["--json", "learn", "missing-topic"])
        .output()
        .expect("invoke learn");
    assert_eq!(out.status.code(), Some(4));
    let payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(payload["kind"], "user_input");
}
