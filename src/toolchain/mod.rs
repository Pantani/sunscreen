//! Toolchain detection and version reporting.
//!
//! Exposes a small, injectable surface so the `doctor` command can probe
//! external CLI tools (anchor, solana, rustc, cargo, node, pnpm, ...) and
//! report whether they meet the minimum versions required by the project.
//!
//! Detection is parallelized via [`std::thread::scope`] — each tool spawns
//! a worker that shells out to `<bin> --version` once and parses with a
//! tolerant regex.

pub mod detect;
pub mod registry;

pub use detect::{detect_all, CommandRunner, RealRunner, Status, ToolReport};
pub use registry::{known, ToolSpec};
