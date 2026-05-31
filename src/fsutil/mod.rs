//! Filesystem utilities for sunscreen.
//!
//! Currently exposes [`transaction::Transaction`] — a two-phase commit helper
//! used by scaffolding commands to guarantee that a failed render never
//! leaves the workspace half-mutated. See `docs/adr/ADR-0001-solis-cli.md`
//! §7.7 (Sub-ADR-007).

pub mod transaction;

pub use transaction::{Transaction, TxError};
