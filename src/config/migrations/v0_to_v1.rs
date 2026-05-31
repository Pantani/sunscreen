//! Migration: schema v0 -> v1.
//!
//! In the hypothetical v0 layout, the project identifier lived at
//! `project.id`. v1 renames that key to `project.name`. All other fields
//! are preserved untouched.

use anyhow::Result;
use serde_yaml::Value;

use crate::config::migrator::Migration;

/// Renames `project.id` to `project.name` when the latter is absent.
pub struct V0ToV1;

impl Migration for V0ToV1 {
    fn from(&self) -> u32 {
        0
    }

    fn to(&self) -> u32 {
        1
    }

    fn apply(&self, raw: &mut Value) -> Result<()> {
        let Some(root) = raw.as_mapping_mut() else {
            return Ok(());
        };
        let project_key = Value::String("project".to_string());
        let Some(project) = root.get_mut(&project_key) else {
            return Ok(());
        };
        let Some(project_map) = project.as_mapping_mut() else {
            return Ok(());
        };
        let id_key = Value::String("id".to_string());
        let name_key = Value::String("name".to_string());
        let has_name = project_map.contains_key(&name_key);
        if has_name {
            // If both exist, drop legacy `id` to avoid deny_unknown_fields rejection.
            project_map.remove(&id_key);
            return Ok(());
        }
        if let Some(id_val) = project_map.remove(&id_key) {
            project_map.insert(name_key, id_val);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaml(s: &str) -> Value {
        serde_yaml::from_str(s).expect("yaml")
    }

    #[test]
    fn renames_id_to_name() {
        let mut v = yaml("version: 0\nproject:\n  id: foo\n");
        V0ToV1.apply(&mut v).unwrap();
        let project = v.get("project").unwrap().as_mapping().unwrap();
        assert!(!project.contains_key(Value::String("id".to_string())));
        assert_eq!(
            project
                .get(Value::String("name".to_string()))
                .and_then(Value::as_str),
            Some("foo")
        );
    }

    #[test]
    fn keeps_name_when_both_present() {
        let mut v = yaml("version: 0\nproject:\n  id: legacy\n  name: keep\n");
        V0ToV1.apply(&mut v).unwrap();
        let project = v.get("project").unwrap().as_mapping().unwrap();
        assert!(!project.contains_key(Value::String("id".to_string())));
        assert_eq!(
            project
                .get(Value::String("name".to_string()))
                .and_then(Value::as_str),
            Some("keep")
        );
    }

    #[test]
    fn no_project_is_noop() {
        let mut v = yaml("version: 0\n");
        V0ToV1.apply(&mut v).unwrap();
        assert!(v.get("project").is_none());
    }
}
