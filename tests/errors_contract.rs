use sunscreen::SunscreenError;

#[cfg(feature = "onboarding")]
fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

#[test]
fn all_sunscreen_errors_have_stable_exit_kind_and_next_step() {
    let cases = [
        SunscreenError::Other(anyhow::anyhow!("boom")),
        SunscreenError::ToolchainMissing("anchor".into()),
        SunscreenError::ConfigInvalid("bad yaml".into()),
        SunscreenError::UserInput("bad flag".into()),
        SunscreenError::WorkspaceMissing("no workspace".into()),
        SunscreenError::InstructionDrift {
            path: "programs/demo/src/lib.rs".into(),
            hint: "markers missing".into(),
        },
        SunscreenError::PathConflict("target exists".into()),
        SunscreenError::Network("rpc failed".into()),
        SunscreenError::PluginRuntime("plugin crashed".into()),
    ];

    for err in cases {
        let (code, kind) = expected_contract(&err);
        assert_eq!(err.exit_code(), code, "{err}");
        assert_eq!(err.kind_str(), kind, "{err}");
        assert!(
            err.next_step().is_some_and(|step| !step.is_empty()),
            "{err} missing next_step"
        );
    }
}

fn expected_contract(err: &SunscreenError) -> (i32, &'static str) {
    match err {
        SunscreenError::Other(_) => (1, "other"),
        SunscreenError::ToolchainMissing(_) => (2, "toolchain_missing"),
        SunscreenError::ConfigInvalid(_) => (3, "config_invalid"),
        SunscreenError::UserInput(_) => (4, "user_input"),
        SunscreenError::WorkspaceMissing(_) => (5, "workspace_missing"),
        SunscreenError::InstructionDrift { .. } => (6, "instruction_drift"),
        SunscreenError::PathConflict(_) => (7, "path_conflict"),
        SunscreenError::Network(_) => (8, "network"),
        SunscreenError::PluginRuntime(_) => (9, "plugin_runtime"),
    }
}

#[cfg(feature = "onboarding")]
#[test]
fn json_error_schema_preserves_legacy_fields_and_adds_next_step() {
    let out = std::process::Command::new(sunscreen_bin())
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
