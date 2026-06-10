//! Beginner onboarding layer.

use std::io::{self, Write};
use std::path::Path;

use crate::error::SunscreenError;

pub mod args;
pub mod deploy;
pub mod examples;
pub mod learn;
pub mod recipes;
pub mod tty;
pub mod wallet;
pub mod wizard;

pub(crate) fn resolve_name(
    name: Option<&str>,
    non_interactive: bool,
    non_interactive_message: &'static str,
) -> Result<String, SunscreenError> {
    if let Some(name) = name.filter(|value| !value.trim().is_empty()) {
        return Ok(name.trim().to_string());
    }
    if !tty::is_interactive(non_interactive) {
        return Err(SunscreenError::UserInput(non_interactive_message.into()));
    }
    eprint!("Project name: ");
    io::stderr()
        .flush()
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("flush prompt: {err}")))?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("read prompt: {err}")))?;
    let name = input.trim();
    if name.is_empty() {
        return Err(SunscreenError::UserInput(
            "project name cannot be empty".into(),
        ));
    }
    Ok(name.to_string())
}

pub(crate) fn preflight_path(path: &Path, dry_run: bool) -> Result<(), SunscreenError> {
    if dry_run || !path.exists() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(path).map_err(|err| {
        SunscreenError::Other(anyhow::anyhow!(
            "read output directory {}: {err}",
            path.display()
        ))
    })?;
    if entries.next().is_some() {
        return Err(SunscreenError::PathConflict(format!(
            "destination already exists and is not empty: {}",
            path.display()
        )));
    }
    Ok(())
}
