//! `sunscreen init` wizard.

use std::path::PathBuf;

use crate::bootstrap::{self, Framework, NewArgs};
use crate::error::SunscreenError;
use crate::onboarding::args::{InitArgs, QuickstartRecipeArg};
use crate::onboarding::recipes::{self, RecipeApplier};
use crate::onboarding::{preflight_path, resolve_name};
use crate::strings::en_US;

pub(crate) fn run<A: RecipeApplier>(
    args: &InitArgs,
    json: bool,
    recipe_applier: &A,
) -> Result<i32, SunscreenError> {
    let name = resolve_name(
        args.name.as_deref(),
        args.non_interactive,
        "`sunscreen init` needs a project name in non-interactive mode; try `sunscreen init my-app --non-interactive`",
    )?;
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
    let report = bootstrap::create_workspace(&new_args)?;
    if let Some(recipe) = preset {
        if !report.dry_run {
            recipes::apply_recipe_in_workspace(
                recipe,
                &name,
                args.frontend,
                &report.path,
                recipe_applier,
            )?;
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
