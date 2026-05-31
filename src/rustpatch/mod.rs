//! Marker-based, regex-free Rust source patching.
//!
//! See `docs/adr/ADR-0001-solis-cli.md` §7.1 (Sub-ADR-001) and §8.2.
//!
//! Two marker shapes are recognised:
//!
//! ```text
//! // === sunscreen:auto-generated:begin segment=<name> version=<n> [generator=<g>] ===
//! ... body owned by the CLI ...
//! // === sunscreen:auto-generated:end segment=<name> ===
//!
//! // === sunscreen:user-region:begin segment=<name> ===
//! ... body owned by the user, never touched by `apply` ...
//! // === sunscreen:user-region:end segment=<name> ===
//! ```
//!
//! `scan` enumerates markers. `apply` rewrites the body between matching
//! auto-generated begin/end pairs. User regions are read-only.
//!
//! Implementation notes:
//! - **No regex.** Everything is line-oriented string scan.
//! - `apply` is idempotent: applying the same patch set twice yields the
//!   same output.
//! - Whitespace prefix on a marker line is preserved verbatim. The marker
//!   lines themselves are never modified.

pub mod marker;

pub use marker::{apply, scan, Marker, MarkerKind, Patch, RustpatchError};
