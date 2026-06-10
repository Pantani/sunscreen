//! `sunscreen quickstart` recipes.

use std::path::{Path, PathBuf};

use heck::ToSnakeCase;

use crate::bootstrap::{self, Framework, Frontend, NewArgs};
use crate::error::SunscreenError;
use crate::onboarding::args::{ClusterArg, QuickstartArgs, QuickstartRecipeArg};
use crate::onboarding::{preflight_path, resolve_name};
use crate::strings::en_US;
use crate::workspace;

pub mod blog;
pub mod dao;
pub mod nft;
pub mod token;

pub(crate) trait RecipeApplier {
    fn apply_spl_token(
        &self,
        name: &str,
        program: &str,
        workspace_root: &Path,
    ) -> Result<(), SunscreenError>;

    fn apply_metaplex_nft(
        &self,
        name: &str,
        program: &str,
        workspace_root: &Path,
    ) -> Result<(), SunscreenError>;

    #[allow(clippy::too_many_arguments)]
    fn apply_crud(
        &self,
        name: &str,
        program: &str,
        fields: &str,
        no_update: bool,
        no_delete: bool,
        no_events: bool,
        no_frontend: bool,
        workspace_root: &Path,
    ) -> Result<(), SunscreenError>;
}

pub(crate) fn run<A: RecipeApplier>(
    args: &QuickstartArgs,
    json: bool,
    recipe_applier: &A,
) -> Result<i32, SunscreenError> {
    let name = resolve_name(
        args.name.as_deref(),
        args.non_interactive,
        "`sunscreen quickstart` needs --name in non-interactive mode",
    )?;
    let dest = args.path.clone().unwrap_or_else(|| PathBuf::from(&name));
    preflight_path(&dest, args.dry_run)?;
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
    let workspace = bootstrap::create_workspace(&new_args)?;
    apply_recipe_in_workspace(
        args.recipe,
        &name,
        args.frontend,
        &workspace.path,
        recipe_applier,
    )?;
    emit_report(json, &plan, false, workspace.written);
    Ok(0)
}

pub(crate) fn apply_recipe_in_workspace<A: RecipeApplier>(
    recipe: QuickstartRecipeArg,
    project_name: &str,
    frontend: Frontend,
    workspace_path: &Path,
    recipe_applier: &A,
) -> Result<(), SunscreenError> {
    let ws = workspace::find_root(Some(workspace_path))?;
    apply_recipe(recipe, project_name, frontend, &ws.root, recipe_applier)
}

fn apply_recipe<A: RecipeApplier>(
    recipe: QuickstartRecipeArg,
    project_name: &str,
    frontend: Frontend,
    workspace_root: &Path,
    recipe_applier: &A,
) -> Result<(), SunscreenError> {
    let program = project_name.to_snake_case();
    match recipe {
        QuickstartRecipeArg::Token => {
            recipe_applier.apply_spl_token(token::RESOURCE_NAME, &program, workspace_root)?
        }
        QuickstartRecipeArg::Nft => {
            recipe_applier.apply_metaplex_nft(nft::RESOURCE_NAME, &program, workspace_root)?
        }
        QuickstartRecipeArg::Dao => recipe_applier.apply_crud(
            dao::RESOURCE_NAME,
            &program,
            dao::FIELDS,
            false,
            false,
            false,
            frontend == Frontend::None,
            workspace_root,
        )?,
        QuickstartRecipeArg::Blog => recipe_applier.apply_crud(
            blog::RESOURCE_NAME,
            &program,
            blog::FIELDS,
            false,
            false,
            false,
            frontend == Frontend::None,
            workspace_root,
        )?,
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
        next_steps.insert(2, en_US::WALLET_NEW_STEP.to_string());
        next_steps.insert(3, en_US::DEPLOY_DEVNET_STEP.replace("{program}", &program));
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
