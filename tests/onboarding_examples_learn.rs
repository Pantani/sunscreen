#![cfg(feature = "onboarding")]

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

fn normalize_json_strings(value: &mut serde_json::Value, from: &str, to: &str) {
    match value {
        serde_json::Value::String(text) => {
            *text = text.replace(from, to);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_json_strings(item, from, to);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                normalize_json_strings(item, from, to);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

#[test]
fn examples_list_describe_and_use_are_embedded_and_deterministic() {
    let list = json_stdout(&["--json", "examples", "list"]);
    insta::assert_json_snapshot!("examples_list", list);

    let filtered = json_stdout(&["--json", "examples", "list", "--tag", "crud"]);
    insta::assert_json_snapshot!("examples_list_tag_crud", filtered);

    let describe = json_stdout(&["--json", "examples", "describe", "nft-collection"]);
    insta::assert_json_snapshot!("examples_describe_nft_collection", describe);

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
    let mut payload: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    normalize_json_strings(&mut payload, &dest.display().to_string(), "[EXAMPLE_DEST]");
    insta::assert_json_snapshot!("examples_use_blog_crud", payload);
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
    let mut payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    normalize_json_strings(
        &mut payload,
        &tmp.path().display().to_string(),
        "[EXAMPLE_DEST]",
    );
    insta::assert_json_snapshot!("examples_use_path_conflict_error", payload);
}

#[test]
fn learn_lists_and_renders_topics_from_embedded_markdown() {
    let list = json_stdout(&["--json", "learn"]);
    insta::assert_json_snapshot!("learn_list", list);

    let pda = json_stdout(&["--json", "learn", "pda"]);
    insta::assert_json_snapshot!("learn_pda", pda);

    let out = Command::new(sunscreen_bin())
        .args(["--json", "learn", "missing-topic"])
        .output()
        .expect("invoke learn");
    assert_eq!(out.status.code(), Some(4));
    let payload: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    insta::assert_json_snapshot!("learn_missing_topic_error", payload);
}
