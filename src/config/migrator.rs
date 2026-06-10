//! Versioned configuration migrations.
//!
//! The framework is present from v1 even though there are no registered
//! migrations yet — the goal is to make adding `v1 -> v2` a mechanical task.

use anyhow::{bail, Result};

use crate::config::migrations::Migration;

/// Registry of all known migrations, in apply order.
#[must_use]
pub fn registry() -> Vec<Box<dyn Migration>> {
    vec![Box::new(crate::config::migrations::v0_to_v1::V0ToV1)]
}

/// Apply migrations from the document's current `version` up to `target`.
///
/// `raw` must be a YAML mapping with a numeric `version` field. Migrations are
/// looked up linearly: for each step we pick the migration whose `from` matches
/// the current version and apply it, then bump the version.
pub fn migrate(raw: &mut serde_yaml::Value, target: u32) -> Result<()> {
    let migrations = registry();
    loop {
        let current = current_version(raw)?;
        if current == target {
            return Ok(());
        }
        if current > target {
            bail!(
                "cannot downgrade config from version {current} to {target}: no migrations available"
            );
        }
        let Some(step) = migrations.iter().find(|m| m.from() == current) else {
            bail!("no migration registered from version {current} (target {target})");
        };
        step.apply(raw)?;
        set_version(raw, step.to())?;
    }
}

fn current_version(raw: &serde_yaml::Value) -> Result<u32> {
    let Some(mapping) = raw.as_mapping() else {
        bail!("config root must be a mapping");
    };
    let v = mapping
        .get(serde_yaml::Value::String("version".to_string()))
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(1);
    Ok(u32::try_from(v).unwrap_or(1))
}

fn set_version(raw: &mut serde_yaml::Value, version: u32) -> Result<()> {
    let Some(mapping) = raw.as_mapping_mut() else {
        bail!("config root must be a mapping");
    };
    mapping.insert(
        serde_yaml::Value::String("version".to_string()),
        serde_yaml::Value::Number(serde_yaml::Number::from(u64::from(version))),
    );
    Ok(())
}
