//! Runtime orchestration primitives for Phase 3.

use std::path::Path;

pub mod pipeline;
pub mod serve;
pub mod subprocess;
pub mod supervisor;
pub mod surfpool;
pub mod testvalidator;
pub mod validator;
pub mod watcher;

pub(crate) fn render_event_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
