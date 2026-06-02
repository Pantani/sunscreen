//! `sunscreen quickstart` recipes.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use heck::ToSnakeCase;

use crate::cli::chain::{self, Framework, Frontend, NewArgs};
use crate::cli::onboarding::{ClusterArg, QuickstartArgs, QuickstartRecipeArg};
use crate::cli::scaffold::{self, BuiltinRecipeArgs, CrudArgs};
use crate::error::SunscreenError;
use crate::onboarding::tty;
use crate::strings::en_US;

pub mod blog;
pub mod dao;
pub mod nft;
pub mod token;

pub fn run(args: &QuickstartArgs, json: bool) -> Result<i32, SunscreenError> {
    let name = resolve_name(args)?;
    let dest = args.path.clone().unwrap_or_else(|| PathBuf::from(&name));
    preflight_dest(&dest, args.dry_run)?;
    let plan = plan_for(args.recipe, &name, args.cluster, args.frontend, &dest);

    if args.dry_run {
        emit_report(json, &plan, true, 0);
        return Ok(0);
    }

    let new_args = NewArgs {
        name: name.clone(),
        framework: Framework::Anchor,
        frontend: args.frontend,
        path: args.path.clone(),
        dry_run: false,
    };
    let workspace = chain::create_workspace(&new_args)?;
    apply_recipe_in_workspace(args.recipe, &name, args.frontend, &workspace.path)?;
    emit_report(json, &plan, false, workspace.written);
    Ok(0)
}

pub(crate) fn apply_recipe_in_workspace(
    recipe: QuickstartRecipeArg,
    project_name: &str,
    frontend: Frontend,
    workspace_path: &Path,
) -> Result<(), SunscreenError> {
    let _cwd = CurrentDirGuard::push(workspace_path)?;
    apply_recipe(recipe, project_name, frontend)
}

fn apply_recipe(
    recipe: QuickstartRecipeArg,
    project_name: &str,
    frontend: Frontend,
) -> Result<(), SunscreenError> {
    let program = project_name.to_snake_case();
    match recipe {
        QuickstartRecipeArg::Token => scaffold::run_spl_token_quiet(&BuiltinRecipeArgs {
            name: token::RESOURCE_NAME.to_string(),
            program,
            dry_run: false,
        })?,
        QuickstartRecipeArg::Nft => scaffold::run_metaplex_nft_quiet(&BuiltinRecipeArgs {
            name: nft::RESOURCE_NAME.to_string(),
            program,
            dry_run: false,
        })?,
        QuickstartRecipeArg::Dao => scaffold::run_crud_quiet(&CrudArgs {
            name: dao::RESOURCE_NAME.to_string(),
            program,
            fields: dao::FIELDS.to_string(),
            no_update: false,
            no_delete: false,
            no_events: false,
            no_frontend: frontend == Frontend::None,
            dry_run: false,
        })?,
        QuickstartRecipeArg::Blog => scaffold::run_crud_quiet(&CrudArgs {
            name: blog::RESOURCE_NAME.to_string(),
            program,
            fields: blog::FIELDS.to_string(),
            no_update: false,
            no_delete: false,
            no_events: false,
            no_frontend: frontend == Frontend::None,
            dry_run: false,
        })?,
    };
    Ok(())
}

#[derive(Debug)]
struct QuickstartPlan {
    recipe: &'static str,
    resource: &'static str,
    description: &'static str,
    project: String,
    path: PathBuf,
    cluster: &'static str,
    frontend: &'static str,
    operations: Vec<String>,
    next_steps: Vec<String>,
}

fn plan_for(
    recipe: QuickstartRecipeArg,
    name: &str,
    cluster: ClusterArg,
    frontend: Frontend,
    dest: &Path,
) -> QuickstartPlan {
    let (recipe_name, resource, description, scaffold_op) = match recipe {
        QuickstartRecipeArg::Token => (
            "token",
            token::RESOURCE_NAME,
            token::DESCRIPTION,
            format!("scaffold spl-token {}", token::RESOURCE_NAME),
        ),
        QuickstartRecipeArg::Nft => (
            "nft",
            nft::RESOURCE_NAME,
            nft::DESCRIPTION,
            format!("scaffold metaplex-nft {}", nft::RESOURCE_NAME),
        ),
        QuickstartRecipeArg::Dao => (
            "dao",
            dao::RESOURCE_NAME,
            dao::DESCRIPTION,
            format!("scaffold crud {}", dao::RESOURCE_NAME),
        ),
        QuickstartRecipeArg::Blog => (
            "blog",
            blog::RESOURCE_NAME,
            blog::DESCRIPTION,
            format!("scaffold crud {}", blog::RESOURCE_NAME),
        ),
    };
    let program = name.to_snake_case();
    let mut next_steps = vec![
        format!("cd {}", dest.display()),
        en_US::QUICKSTART_BUILD_STEP.to_string(),
        en_US::QUICKSTART_SERVE_STEP.to_string(),
    ];
    if cluster == ClusterArg::Devnet {
        next_steps.insert(1, en_US::WALLET_NEW_STEP.to_string());
        next_steps.insert(2, en_US::DEPLOY_DEVNET_STEP.replace("{program}", &program));
    }
    QuickstartPlan {
        recipe: recipe_name,
        resource,
        description,
        project: name.to_string(),
        path: dest.to_path_buf(),
        cluster: cluster.as_str(),
        frontend: frontend_str(frontend),
        operations: vec![
            "chain new".to_string(),
            scaffold_op,
            "generate-ready frontend hooks placeholder".to_string(),
        ],
        next_steps,
    }
}

fn resolve_name(args: &QuickstartArgs) -> Result<String, SunscreenError> {
    if let Some(name) = args.name.as_ref().filter(|value| !value.trim().is_empty()) {
        return Ok(name.trim().to_string());
    }
    if !tty::is_interactive(args.non_interactive) {
        return Err(SunscreenError::UserInput(
            "`sunscreen quickstart` needs --name in non-interactive mode".into(),
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

fn preflight_dest(path: &Path, dry_run: bool) -> Result<(), SunscreenError> {
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

fn emit_report(json: bool, plan: &QuickstartPlan, dry_run: bool, workspace_files: usize) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "quickstart",
                "recipe": plan.recipe,
                "resource": plan.resource,
                "description": plan.description,
                "project": plan.project,
                "path": plan.path.display().to_string(),
                "cluster": plan.cluster,
                "frontend": plan.frontend,
                "dry_run": dry_run,
                "workspace_files": workspace_files,
                "operations": plan.operations,
                "next_steps": plan.next_steps,
            })
        );
    } else if dry_run {
        println!(
            "dry-run: would create `{}` quickstart `{}` at {}",
            plan.recipe,
            plan.project,
            plan.path.display()
        );
        for op in &plan.operations {
            println!("  {op}");
        }
    } else {
        println!(
            "created `{}` quickstart `{}` at {}",
            plan.recipe,
            plan.project,
            plan.path.display()
        );
        println!("next steps:");
        for step in &plan.next_steps {
            println!("  {step}");
        }
    }
}

fn frontend_str(frontend: Frontend) -> &'static str {
    match frontend {
        Frontend::Next => "next",
        Frontend::Vite => "vite",
        Frontend::None => "none",
    }
}

struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    fn push(path: &Path) -> Result<Self, SunscreenError> {
        let previous = std::env::current_dir()
            .map_err(|err| SunscreenError::Other(anyhow::anyhow!("read current dir: {err}")))?;
        std::env::set_current_dir(path).map_err(|err| {
            SunscreenError::Other(anyhow::anyhow!("enter workspace {}: {err}", path.display()))
        })?;
        Ok(Self { previous })
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous);
    }
}
