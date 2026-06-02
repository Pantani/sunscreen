//! Unit tests for the Phase 4 Codama config wrapper.

use std::path::Path;

use sunscreen::codegen::codama_config::{render_codama_config_json, write_codama_config};

#[test]
fn codama_config_serializes_stable_js_client_script() {
    let rendered = render_codama_config_json("demo-app", "demo_app").expect("render codama config");
    let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");

    assert_eq!(
        value.get("idl").and_then(|v| v.as_str()),
        Some("target/idl/demo_app.json")
    );
    assert_eq!(
        value.pointer("/scripts/js/from").and_then(|v| v.as_str()),
        Some("@codama/renderers-js")
    );
    assert_eq!(
        value.pointer("/scripts/js/args/0").and_then(|v| v.as_str()),
        Some("clients/js/src/generated")
    );
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
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("codama.json")).unwrap(),
        render_codama_config_json("demo-app", "demo_app").unwrap()
    );
}

#[test]
fn write_codama_config_rejects_missing_workspace_directory() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("missing");

    let err = write_codama_config(&missing, "demo-app", "demo_app")
        .expect_err("missing workspace must fail");

    assert!(err.to_string().contains("codama.json"));
    assert!(!Path::new(&missing).exists());
}
