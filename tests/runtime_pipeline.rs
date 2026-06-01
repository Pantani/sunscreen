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

#[test]
fn build_pipeline_runs_anchor_then_codama_in_workspace_root() {
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);
    let root = PathBuf::from("/tmp/sunscreen-workspace");

    let report = BuildPipeline::new(&root)
        .run(&runner, PipelineOptions { run_codama: true })
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
fn build_pipeline_can_skip_codama() {
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok")]);
    let root = PathBuf::from("/tmp/sunscreen-workspace");

    let report = BuildPipeline::new(&root)
        .run(&runner, PipelineOptions { run_codama: false })
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
        .run(&runner, PipelineOptions { run_codama: true })
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
