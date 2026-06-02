//! Unit tests for the Phase 4 Codama config wrapper.

use std::path::Path;

use sunscreen::codegen::codama::infer_idl_stem;
use sunscreen::codegen::codama_config::{render_codama_config_json, write_codama_config};

#[test]
fn codama_config_serializes_stable_js_client_script() {
    let rendered = render_codama_config_json("demo-app", "demo_app").expect("render codama config");
    let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");

    insta::assert_json_snapshot!(value);
    assert!(rendered.ends_with('\n'));
}

#[test]
fn write_codama_config_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let first =
        write_codama_config(tmp.path(), "demo-app", "demo_app").expect("first codama config write");
    let second = write_codama_config(tmp.path(), "demo-app", "demo_app")
        .expect("second codama config write");

    assert!(first.changed);
    assert!(!second.changed);
    assert_eq!(first.path, tmp.path().join("codama.json"));
    assert_eq!(second.path, first.path);
    let content = std::fs::read_to_string(tmp.path().join("codama.json")).unwrap();
    insta::assert_snapshot!(content);
}

#[test]
fn write_codama_config_rejects_missing_workspace_directory() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("missing");

    let err = write_codama_config(&missing, "demo-app", "demo_app")
        .expect_err("missing workspace must fail");

    let rendered = err.to_string().replace(
        &missing.join("codama.json").display().to_string(),
        "<workspace>/codama.json",
    );
    insta::assert_snapshot!(rendered);
    assert!(!Path::new(&missing).exists());
}

#[test]
fn infer_idl_stem_normalizes_target_idl_filename() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let idl_dir = tmp.path().join("target/idl");
    std::fs::create_dir_all(&idl_dir).expect("create idl dir");
    std::fs::write(idl_dir.join("DemoApp.json"), "{}\n").expect("write idl");

    let stem = infer_idl_stem(tmp.path(), None).expect("infer idl stem");

    assert_eq!(stem, "demo_app");
}
