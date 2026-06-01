//! Tests for wiring watcher batches into the Phase 3 build pipeline.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use notify::{Event, EventKind};
use sunscreen::runtime::pipeline::{PipelineOptions, PipelineStep};
use sunscreen::runtime::subprocess::{CommandOutput, CommandSpec, ProcessError, ProcessRunner};
use sunscreen::runtime::watcher::{WatchBuildLoop, WatchDebouncer};

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
fn debouncer_accepts_notify_events() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(50));
    let event = Event::new(EventKind::Any)
        .add_path(PathBuf::from("README.md"))
        .add_path(PathBuf::from("programs/demo/src/lib.rs"));

    assert!(watcher.observe_notify_event(&event, start).is_none());

    let batch = watcher
        .flush_due(start + Duration::from_millis(50))
        .expect("debounced notify event");
    assert_eq!(batch.paths, [PathBuf::from("programs/demo/src/lib.rs")]);
}

#[test]
fn debouncer_notify_event_does_not_flush_due_batch() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(50));
    watcher.observe("programs/demo/src/lib.rs", start);
    let event = Event::new(EventKind::Any).add_path(PathBuf::from("README.md"));

    assert!(watcher
        .observe_notify_event(&event, start + Duration::from_millis(50))
        .is_none());

    let batch = watcher
        .flush_due(start + Duration::from_millis(50))
        .expect("notify adapter must leave due batch for explicit flush");
    assert_eq!(batch.paths, [PathBuf::from("programs/demo/src/lib.rs")]);
}

#[test]
fn watch_build_loop_runs_pipeline_when_debounced_batch_is_due() {
    let start = Instant::now();
    let runner = FakeRunner::with_outputs(vec![output(0, "anchor ok"), output(0, "codama ok")]);
    let root = PathBuf::from("/tmp/sunscreen-workspace");
    let mut loop_ = WatchBuildLoop::new(
        &root,
        Duration::from_millis(25),
        PipelineOptions { run_codama: true },
    );
    let event =
        Event::new(EventKind::Any).add_path(root.join("programs/demo/src/instructions/deposit.rs"));

    loop_.observe_notify_event(&event, start);
    assert!(loop_
        .flush_due(start + Duration::from_millis(24), &runner)
        .expect("watch loop")
        .is_none());

    let report = loop_
        .flush_due(start + Duration::from_millis(25), &runner)
        .expect("watch loop")
        .expect("pipeline report");

    assert_eq!(
        report.batch.paths,
        [PathBuf::from("programs/demo/src/instructions/deposit.rs")]
    );
    assert!(report.pipeline.success());
    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].display_argv(), ["anchor", "build"]);
    assert_eq!(calls[1].display_argv(), ["pnpm", "exec", "codama", "run"]);
    assert_eq!(calls[0].cwd.as_deref(), Some(root.as_path()));
    assert_eq!(calls[1].cwd.as_deref(), Some(root.as_path()));

    let finished_steps: Vec<_> = report
        .pipeline
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
