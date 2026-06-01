//! Tests for the Phase 3 headless serve loop core.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use notify::{Event, EventKind};
use sunscreen::runtime::pipeline::PipelineOptions;
use sunscreen::runtime::serve::{HeadlessServeLoop, ServeLoopInput};
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
fn headless_serve_loop_emits_json_when_debounced_event_runs_pipeline() {
    let start = Instant::now();
    let root = PathBuf::from("/tmp/sunscreen-workspace");
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);
    let mut loop_ =
        HeadlessServeLoop::new(&root, Duration::from_millis(25), PipelineOptions::default());
    let event =
        Event::new(EventKind::Any).add_path(root.join("programs/demo/src/instructions/deposit.rs"));

    assert!(loop_
        .handle_input(ServeLoopInput::NotifyEvent(event, start), &runner)
        .expect("serve event")
        .is_empty());
    assert!(loop_
        .handle_input(
            ServeLoopInput::Tick(start + Duration::from_millis(24)),
            &runner
        )
        .expect("serve tick")
        .is_empty());

    let events = loop_
        .handle_input(
            ServeLoopInput::Tick(start + Duration::from_millis(25)),
            &runner,
        )
        .expect("serve tick");

    let names: Vec<_> = events
        .iter()
        .map(|event| event.get("event").and_then(|v| v.as_str()).unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "chain_serve_build_started",
            "command_started",
            "command_finished",
            "command_started",
            "command_finished",
            "chain_serve_build_finished",
        ]
    );
    assert_eq!(
        events[0].get("paths").unwrap(),
        &serde_json::json!(["programs/demo/src/instructions/deposit.rs"])
    );
    assert_eq!(
        events
            .last()
            .unwrap()
            .get("status")
            .and_then(|v| v.as_str()),
        Some("ok")
    );

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].display_argv(), ["anchor", "build"]);
    assert_eq!(calls[1].display_argv(), ["pnpm", "exec", "codama", "run"]);
}

#[test]
fn headless_serve_loop_flushes_due_batch_after_notify_event() {
    let start = Instant::now();
    let root = PathBuf::from("/tmp/sunscreen-workspace");
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);
    let mut loop_ =
        HeadlessServeLoop::new(&root, Duration::from_millis(25), PipelineOptions::default());
    let source_event =
        Event::new(EventKind::Any).add_path(root.join("programs/demo/src/instructions/deposit.rs"));
    let ignored_event = Event::new(EventKind::Any).add_path(root.join("README.md"));

    assert!(loop_
        .handle_input(ServeLoopInput::NotifyEvent(source_event, start), &runner)
        .expect("serve event")
        .is_empty());

    let events = loop_
        .handle_input(
            ServeLoopInput::NotifyEvent(ignored_event, start + Duration::from_millis(25)),
            &runner,
        )
        .expect("serve event should flush due batch");

    assert_eq!(
        events[0].get("event").and_then(|v| v.as_str()),
        Some("chain_serve_build_started")
    );
    assert_eq!(runner.calls().len(), 2);
}
