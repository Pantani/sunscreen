//! `sunscreen init` wizard.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::cli::chain::{self, Framework, NewArgs};
use crate::cli::onboarding::{InitArgs, QuickstartRecipeArg};
use crate::error::SunscreenError;
use crate::onboarding::{recipes, tty};
use crate::strings::en_US;

pub fn run(args: &InitArgs, json: bool) -> Result<i32, SunscreenError> {
    let name = resolve_name(args)?;
    let preset = resolve_preset(args.from_preset.as_deref())?;
    let dest = args.path.clone().unwrap_or_else(|| PathBuf::from(&name));
    preflight_path(&dest, args.dry_run)?;

    let new_args = NewArgs {
        name: name.clone(),
        framework: Framework::Anchor,
        frontend: args.frontend,
        path: args.path.clone(),
        dry_run: args.dry_run,
    };
    let report = chain::create_workspace(&new_args)?;
    if let Some(recipe) = preset {
        if !report.dry_run {
            recipes::apply_recipe_in_workspace(recipe, &name, args.frontend, &report.path)?;
        }
    }
    let next_step = en_US::INIT_NEXT_STEP.replace("{path}", &report.path.display().to_string());

    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "init",
                "project": name,
                "preset": args.from_preset.as_deref().unwrap_or("empty"),
                "preset_applied": preset.is_some() && !report.dry_run,
                "frontend": format!("{:?}", args.frontend).to_ascii_lowercase(),
                "path": report.path.display().to_string(),
                "dry_run": report.dry_run,
                "files": report.files,
                "written": report.written,
                "next_step": next_step,
            })
        );
    } else if report.dry_run {
        println!(
            "dry-run: would initialize `{}` at {}",
            name,
            report.path.display()
        );
        for file in &report.files {
            println!("  {file}");
        }
        println!("next: {next_step}");
    } else {
        println!(
            "initialized `{}` at {} ({} files)",
            name,
            report.path.display(),
            report.written
        );
        println!("next: {next_step}");
    }
    Ok(0)
}

fn resolve_name(args: &InitArgs) -> Result<String, SunscreenError> {
    if let Some(name) = args.name.as_ref().filter(|value| !value.trim().is_empty()) {
        return Ok(name.trim().to_string());
    }
    if !tty::is_interactive(args.non_interactive) {
        return Err(SunscreenError::UserInput(
            "`sunscreen init` needs a project name in non-interactive mode; try `sunscreen init my-app --non-interactive`".into(),
        ));
    }
    print!("Project name: ");
    io::stdout()
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

fn resolve_preset(preset: Option<&str>) -> Result<Option<QuickstartRecipeArg>, SunscreenError> {
    let Some(preset) = preset else {
        return Ok(None);
    };
    match preset {
        "empty" => Ok(None),
        "token" => Ok(Some(QuickstartRecipeArg::Token)),
        "nft" => Ok(Some(QuickstartRecipeArg::Nft)),
        "dao" => Ok(Some(QuickstartRecipeArg::Dao)),
        "blog" => Ok(Some(QuickstartRecipeArg::Blog)),
        other => Err(SunscreenError::UserInput(format!(
            "unknown init preset `{other}`; expected token, nft, dao, blog, or empty"
        ))),
    }
}

fn preflight_path(path: &Path, dry_run: bool) -> Result<(), SunscreenError> {
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
