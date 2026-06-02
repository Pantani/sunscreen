use std::process::Command;

use sunscreen::SunscreenError;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

#[test]
fn all_sunscreen_errors_have_stable_exit_kind_and_next_step() {
    let cases = vec![
        (SunscreenError::Other(anyhow::anyhow!("boom")), 1, "other"),
        (
            SunscreenError::ToolchainMissing("anchor".into()),
            2,
            "toolchain_missing",
        ),
        (
            SunscreenError::ConfigInvalid("bad yaml".into()),
            3,
            "config_invalid",
        ),
        (
            SunscreenError::UserInput("bad flag".into()),
            4,
            "user_input",
        ),
        (
            SunscreenError::WorkspaceMissing("no workspace".into()),
            5,
            "workspace_missing",
        ),
        (
            SunscreenError::InstructionDrift {
                path: "programs/demo/src/lib.rs".into(),
                hint: "markers missing".into(),
            },
            6,
            "instruction_drift",
        ),
        (
            SunscreenError::PathConflict("target exists".into()),
            7,
            "path_conflict",
        ),
        (SunscreenError::Network("rpc failed".into()), 8, "network"),
    ];

    for (err, code, kind) in cases {
        assert_eq!(err.exit_code(), code, "{err}");
        assert_eq!(err.kind_str(), kind, "{err}");
        assert!(
            err.next_step().is_some_and(|step| !step.is_empty()),
            "{err} missing next_step"
        );
    }
}

#[test]
fn json_error_schema_preserves_legacy_fields_and_adds_next_step() {
    let out = Command::new(sunscreen_bin())
        .args(["--json", "quickstart", "token", "--non-interactive"])
        .output()
        .expect("invoke sunscreen");
    assert_eq!(out.status.code(), Some(4));
    let payload: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("stderr json payload");
    assert_eq!(payload["kind"], "user_input");
    assert!(payload["error"].as_str().unwrap().contains("--name"));
    assert_eq!(payload["exit_code"], 4);
    assert!(payload["next_step"].as_str().unwrap().contains("--help"));
}
