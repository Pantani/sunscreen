//! Tests for Phase 3 local validator runtime supervision.

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;

use sunscreen::process::{CommandSpec, ManagedProcess, ProcessError, ProcessSpawner};
use sunscreen::runtime::supervisor::RuntimeSupervisor;
use sunscreen::runtime::surfpool::SurfpoolRuntime;
use sunscreen::runtime::testvalidator::TestValidatorRuntime;
use sunscreen::runtime::validator::{Runtime, RuntimePorts};

#[test]
fn surfpool_runtime_declares_command_and_endpoints() {
    let root = PathBuf::from("/tmp/sunscreen-workspace");
    let runtime = SurfpoolRuntime::new(RuntimePorts::new(8899, 8900));

    let spec = runtime.command(&root);

    assert_eq!(runtime.name(), "surfpool");
    assert_eq!(
        spec.display_argv(),
        ["surfpool", "start", "--port", "8899", "--ws-port", "8900"]
    );
    assert_eq!(spec.cwd.as_deref(), Some(root.as_path()));
    assert_eq!(runtime.endpoints().rpc, "http://127.0.0.1:8899");
    assert_eq!(runtime.endpoints().ws, "ws://127.0.0.1:8900");
}

#[test]
fn testvalidator_runtime_declares_command_and_endpoints() {
    let root = PathBuf::from("/tmp/sunscreen-workspace");
    let runtime = TestValidatorRuntime::new(RuntimePorts::new(8899, 8900));

    let spec = runtime.command(&root);

    assert_eq!(runtime.name(), "test-validator");
    assert_eq!(
        spec.display_argv(),
        ["solana-test-validator", "--rpc-port", "8899", "--reset"]
    );
    assert_eq!(spec.cwd.as_deref(), Some(root.as_path()));
    assert_eq!(runtime.endpoints().rpc, "http://127.0.0.1:8899");
    assert_eq!(runtime.endpoints().ws, "ws://127.0.0.1:8900");
}

#[test]
fn supervisor_starts_and_stops_runtime_process() {
    let root = PathBuf::from("/tmp/sunscreen-workspace");
    let killed = Rc::new(RefCell::new(false));
    let spawner = FakeSpawner {
        calls: RefCell::new(Vec::new()),
        child: RefCell::new(Some(FakeManagedProcess {
            pid: 42,
            killed: Rc::clone(&killed),
        })),
    };
    let runtime = SurfpoolRuntime::new(RuntimePorts::new(8899, 8900));
    let mut supervisor = RuntimeSupervisor::new(runtime, &root);

    let report = supervisor.start(&spawner).expect("start runtime");

    assert_eq!(report.runtime, "surfpool");
    assert_eq!(report.pid, 42);
    assert_eq!(report.rpc_endpoint, "http://127.0.0.1:8899");
    assert_eq!(report.ws_endpoint, "ws://127.0.0.1:8900");
    assert_eq!(spawner.calls.borrow().len(), 1);
    assert_eq!(
        spawner.calls.borrow()[0].display_argv(),
        ["surfpool", "start", "--port", "8899", "--ws-port", "8900"]
    );

    supervisor.stop().expect("stop runtime");
    assert!(*killed.borrow());
}

struct FakeSpawner {
    calls: RefCell<Vec<CommandSpec>>,
    child: RefCell<Option<FakeManagedProcess>>,
}

impl ProcessSpawner for FakeSpawner {
    fn spawn(&self, spec: CommandSpec) -> Result<Box<dyn ManagedProcess>, ProcessError> {
        self.calls.borrow_mut().push(spec);
        Ok(Box::new(
            self.child
                .borrow_mut()
                .take()
                .expect("fake child available"),
        ))
    }
}

struct FakeManagedProcess {
    pid: u32,
    killed: Rc<RefCell<bool>>,
}

impl ManagedProcess for FakeManagedProcess {
    fn id(&self) -> u32 {
        self.pid
    }

    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        Ok(None)
    }

    fn stop(&mut self) -> io::Result<()> {
        *self.killed.borrow_mut() = true;
        Ok(())
    }
}
