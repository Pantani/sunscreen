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
pub mod fix;
pub mod preflight;
pub mod registry;

pub use detect::{
    detect_all, detect_anchor, detect_codama, detect_one, detect_rustfmt, detect_solana,
    detect_surfpool, is_available, CommandRunner, RealRunner, Status, ToolReport,
};
pub use fix::{
    finalize_fix_results, fix_reports, fix_reports_with_logger, ToolFixLogEvent, ToolFixResult,
    ToolFixStatus,
};
pub use preflight::{
    preflight_chain_new, preflight_chain_new_with, Frontend, PreflightError, PreflightReport,
};
pub use registry::{known, ToolSpec};
