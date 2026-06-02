//! sunscreen — Solana CLI scaffolding & orchestration tool.
//!
//! See `docs/adr/ADR-0001-solis-cli.md` for design rationale.

pub mod cli;
pub mod codegen;
pub mod config;
pub mod error;
pub mod fsutil;
#[cfg(feature = "onboarding")]
pub mod onboarding;
pub mod plugin;
pub mod runtime;
pub mod rustpatch;
pub mod scaffold;
#[cfg(feature = "onboarding")]
pub mod strings;
pub mod templates;
pub mod toolchain;
pub mod tui;
pub mod workspace;

pub use error::SunscreenError;
