//! Concrete migrations registered with the migrator.

use anyhow::Result;

pub mod v0_to_v1;

/// A single, ordered migration step on the raw YAML document.
pub trait Migration: Send + Sync {
    /// Source version this migration upgrades from.
    fn from(&self) -> u32;
    /// Target version after this migration is applied.
    fn to(&self) -> u32;
    /// Apply the transformation in-place on the raw YAML value.
    fn apply(&self, raw: &mut serde_yaml::Value) -> Result<()>;
}
