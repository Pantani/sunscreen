//! Tests for the Phase 3 build -> codama runtime pipeline.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;

use sunscreen::runtime::pipeline::{BuildPipeline, PipelineOptions, PipelineStep};
use sunscreen::runtime::subprocess::{CommandOutput, CommandSpec, ProcessError, ProcessRunner};

#[derive(Default)]
struct FakeRunner {
    outputs: RefCell<VecDeque<CommandOutput>>,
    calls: RefCell<Vec<CommandSpec>>,
}

impl FakeRunner {
    fn with_outputs(outputs: Vec<CommandOutput>) -> Self {
        Self {
            outputs: RefCell::new(outputs.into()),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<CommandSpec> {
        self.calls.borrow().clone()
    }
}

impl ProcessRunner for FakeRunner {
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError> {
        self.calls.borrow_mut().push(spec);
        Ok(self.outputs.borrow_mut().pop_front().expect("fake output"))
    }
}

fn output(exit_code: i32, stdout: &str) -> CommandOutput {
    CommandOutput {
        exit_code,
        stdout: stdout.into(),
        stderr: String::new(),
        duration_ms: 7,
    }
}

fn no_frontend_notify() -> PipelineOptions {
    PipelineOptions {
        notify_frontend: false,
        ..PipelineOptions::default()
    }
}

#[test]
fn build_pipeline_runs_anchor_then_codama_in_workspace_root() {
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);
    let root = PathBuf::from("/tmp/sunscreen-workspace");

    let report = BuildPipeline::new(&root)
        .run(&runner, no_frontend_notify())
        .expect("pipeline run");

    assert!(report.success());
    assert_eq!(report.exit_code, 0);
    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].display_argv(), ["anchor", "build"]);
    assert_eq!(calls[1].display_argv(), ["pnpm", "exec", "codama", "run"]);
    assert_eq!(calls[0].cwd.as_deref(), Some(root.as_path()));
    assert_eq!(calls[1].cwd.as_deref(), Some(root.as_path()));

    let finished_steps: Vec<_> = report
        .events
        .iter()
        .filter(|event| event.event == "command_finished")
        .map(|event| event.step)
        .collect();
    assert_eq!(
        finished_steps,
        [PipelineStep::AnchorBuild, PipelineStep::CodamaRun]
    );
}

#[test]
fn build_pipeline_json_cwd_uses_forward_slashes() {
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok")]);
    let root = PathBuf::from(r"C:\tmp\sunscreen-workspace");

    let report = BuildPipeline::new(&root)
        .run(
            &runner,
            PipelineOptions {
                run_codama: false,
                ..no_frontend_notify()
            },
        )
        .expect("pipeline run");

    assert_eq!(
        report.events[0]
            .to_json()
            .get("cwd")
            .and_then(|v| v.as_str()),
        Some("C:/tmp/sunscreen-workspace")
    );
    assert_eq!(
        report.events[1]
            .to_json()
            .get("cwd")
            .and_then(|v| v.as_str()),
        Some("C:/tmp/sunscreen-workspace")
    );
}

#[test]
fn build_pipeline_can_skip_codama() {
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok")]);
    let root = PathBuf::from("/tmp/sunscreen-workspace");

    let report = BuildPipeline::new(&root)
        .run(
            &runner,
            PipelineOptions {
                run_codama: false,
                ..no_frontend_notify()
            },
        )
        .expect("pipeline run");

    assert!(report.success());
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].display_argv(), ["anchor", "build"]);
}

#[test]
fn build_pipeline_stops_before_codama_when_anchor_fails() {
    let runner = FakeRunner::with_outputs(vec![output(42, "anchor failed")]);
    let root = PathBuf::from("/tmp/sunscreen-workspace");

    let report = BuildPipeline::new(&root)
        .run(&runner, no_frontend_notify())
        .expect("pipeline run");

    assert!(!report.success());
    assert_eq!(report.exit_code, 42);
    assert_eq!(runner.calls().len(), 1);
    assert_eq!(
        report.events.last().unwrap().step,
        PipelineStep::AnchorBuild
    );
    assert_eq!(
        report.events.last().unwrap().status.as_deref(),
        Some("failed")
    );
}

#[test]
fn build_pipeline_notifies_scaffolded_frontend_after_codama_success() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir(root.join("app")).expect("create app dir");
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);

    let report = BuildPipeline::new(root)
        .run(&runner, PipelineOptions::default())
        .expect("pipeline run");

    assert!(report.success());
    let reload = root.join("app/.sunscreen/reload");
    assert!(
        reload.exists(),
        "frontend reload sentinel should be touched"
    );
    let notify = report
        .events
        .iter()
        .find(|event| event.step == PipelineStep::FrontendNotify)
        .expect("frontend notify event");
    assert_eq!(notify.event, "frontend_notified");
    let expected_path = format!("{}/app/.sunscreen/reload", root.display()).replace('\\', "/");
    assert_eq!(
        notify.command,
        [
            "sunscreen-internal",
            "frontend-notify",
            expected_path.as_str()
        ]
    );
    assert_eq!(
        notify
            .to_json()
            .get("path")
            .and_then(|value| value.as_str()),
        Some(expected_path.as_str())
    );
}

#[test]
fn build_pipeline_notifies_configured_frontend_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir(root.join("web")).expect("create custom frontend dir");
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);

    let report = BuildPipeline::new(root)
        .run(
            &runner,
            PipelineOptions {
                frontend_path: Some(PathBuf::from("web")),
                ..PipelineOptions::default()
            },
        )
        .expect("pipeline run");

    assert!(report.success());
    assert!(root.join("web/.sunscreen/reload").exists());
    assert!(!root.join("app/.sunscreen/reload").exists());
}

#[test]
fn build_pipeline_surfaces_frontend_notify_write_errors() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir(root.join("app")).expect("create app dir");
    std::fs::write(root.join("app/.sunscreen"), "not a dir").expect("create blocking file");
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);

    let err = BuildPipeline::new(root)
        .run(&runner, PipelineOptions::default())
        .expect_err("frontend notify write failure should fail pipeline");

    assert_eq!(err.step, PipelineStep::FrontendNotify);
}

#[test]
fn build_pipeline_skips_frontend_notify_without_app_dir() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);

    let report = BuildPipeline::new(tmp.path())
        .run(&runner, PipelineOptions::default())
        .expect("pipeline run");

    assert!(!report
        .events
        .iter()
        .any(|event| event.step == PipelineStep::FrontendNotify));
    assert!(!tmp.path().join("app/.sunscreen/reload").exists());
}
