//! `sunscreen scaffold` subcommand group.
//!
//! Phase 2 R1 shipped `instruction`. R3 adds `account`, `event`, and `error`
//! using the same architecture (Args struct → `run_*` → Transaction commit).
//! `program` remains reserved for R4.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use heck::{ToKebabCase, ToPascalCase, ToSnakeCase};

use crate::config::schema::{Framework as ConfigFramework, Frontend};
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::plugin::manager::{self, PluginManager};
use crate::rustpatch::{apply, scan, MarkerKind, Patch, RustpatchError};
use crate::scaffold::crud::{build as build_crud_recipe, CrudRecipeOptions};
use crate::scaffold::recipes::metaplex_nft::{
    build as build_metaplex_nft_recipe, MetaplexNftRecipeOptions,
};
use crate::scaffold::recipes::spl_token::{build as build_spl_token_recipe, SplTokenRecipeOptions};
use crate::scaffold::{GeneratedFile, RecipePlan, RecipeStep};
use crate::templates::{
    render_account_file, render_account_mod_segment, render_dispatch_segment, render_error_variant,
    render_event_entry, render_instruction, render_instructions_mod_segment, render_program,
    AccountCtx, AccountKind, AccountSpec, ArgSpec, ErrorVariant, EventCtx, InstructionCtx,
    InstructionDispatch, ERROR_VARIANTS_SEGMENT_BEGIN, ERROR_VARIANTS_SEGMENT_END,
    EVENTS_FILE_HEADER, EVENTS_SEGMENT_BEGIN, EVENTS_SEGMENT_END,
};
use crate::workspace::{self, ProgramView, WorkspaceError};

/// Subcommands grouped under `sunscreen scaffold`.
#[derive(Debug, Subcommand)]
pub enum ScaffoldCmd {
    /// Generate a new Anchor instruction handler.
    Instruction(InstructionArgs),
    /// Generate a new `#[account]` state struct.
    Account(AccountArgs),
    /// Add a new `#[event]` struct to the program's `events.rs`.
    Event(EventArgs),
    /// Add a new variant to the program's `#[error_code]` enum.
    Error(ErrorArgs),
    /// Add a new Anchor program crate to an existing multi-program workspace.
    Program(ProgramArgs),
    /// Generate a CRUD dApp slice for a resource.
    Crud(CrudArgs),
    /// Generate an SPL token recipe slice.
    SplToken(BuiltinRecipeArgs),
    /// Generate a Metaplex NFT recipe slice.
    MetaplexNft(BuiltinRecipeArgs),
    /// Route a plugin-declared scaffold command.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

/// Flags for `sunscreen scaffold crud`.
#[derive(Debug, Args)]
pub struct CrudArgs {
    /// Resource name. Used as the account name and instruction suffix.
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
    /// Comma-separated resource fields, e.g. `"authority:Pubkey,title:String,body:String"`.
    #[arg(
        long,
        value_name = "LIST",
        default_value = "authority:Pubkey,title:String,body:String,published:bool"
    )]
    pub fields: String,
    /// Skip `update_<resource>`.
    #[arg(long, default_value_t = false)]
    pub no_update: bool,
    /// Skip `delete_<resource>`.
    #[arg(long, default_value_t = false)]
    pub no_delete: bool,
    /// Skip generated event structs and instruction `emit!` stubs.
    #[arg(long, default_value_t = false)]
    pub no_events: bool,
    /// Skip recipe frontend hooks even when the workspace has a frontend.
    #[arg(long, default_value_t = false)]
    pub no_frontend: bool,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Shared flags for built-in recipe scaffolders.
#[derive(Debug, Args)]
pub struct BuiltinRecipeArgs {
    /// Recipe resource name.
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Flags for `sunscreen scaffold program`.
#[derive(Debug, Args)]
pub struct ProgramArgs {
    /// Program name. Stored kebab-case in `sunscreen.yml`, but snake_case
    /// in `Anchor.toml` (`<snake> = "<pubkey>"`) and on disk
    /// (`programs/<snake>/`) to match Rust crate / module conventions.
    pub name: String,
    /// Optional Anchor program ID (base58 pubkey). Defaults to the canonical
    /// dummy ID — replace later via `solana-keygen new -o target/deploy/...`
    /// and `anchor keys sync`.
    #[arg(long, value_name = "PUBKEY")]
    pub id: Option<String>,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Flags for `sunscreen scaffold account`.
#[derive(Debug, Args)]
pub struct AccountArgs {
    /// Account struct name (PascalCased in source; folded to snake_case for
    /// the file/module name).
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`). Auto-detected when only one program exists.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Comma-separated struct fields, e.g. `"owner:Pubkey,total:u64"`.
    #[arg(long, value_name = "LIST", default_value = "")]
    pub fields: String,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Flags for `sunscreen scaffold event`.
#[derive(Debug, Args)]
pub struct EventArgs {
    /// Event struct name.
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`). Auto-detected when only one program exists.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Comma-separated struct fields, e.g. `"amount:u64,user:Pubkey"`.
    #[arg(long, value_name = "LIST", default_value = "")]
    pub fields: String,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Flags for `sunscreen scaffold error`.
#[derive(Debug, Args)]
pub struct ErrorArgs {
    /// Error variant name.
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`). Auto-detected when only one program exists.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Human-readable message bound to `#[msg("…")]`.
    #[arg(long, value_name = "STRING", default_value = "")]
    pub msg: String,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Flags for `sunscreen scaffold instruction`.
#[derive(Debug, Args)]
pub struct InstructionArgs {
    /// Instruction name. Must start with a letter and contain only `[a-zA-Z0-9_]`.
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`). Auto-detected when only one program exists.
    #[arg(long, value_name = "NAME")]
    pub program: Option<String>,
    /// Comma-separated handler args, e.g. `"amount:u64,memo:String"`.
    #[arg(long, value_name = "LIST", default_value = "")]
    pub args: String,
    /// Comma-separated accounts, e.g. `"vault:mut|signer,user:signer|seeds=b\"vault\";user.key()"`.
    ///
    /// Per-account flags can be pipe-separated or ADR-style colon-separated.
    /// Supported flags: `mut`, `signer`, `system`, `token`, `assoc_token`,
    /// `ata`, `seeds=<expr>[;<expr>...]`. Shorthands such as `system_program`,
    /// `token_program`, and `associated_token_program` infer program accounts.
    #[arg(long, value_name = "LIST", default_value = "")]
    pub accounts: String,
    /// If set, emit `<EventName>` from the handler.
    #[arg(long, value_name = "NAME")]
    pub emit: Option<String>,
    /// Print the planned changes without touching disk.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Dispatch entry invoked from `cli::root`.
pub fn run(cmd: &ScaffoldCmd, json: bool) -> Result<i32, SunscreenError> {
    match cmd {
        ScaffoldCmd::Instruction(args) => run_instruction(args, json, false),
        ScaffoldCmd::Account(args) => run_account(args, json, false),
        ScaffoldCmd::Event(args) => run_event(args, json, false),
        ScaffoldCmd::Error(args) => run_error(args, json, false),
        ScaffoldCmd::Program(args) => run_program(args, json, false),
        ScaffoldCmd::Crud(args) => run_crud(args, json),
        ScaffoldCmd::SplToken(args) => run_spl_token(args, json),
        ScaffoldCmd::MetaplexNft(args) => run_metaplex_nft(args, json),
        ScaffoldCmd::External(args) => run_external(args, json),
    }
}

fn run_external(args: &[OsString], json: bool) -> Result<i32, SunscreenError> {
    let Some(command) = args.first() else {
        return Err(SunscreenError::UserInput(
            "missing scaffold plugin command".to_string(),
        ));
    };
    let command = command.to_string_lossy().into_owned();
    let forwarded = args
        .iter()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let manager = PluginManager::discover_current_workspace()?;
    let report = manager.run_scaffold_command(&command, &forwarded)?;
    if json {
        println!(
            "{}",
            manager::report_json(&report, format!("scaffold {command}"))
        );
    } else {
        println!(
            "plugin {plugin} scaffolded {command}",
            plugin = report.plugin
        );
    }
    Ok(0)
}

#[cfg(feature = "onboarding")]
pub(crate) fn run_crud_quiet(
    args: &CrudArgs,
    workspace_root: &Path,
) -> Result<i32, SunscreenError> {
    run_crud_impl(args, false, true, Some(workspace_root))
}

#[cfg(feature = "onboarding")]
pub(crate) fn run_spl_token_quiet(
    args: &BuiltinRecipeArgs,
    workspace_root: &Path,
) -> Result<i32, SunscreenError> {
    run_spl_token_impl(args, false, true, Some(workspace_root))
}

#[cfg(feature = "onboarding")]
pub(crate) fn run_metaplex_nft_quiet(
    args: &BuiltinRecipeArgs,
    workspace_root: &Path,
) -> Result<i32, SunscreenError> {
    run_metaplex_nft_impl(args, false, true, Some(workspace_root))
}

fn run_crud(args: &CrudArgs, json: bool) -> Result<i32, SunscreenError> {
    run_crud_impl(args, json, false, None)
}

fn run_crud_impl(
    args: &CrudArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "resource name")?;
    let _ = parse_fields(&args.fields)?;
    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;
    let frontend_root = if args.no_frontend || ws.config.workspace.frontend == Frontend::None {
        None
    } else {
        Some(
            ws.config
                .workspace
                .frontend_path
                .clone()
                .unwrap_or_else(|| "app".to_string()),
        )
    };
    let plan = build_crud_recipe(CrudRecipeOptions {
        resource: args.name.clone(),
        fields: args.fields.clone(),
        include_update: !args.no_update,
        include_delete: !args.no_delete,
        include_events: !args.no_events,
        include_frontend: !args.no_frontend,
        frontend_root,
    });
    execute_recipe(
        &ws.root,
        program,
        &args.program,
        plan,
        args.dry_run,
        json,
        quiet,
    )
}

fn run_spl_token(args: &BuiltinRecipeArgs, json: bool) -> Result<i32, SunscreenError> {
    run_spl_token_impl(args, json, false, None)
}

fn run_spl_token_impl(
    args: &BuiltinRecipeArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "recipe name")?;
    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;
    let plan = build_spl_token_recipe(SplTokenRecipeOptions {
        name: args.name.clone(),
    });
    execute_recipe(
        &ws.root,
        program,
        &args.program,
        plan,
        args.dry_run,
        json,
        quiet,
    )
}

fn run_metaplex_nft(args: &BuiltinRecipeArgs, json: bool) -> Result<i32, SunscreenError> {
    run_metaplex_nft_impl(args, json, false, None)
}

fn run_metaplex_nft_impl(
    args: &BuiltinRecipeArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "recipe name")?;
    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;
    let plan = build_metaplex_nft_recipe(MetaplexNftRecipeOptions {
        name: args.name.clone(),
    });
    execute_recipe(
        &ws.root,
        program,
        &args.program,
        plan,
        args.dry_run,
        json,
        quiet,
    )
}

fn execute_recipe(
    workspace_root: &Path,
    program: &ProgramView,
    program_arg: &str,
    plan: RecipePlan,
    dry_run: bool,
    json: bool,
    quiet: bool,
) -> Result<i32, SunscreenError> {
    let program_dir_name = program
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&program.name);
    let files = recipe_files(program_dir_name, &plan)?;

    // Preflight all primitive mutations first. This catches marker drift,
    // existing-name conflicts, and parse errors before any recipe step writes.
    for step in &plan.steps {
        execute_recipe_step(workspace_root, program_arg, step, true)?;
    }
    preflight_generated_files(workspace_root, &files)?;

    let before = snapshot_workspace(workspace_root)?;
    if !dry_run {
        for step in &plan.steps {
            execute_recipe_step(workspace_root, program_arg, step, false)?;
        }
        write_generated_files(workspace_root, &files)?;
    }
    let changed_files = if dry_run {
        files
            .iter()
            .map(|file| file.relative_path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>()
    } else {
        let after = snapshot_workspace(workspace_root)?;
        diff_snapshots(&before, &after)
    };
    let unchanged = !dry_run && changed_files.is_empty();

    if !quiet {
        emit_recipe_result(
            json,
            &RecipeResult {
                recipe: plan.kind.as_str(),
                resource: &plan.resource,
                program: program_arg,
                dry_run,
                unchanged,
                files: &changed_files,
                steps: plan.steps.len(),
            },
        );
    }
    Ok(0)
}

fn execute_recipe_step(
    workspace_root: &Path,
    program: &str,
    step: &RecipeStep,
    dry_run: bool,
) -> Result<(), SunscreenError> {
    match step {
        RecipeStep::Account { name, fields } => run_account_in_workspace(
            &AccountArgs {
                name: name.clone(),
                program: Some(program.to_string()),
                fields: fields.clone(),
                dry_run,
            },
            false,
            true,
            Some(workspace_root),
        )
        .map(|_| ()),
        RecipeStep::Event { name, fields } => run_event_in_workspace(
            &EventArgs {
                name: name.clone(),
                program: Some(program.to_string()),
                fields: fields.clone(),
                dry_run,
            },
            false,
            true,
            Some(workspace_root),
        )
        .map(|_| ()),
        RecipeStep::Error { name, message } => run_error_in_workspace(
            &ErrorArgs {
                name: name.clone(),
                program: Some(program.to_string()),
                msg: message.clone(),
                dry_run,
            },
            false,
            true,
            Some(workspace_root),
        )
        .map(|_| ()),
        RecipeStep::Instruction {
            name,
            args,
            accounts,
            emit,
        } => run_instruction_in_workspace(
            &InstructionArgs {
                name: name.clone(),
                program: Some(program.to_string()),
                args: args.clone(),
                accounts: accounts.clone(),
                emit: emit.clone(),
                dry_run,
            },
            false,
            true,
            Some(workspace_root),
        )
        .map(|_| ()),
    }
}

fn recipe_files(
    program_dir_name: &str,
    plan: &RecipePlan,
) -> Result<Vec<GeneratedFile>, SunscreenError> {
    let mut files = Vec::with_capacity(plan.files.len());
    for file in &plan.files {
        let replaced = file
            .relative_path
            .to_string_lossy()
            .replace("__PROGRAM__", program_dir_name);
        ensure_safe_recipe_path(&replaced)?;
        files.push(GeneratedFile {
            relative_path: PathBuf::from(replaced),
            contents: file.contents.clone(),
        });
    }
    Ok(files)
}

fn preflight_generated_files(
    workspace_root: &Path,
    files: &[GeneratedFile],
) -> Result<(), SunscreenError> {
    for file in files {
        let abs = workspace_root.join(&file.relative_path);
        if abs.exists() {
            let current = std::fs::read_to_string(&abs).map_err(|err| {
                SunscreenError::Other(anyhow::anyhow!("read {}: {err}", abs.display()))
            })?;
            if current != file.contents {
                return Err(SunscreenError::UserInput(format!(
                    "recipe output {} already exists with different contents; edit it manually or remove it to regenerate",
                    file.relative_path.display()
                )));
            }
        }
    }
    Ok(())
}

fn write_generated_files(
    workspace_root: &Path,
    files: &[GeneratedFile],
) -> Result<(), SunscreenError> {
    let mut tx = Transaction::new(workspace_root).map_err(map_tx_err)?;
    for file in files {
        let abs = workspace_root.join(&file.relative_path);
        if abs.exists() {
            continue;
        }
        tx.stage(
            &file.relative_path.to_string_lossy().replace('\\', "/"),
            file.contents.as_bytes(),
        )
        .map_err(map_tx_err)?;
    }
    tx.commit().map_err(map_tx_err)?;
    Ok(())
}

fn ensure_safe_recipe_path(path: &str) -> Result<(), SunscreenError> {
    let path = Path::new(path);
    let unsafe_path = path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::Prefix(_)
                    | std::path::Component::RootDir
            )
        });
    if unsafe_path {
        return Err(SunscreenError::UserInput(format!(
            "recipe output path must stay inside the workspace: {}",
            path.display()
        )));
    }
    Ok(())
}

fn snapshot_workspace(workspace_root: &Path) -> Result<BTreeMap<String, Vec<u8>>, SunscreenError> {
    let mut out = BTreeMap::new();
    collect_snapshot(workspace_root, workspace_root, &mut out)?;
    Ok(out)
}

fn collect_snapshot(
    workspace_root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), SunscreenError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("read {}: {err}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|err| SunscreenError::Other(anyhow::anyhow!(err)))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if matches!(file_name, "target" | "node_modules" | ".git") {
            continue;
        }
        if path.is_dir() {
            collect_snapshot(workspace_root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path).map_err(|err| {
                SunscreenError::Other(anyhow::anyhow!("read {}: {err}", path.display()))
            })?;
            out.insert(rel, bytes);
        }
    }
    Ok(())
}

fn diff_snapshots(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let mut changed = Vec::new();
    for (path, bytes) in after {
        if before.get(path) != Some(bytes) {
            changed.push(path.clone());
        }
    }
    changed
}

struct RecipeResult<'a> {
    recipe: &'a str,
    resource: &'a str,
    program: &'a str,
    dry_run: bool,
    unchanged: bool,
    files: &'a [String],
    steps: usize,
}

fn emit_recipe_result(json: bool, result: &RecipeResult<'_>) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "recipe": result.recipe,
            "resource": result.resource,
            "program": result.program,
            "dry_run": result.dry_run,
            "unchanged": result.unchanged,
            "steps": result.steps,
            "files": result.files,
            "written": if result.dry_run { 0 } else { result.files.len() },
        });
        println!("{payload}");
    } else if result.dry_run {
        println!(
            "dry-run: would scaffold {} recipe `{}` in `{}`",
            result.recipe, result.resource, result.program
        );
        for f in result.files {
            println!("  {f}");
        }
    } else if result.unchanged {
        println!(
            "scaffold {} recipe `{}`: unchanged (idempotent no-op)",
            result.recipe, result.resource
        );
    } else {
        println!(
            "scaffolded {recipe} recipe `{resource}` for program `{program}` ({} files changed)",
            result.files.len(),
            recipe = result.recipe,
            resource = result.resource,
            program = result.program
        );
        for f in result.files {
            println!("  {f}");
        }
    }
}

fn run_instruction(args: &InstructionArgs, json: bool, quiet: bool) -> Result<i32, SunscreenError> {
    run_instruction_in_workspace(args, json, quiet, None)
}

fn run_instruction_in_workspace(
    args: &InstructionArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "instruction name")?;
    if let Some(emit) = &args.emit {
        validate_ident(emit, "--emit event name")?;
    }

    let parsed_args = parse_args(&args.args)?;
    let parsed_accounts = parse_accounts(&args.accounts)?;

    // 1. Locate workspace.
    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program_name = resolve_program(args.program.as_deref(), &ws)?;
    let ix_snake = args.name.to_snake_case();
    let program_snake = program_name.to_snake_case();
    let program: &ProgramView = workspace::find_program(&ws, &program_name).map_err(map_ws_err)?;
    let emit_fields = if let Some(emit) = &args.emit {
        event_fields_for_emit(program, emit)?
    } else {
        Vec::new()
    };

    // 2. Render instruction body.
    let ctx = InstructionCtx {
        program_name: program_snake.clone(),
        instruction_name: ix_snake.clone(),
        args: parsed_args.clone(),
        accounts: parsed_accounts,
        emit: args.emit.clone(),
        emit_fields,
    };
    let instruction_body = render_instruction(&ctx)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render instruction: {e}")))?;

    // 3. Plan file changes.
    let ix_abs = program.instructions_dir.join(format!("{ix_snake}.rs"));
    let ix_rel = relative_to(&ws.root, &program.instructions_dir).join(format!("{ix_snake}.rs"));
    let mod_rel = relative_to(&ws.root, &program.instructions_mod_rs);
    let lib_rel = relative_to(&ws.root, &program.lib_rs);

    // Idempotency: if the instruction file already exists, compare ONLY the
    // auto-generated `segment=file` region. The `segment=handler` user-region
    // is owned by the user and must be preserved across re-runs.
    // - Auto-generated region identical → unchanged (no-op).
    // - Auto-generated region differs → re-apply just that patch, preserving
    //   the user-region content verbatim.
    //
    // `ix_existing_patched`, when `Some`, holds the rewritten file contents to
    // be written back (with the user-region preserved).
    let ix_plan = plan_instruction_file(&ix_abs, &ix_rel, &instruction_body)?;

    // 4. Compute new instructions/mod.rs content.
    let all_instructions = merged_instruction_names(program, &ix_snake);
    let mod_segment_body = render_instructions_mod_segment(&all_instructions);
    let (new_mod_contents, mod_action) =
        build_mod_rs(&program.instructions_mod_rs, &mod_segment_body)?;

    // Detect mod.rs no-op: if file exists and would be identical, skip.
    let mod_status =
        status_after_action(&program.instructions_mod_rs, &new_mod_contents, mod_action);

    // 5. Compute new lib.rs content (best-effort; skip if marker missing).
    //
    // Recover existing dispatch entries (with their args) from the current
    // lib.rs `dispatch` segment so previously-scaffolded instructions keep
    // their argument lists. Then merge in the new instruction, deduplicating
    // by name (the new entry's args win for that name).
    let existing_dispatches = read_existing_dispatches(&program.lib_rs)?;
    let dispatches = merge_instruction_dispatches(
        &all_instructions,
        &ix_snake,
        &parsed_args,
        &existing_dispatches,
    );
    let dispatch_body = render_dispatch_segment(&program_snake, &dispatches);
    let (lib_new, lib_status) = try_patch_lib_rs(&program.lib_rs, &dispatch_body)?;

    // Detect lib.rs no-op: if patched content equals current, mark unchanged.
    let lib_file_status: FileStatus = if lib_status.patched {
        let current = std::fs::read_to_string(&program.lib_rs).unwrap_or_default();
        if current == lib_new {
            FileStatus::Unchanged
        } else {
            FileStatus::Updated
        }
    } else {
        FileStatus::Skipped
    };

    // 6. Stage everything via Transaction.
    // Normalize separators to '/' for stable JSON output on all platforms.
    let plan_files = instruction_plan_files(&ix_rel, &mod_rel, &lib_rel, lib_status.patched);

    let unchanged = ix_plan.status == FileStatus::Unchanged
        && mod_status == FileStatus::Unchanged
        && lib_file_status != FileStatus::Updated;

    if args.dry_run {
        if !quiet {
            emit_dry_run(json, &args.name, &program_name, &plan_files, &lib_status);
        }
        return Ok(0);
    }

    // Atomic commit: stage all three file changes (new instruction file +
    // in-place mod.rs / lib.rs patches) into one transaction so they land
    // together or not at all. New files use `stage`; in-place rewrites use
    // `stage_replace` (which captures originals for rollback).
    let written = commit_instruction_changes(InstructionCommitPlan {
        root: &ws.root,
        ix_abs: &ix_abs,
        ix_rel: &ix_rel,
        instruction_body: &instruction_body,
        ix_plan: &ix_plan,
        mod_abs: &program.instructions_mod_rs,
        mod_rel: &mod_rel,
        mod_contents: &new_mod_contents,
        mod_action,
        mod_status,
        lib_abs: &program.lib_rs,
        lib_contents: &lib_new,
        lib_status: lib_file_status,
    })?;

    if quiet {
        return Ok(0);
    }

    emit_instruction_result(InstructionResult {
        json,
        ix_snake: &ix_snake,
        program_name: &program_name,
        plan_files: &plan_files,
        lib_rel: &lib_rel,
        lib_status,
        ix_status: ix_plan.status,
        mod_status,
        lib_file_status,
        unchanged,
        written_count: written.len(),
    });
    Ok(0)
}

struct InstructionFilePlan {
    status: FileStatus,
    patched: Option<String>,
}

fn plan_instruction_file(
    ix_abs: &Path,
    ix_rel: &Path,
    instruction_body: &str,
) -> Result<InstructionFilePlan, SunscreenError> {
    if !ix_abs.exists() {
        return Ok(InstructionFilePlan {
            status: FileStatus::Created,
            patched: None,
        });
    }

    let existing = std::fs::read_to_string(ix_abs)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", ix_abs.display())))?;
    let rendered_file_body = extract_segment_body(instruction_body, "file").ok_or_else(|| {
        SunscreenError::Other(anyhow::anyhow!(
            "rendered instruction missing `segment=file` markers"
        ))
    })?;
    match extract_segment_body(&existing, "file") {
        None => plan_unmarked_instruction_file(&existing, instruction_body, ix_rel),
        Some(existing_body) if existing_body == rendered_file_body => Ok(InstructionFilePlan {
            status: FileStatus::Unchanged,
            patched: None,
        }),
        Some(_) => repatch_instruction_file(&existing, &rendered_file_body),
    }
}

fn plan_unmarked_instruction_file(
    existing: &str,
    instruction_body: &str,
    ix_rel: &Path,
) -> Result<InstructionFilePlan, SunscreenError> {
    if existing == instruction_body {
        return Ok(InstructionFilePlan {
            status: FileStatus::Unchanged,
            patched: None,
        });
    }
    Err(SunscreenError::InstructionDrift {
        path: ix_rel.to_string_lossy().into_owned(),
        hint: "instruction file has no auto-generated markers; restore from VCS or delete to regenerate"
            .to_string(),
    })
}

fn repatch_instruction_file(
    existing: &str,
    rendered_file_body: &str,
) -> Result<InstructionFilePlan, SunscreenError> {
    let body_lines: Vec<String> = rendered_file_body
        .strip_suffix('\n')
        .unwrap_or(rendered_file_body)
        .split('\n')
        .map(str::to_string)
        .collect();
    let patches = vec![Patch {
        segment: "file".to_string(),
        lines: body_lines,
    }];
    let patched = apply(existing, &patches).map_err(map_patch_err)?;
    Ok(InstructionFilePlan {
        status: FileStatus::Updated,
        patched: Some(patched),
    })
}

fn merged_instruction_names(program: &ProgramView, ix_snake: &str) -> Vec<String> {
    let mut names = list_existing_instructions(program);
    if !names.iter().any(|name| name == ix_snake) {
        names.push(ix_snake.to_string());
    }
    names.sort();
    names.dedup();
    names
}

fn merge_instruction_dispatches(
    all_instructions: &[String],
    ix_snake: &str,
    parsed_args: &[ArgSpec],
    existing_dispatches: &[InstructionDispatch],
) -> Vec<InstructionDispatch> {
    all_instructions
        .iter()
        .map(|name| InstructionDispatch {
            name: name.clone(),
            args: dispatch_args_for(name, ix_snake, parsed_args, existing_dispatches),
        })
        .collect()
}

fn dispatch_args_for(
    name: &str,
    ix_snake: &str,
    parsed_args: &[ArgSpec],
    existing_dispatches: &[InstructionDispatch],
) -> Vec<ArgSpec> {
    if name == ix_snake {
        return parsed_args.to_vec();
    }
    existing_dispatches
        .iter()
        .find(|dispatch| dispatch.name == name)
        .map(|dispatch| dispatch.args.clone())
        .unwrap_or_default()
}

fn instruction_plan_files(
    ix_rel: &Path,
    mod_rel: &Path,
    lib_rel: &Path,
    lib_patched: bool,
) -> Vec<String> {
    let mut files = vec![to_fwd_path(ix_rel), to_fwd_path(mod_rel)];
    if lib_patched {
        files.push(to_fwd_path(lib_rel));
    }
    files
}

#[derive(Clone, Copy)]
struct InstructionCommitPlan<'a> {
    root: &'a Path,
    ix_abs: &'a Path,
    ix_rel: &'a Path,
    instruction_body: &'a str,
    ix_plan: &'a InstructionFilePlan,
    mod_abs: &'a Path,
    mod_rel: &'a Path,
    mod_contents: &'a str,
    mod_action: ModAction,
    mod_status: FileStatus,
    lib_abs: &'a Path,
    lib_contents: &'a str,
    lib_status: FileStatus,
}

fn commit_instruction_changes(
    plan: InstructionCommitPlan<'_>,
) -> Result<Vec<PathBuf>, SunscreenError> {
    let mut tx = Transaction::new(plan.root).map_err(map_tx_err)?;
    stage_instruction_file(&mut tx, plan)?;
    stage_host_change(
        &mut tx,
        plan.mod_rel,
        plan.mod_abs,
        plan.mod_contents,
        plan.mod_action,
        plan.mod_status,
    )?;
    if plan.lib_status == FileStatus::Updated {
        tx.stage_replace(plan.lib_abs, plan.lib_contents.as_bytes())
            .map_err(map_tx_err)?;
    }
    tx.commit().map_err(map_tx_err)
}

fn stage_instruction_file(
    tx: &mut Transaction,
    plan: InstructionCommitPlan<'_>,
) -> Result<(), SunscreenError> {
    match plan.ix_plan.status {
        FileStatus::Created => tx
            .stage(&to_fwd_path(plan.ix_rel), plan.instruction_body.as_bytes())
            .map_err(map_tx_err),
        FileStatus::Updated => {
            if let Some(patched) = &plan.ix_plan.patched {
                tx.stage_replace(plan.ix_abs, patched.as_bytes())
                    .map_err(map_tx_err)?;
            }
            Ok(())
        }
        FileStatus::Unchanged | FileStatus::Skipped => Ok(()),
    }
}

struct InstructionResult<'a> {
    json: bool,
    ix_snake: &'a str,
    program_name: &'a str,
    plan_files: &'a [String],
    lib_rel: &'a Path,
    lib_status: LibPatchStatus,
    ix_status: FileStatus,
    mod_status: FileStatus,
    lib_file_status: FileStatus,
    unchanged: bool,
    written_count: usize,
}

fn emit_instruction_result(result: InstructionResult<'_>) {
    if result.json {
        let payload = serde_json::json!({
            "ok": true,
            "instruction": result.ix_snake,
            "program": result.program_name,
            "files": result.plan_files,
            "lib_rs_patched": result.lib_status.patched,
            "unchanged": result.unchanged,
            "instruction_file": result.ix_status.as_str(),
            "mod_file": result.mod_status.as_str(),
            "lib_file": result.lib_file_status.as_str(),
            "written": instruction_written_count(&result),
        });
        println!("{payload}");
    } else if result.unchanged {
        println!(
            "scaffold instruction `{}`: unchanged (idempotent no-op)",
            result.ix_snake
        );
    } else {
        emit_instruction_text_result(&result);
    }
}

fn instruction_written_count(result: &InstructionResult<'_>) -> usize {
    result.written_count
        + usize::from(result.mod_status == FileStatus::Updated)
        + usize::from(result.lib_file_status == FileStatus::Updated)
        + usize::from(result.ix_status == FileStatus::Updated)
}

fn emit_instruction_text_result(result: &InstructionResult<'_>) {
    println!(
        "scaffolded instruction `{}` for program `{}` ({} files)",
        result.ix_snake,
        result.program_name,
        result.plan_files.len()
    );
    for file in result.plan_files {
        println!("  {file}");
    }
    if !result.lib_status.patched {
        println!(
            "warning: {} has no `dispatch` segment marker — skipped",
            result.lib_rel.display()
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileStatus {
    Created,
    Updated,
    Unchanged,
    Skipped,
}

impl FileStatus {
    fn as_str(self) -> &'static str {
        match self {
            FileStatus::Created => "created",
            FileStatus::Updated => "updated",
            FileStatus::Unchanged => "unchanged",
            FileStatus::Skipped => "skipped",
        }
    }
}

fn to_fwd_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn status_after_action(path: &Path, new_contents: &str, action: ModAction) -> FileStatus {
    match action {
        ModAction::Create => FileStatus::Created,
        ModAction::Replace => {
            let current = std::fs::read_to_string(path).unwrap_or_default();
            if current == new_contents {
                FileStatus::Unchanged
            } else {
                FileStatus::Updated
            }
        }
    }
}

fn status_for_rendered_path(path: &Path, new_contents: &str) -> FileStatus {
    if !path.exists() {
        return FileStatus::Created;
    }
    let current = std::fs::read_to_string(path).unwrap_or_default();
    if current == new_contents {
        FileStatus::Unchanged
    } else {
        FileStatus::Updated
    }
}

fn stage_host_change(
    tx: &mut Transaction,
    rel_path: &Path,
    abs_path: &Path,
    contents: &str,
    action: ModAction,
    status: FileStatus,
) -> Result<(), SunscreenError> {
    match (action, status) {
        (_, FileStatus::Unchanged) => Ok(()),
        (ModAction::Create, _) => tx
            .stage(&to_fwd_path(rel_path), contents.as_bytes())
            .map_err(map_tx_err),
        (ModAction::Replace, _) => tx
            .stage_replace(abs_path, contents.as_bytes())
            .map_err(map_tx_err),
    }
}

fn stage_optional_lib_change(
    tx: &mut Transaction,
    lib_rs: &Path,
    contents: Option<&String>,
) -> Result<(), SunscreenError> {
    if let Some(contents) = contents {
        tx.stage_replace(lib_rs, contents.as_bytes())
            .map_err(map_tx_err)?;
    }
    Ok(())
}

fn emit_dry_run(
    json: bool,
    instruction: &str,
    program: &str,
    files: &[String],
    lib_status: &LibPatchStatus,
) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "instruction": instruction,
            "program": program,
            "files": files,
            "lib_rs_patched": lib_status.patched,
        });
        println!("{payload}");
    } else {
        println!("dry-run: would scaffold `{instruction}` in `{program}`");
        for f in files {
            println!("  {f}");
        }
        if !lib_status.patched {
            println!("  (lib.rs `dispatch` segment marker missing — patch skipped)");
        }
    }
}

// ---------------------------------------------------------------------------
// Args / accounts parsing
// ---------------------------------------------------------------------------

fn parse_args(raw: &str) -> Result<Vec<ArgSpec>, SunscreenError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, ty) = entry.split_once(':').ok_or_else(|| {
            SunscreenError::UserInput(format!(
                "invalid --args entry `{entry}` (expected `name:type`)"
            ))
        })?;
        let name = name.trim();
        let ty = ty.trim();
        validate_ident(name, "arg name")?;
        if ty.is_empty() {
            return Err(SunscreenError::UserInput(format!(
                "arg `{name}` has empty type"
            )));
        }
        out.push(ArgSpec {
            name: name.to_string(),
            ty: ty.to_string(),
        });
    }
    Ok(out)
}

fn parse_accounts(raw: &str) -> Result<Vec<AccountSpec>, SunscreenError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        // Format: `name[:flag1|flag2|seeds=...]`.
        let (name, flag_str) = match entry.split_once(':') {
            Some((n, f)) => (n.trim(), f),
            None => (entry, ""),
        };
        validate_ident(name, "account name")?;
        let mut spec = AccountSpec {
            name: name.to_string(),
            mutable: false,
            signer: false,
            seeds: None,
            kind: account_kind_for_shorthand(name).unwrap_or(AccountKind::Generic),
        };

        for flag in split_account_flags(flag_str) {
            let flag = flag.trim();
            if flag.is_empty() {
                continue;
            }
            match flag {
                "mut" => spec.mutable = true,
                "signer" => spec.signer = true,
                "system" => spec.kind = AccountKind::SystemProgram,
                "token" => spec.kind = AccountKind::TokenProgram,
                "assoc_token" | "ata" => spec.kind = AccountKind::AssociatedTokenProgram,
                other => {
                    if let Some(seed_list) = other.strip_prefix("seeds=") {
                        let seeds: Vec<String> = seed_list
                            .split(';')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if seeds.is_empty() {
                            return Err(SunscreenError::UserInput(format!(
                                "`seeds=` for account `{name}` requires at least one seed expression"
                            )));
                        }
                        spec.seeds = Some(seeds);
                    } else {
                        return Err(SunscreenError::UserInput(format!(
                            "unknown account flag `{other}` for `{name}`"
                        )));
                    }
                }
            }
        }
        out.push(spec);
    }
    Ok(out)
}

fn account_kind_for_shorthand(name: &str) -> Option<AccountKind> {
    match name {
        "system_program" => Some(AccountKind::SystemProgram),
        "token_program" => Some(AccountKind::TokenProgram),
        "associated_token_program" | "associated_token" => {
            Some(AccountKind::AssociatedTokenProgram)
        }
        _ => None,
    }
}

fn split_account_flags(raw: &str) -> Vec<&str> {
    if raw.contains('|') {
        return raw.split('|').collect();
    }
    if let Some(seeds_start) = raw.find("seeds=") {
        let mut flags: Vec<&str> = raw[..seeds_start]
            .trim_end_matches(':')
            .split(':')
            .filter(|part| !part.is_empty())
            .collect();
        flags.push(&raw[seeds_start..]);
        flags
    } else {
        raw.split(':').collect()
    }
}

// ---------------------------------------------------------------------------
// mod.rs / lib.rs editing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModAction {
    Create,
    Replace,
}

fn build_mod_rs(
    path: &std::path::Path,
    segment_body: &str,
) -> Result<(String, ModAction), SunscreenError> {
    if !path.exists() {
        // Create a fresh mod.rs with markers and the segment populated.
        let mut out = String::new();
        out.push_str("//! Auto-generated by sunscreen. Edit user regions only.\n\n");
        out.push_str("// === sunscreen:auto-generated:begin segment=instructions version=1 generator=cli ===\n");
        out.push_str(segment_body);
        out.push_str("// === sunscreen:auto-generated:end segment=instructions ===\n");
        return Ok((out, ModAction::Create));
    }
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    // If there's no marker at all, prepend one (idempotent on re-runs).
    let markers = scan(&existing).map_err(map_patch_err)?;
    let has_segment = markers
        .iter()
        .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == "instructions");
    if !has_segment {
        // Splice in a fresh marker block at the end.
        let mut out = existing.clone();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\n// === sunscreen:auto-generated:begin segment=instructions version=1 generator=cli ===\n");
        out.push_str(segment_body);
        out.push_str("// === sunscreen:auto-generated:end segment=instructions ===\n");
        return Ok((out, ModAction::Replace));
    }
    // Use rustpatch::apply.
    let body_lines: Vec<String> = segment_body
        .strip_suffix('\n')
        .unwrap_or(segment_body)
        .split('\n')
        .map(str::to_string)
        .collect();
    let patches = vec![Patch {
        segment: "instructions".to_string(),
        lines: body_lines,
    }];
    let patched = apply(&existing, &patches).map_err(map_patch_err)?;
    Ok((patched, ModAction::Replace))
}

#[derive(Debug, Clone, Copy)]
struct LibPatchStatus {
    patched: bool,
}

/// Ensure `pub mod <name>;` appears at the top level of `lib.rs`. Returns
/// `Some(new_contents)` when the file changed, `None` when the declaration
/// already exists (or `lib.rs` is missing — caller's responsibility).
///
/// Insertion site: immediately after the existing `pub mod instructions;`
/// line (the canonical anchor produced by `chain new`). If that line is
/// absent, insert before the first non-comment, non-`use`, non-blank line.
/// This is a marker-free heuristic: top-level `pub mod` declarations are
/// stable, idempotent text and don't require auto-generated marker blocks.
fn ensure_lib_mod_decl(
    lib_rs: &std::path::Path,
    mod_name: &str,
) -> Result<Option<String>, SunscreenError> {
    if !lib_rs.exists() {
        return Ok(None);
    }
    let existing = std::fs::read_to_string(lib_rs)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", lib_rs.display())))?;
    let needle = format!("pub mod {mod_name};");
    // Match exact line (ignoring leading whitespace) to avoid false positives
    // like `pub mod events_foo;`.
    if existing.lines().any(|l| l.trim() == needle) {
        return Ok(None);
    }
    let nl = detect_line_ending(&existing);
    let trailing_nl = existing.ends_with('\n');
    let lines: Vec<&str> = existing.lines().collect();
    // Anchor: line after `pub mod instructions;` (canonical chain-new layout).
    let anchor = lines
        .iter()
        .position(|l| l.trim() == "pub mod instructions;")
        .map(|i| i + 1)
        // Fallback: first line that isn't `use ...`, comment, or blank.
        .or_else(|| {
            lines.iter().position(|l| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with("//") && !t.starts_with("use ")
            })
        })
        .unwrap_or(lines.len());
    let mut out = String::new();
    let total = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if i == anchor {
            out.push_str(&needle);
            out.push_str(nl);
        }
        out.push_str(l);
        // Preserve the original trailing-newline behaviour: emit a newline
        // after every line except possibly the last.
        if i + 1 < total || trailing_nl {
            out.push_str(nl);
        }
    }
    if anchor >= total {
        if !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str(&needle);
        if trailing_nl {
            out.push_str(nl);
        }
    }
    Ok(Some(out))
}

fn try_patch_lib_rs(
    path: &std::path::Path,
    segment_body: &str,
) -> Result<(String, LibPatchStatus), SunscreenError> {
    if !path.exists() {
        return Ok((String::new(), LibPatchStatus { patched: false }));
    }
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    let markers = scan(&existing).map_err(map_patch_err)?;
    let has_dispatch = markers
        .iter()
        .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == "dispatch");
    if !has_dispatch {
        return Ok((existing, LibPatchStatus { patched: false }));
    }
    let body_lines: Vec<String> = segment_body
        .strip_suffix('\n')
        .unwrap_or(segment_body)
        .split('\n')
        .map(str::to_string)
        .collect();
    let patches = vec![Patch {
        segment: "dispatch".to_string(),
        lines: body_lines,
    }];
    let patched = apply(&existing, &patches).map_err(map_patch_err)?;
    Ok((patched, LibPatchStatus { patched: true }))
}

/// Extract the body (lines strictly between the begin/end markers) of an
/// auto-generated segment from `source`, re-joined with `\n` and terminated
/// with a trailing newline. Returns `None` if the segment is absent or the
/// source cannot be scanned.
fn extract_segment_body(source: &str, segment: &str) -> Option<String> {
    let markers = scan(source).ok()?;
    let m = markers
        .iter()
        .find(|m| m.kind == MarkerKind::AutoGenerated && m.segment == segment)?;
    let lines: Vec<&str> = source.lines().collect();
    // Body lives in (begin, end) exclusive of the marker lines themselves.
    let mut out = String::new();
    for line in &lines[m.begin + 1..m.end] {
        out.push_str(line);
        out.push('\n');
    }
    Some(out)
}

/// Recover existing dispatch entries from a program's `lib.rs` `dispatch`
/// segment. Returns an empty vec if the file or segment is missing.
///
/// Line-oriented parse: matches wrapper signatures of the form
/// `pub fn <name>(ctx: Context<Pascal>[, <arg>: <ty>...]) -> Result<()>`.
fn read_existing_dispatches(
    lib_rs: &std::path::Path,
) -> Result<Vec<InstructionDispatch>, SunscreenError> {
    if !lib_rs.exists() {
        return Ok(Vec::new());
    }
    let existing = std::fs::read_to_string(lib_rs)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", lib_rs.display())))?;
    let Some(body) = extract_segment_body(&existing, "dispatch") else {
        return Ok(Vec::new());
    };
    Ok(parse_dispatch_entries(&body))
}

/// Parse dispatch wrapper signature lines into [`InstructionDispatch`] entries.
fn parse_dispatch_entries(body: &str) -> Vec<InstructionDispatch> {
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("pub fn ") else {
            continue;
        };
        // rest looks like: `name(ctx: Context<Pascal>, a: u64, ...) -> Result<()> {`
        let Some(paren) = rest.find('(') else {
            continue;
        };
        let name = rest[..paren].trim().to_string();
        if name.is_empty() {
            continue;
        }
        let Some(close) = find_dispatch_param_close(rest, paren) else {
            continue;
        };
        let params = &rest[paren + 1..close];
        let args = parse_dispatch_args(params);
        out.push(InstructionDispatch { name, args });
    }
    out
}

fn find_dispatch_param_close(rest: &str, open: usize) -> Option<usize> {
    let chars: Vec<char> = rest[open + 1..].chars().collect();
    let mut depth = 0usize;
    for (idx, &ch) in chars.iter().enumerate() {
        match ch {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            ')' if depth == 0 => return Some(open + 1 + idx),
            _ => {}
        }
    }
    None
}

fn parse_dispatch_args(params: &str) -> Vec<ArgSpec> {
    params.split(',').filter_map(parse_dispatch_arg).collect()
}

fn parse_dispatch_arg(param: &str) -> Option<ArgSpec> {
    let param = param.trim();
    if param.is_empty() || param.starts_with("ctx:") || param.starts_with("ctx :") {
        return None;
    }
    let (arg_name, ty) = param.split_once(':')?;
    let arg_name = arg_name.trim();
    let ty = ty.trim();
    if arg_name.is_empty() || ty.is_empty() {
        return None;
    }
    Some(ArgSpec {
        name: arg_name.to_string(),
        ty: ty.to_string(),
    })
}

fn list_existing_instructions(program: &ProgramView) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&program.instructions_dir) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == "mod" {
            continue;
        }
        let ext_ok = path.extension().and_then(|e| e.to_str()) == Some("rs");
        if !ext_ok {
            continue;
        }
        out.push(stem.to_string());
    }
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn validate_ident(value: &str, what: &str) -> Result<(), SunscreenError> {
    if value.is_empty() {
        return Err(SunscreenError::UserInput(format!("{what} is empty")));
    }
    let first = value.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(SunscreenError::UserInput(format!(
            "{what} `{value}` must start with a letter or `_`"
        )));
    }
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(SunscreenError::UserInput(format!(
            "{what} `{value}` may only contain letters, digits, and `_`"
        )));
    }
    Ok(())
}

fn relative_to(root: &std::path::Path, target: &std::path::Path) -> PathBuf {
    target
        .strip_prefix(root)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|_| target.to_path_buf())
}

/// Resolve the program name from an optional CLI flag.
/// When `--program` is omitted and there is exactly one program in the workspace,
/// that program is used automatically (with a note printed to stderr).
/// When omitted and there are 0 or 2+ programs, a clear error is returned.
fn resolve_program(
    explicit: Option<&str>,
    ws: &workspace::WorkspaceRoot,
) -> Result<String, SunscreenError> {
    if let Some(name) = explicit {
        return Ok(name.to_string());
    }
    match ws.programs.as_slice() {
        [] => Err(SunscreenError::UserInput(
            "workspace has no programs; run `sunscreen scaffold program <name>` first".to_string(),
        )),
        [single] => {
            eprintln!("note: using program `{}`", single.name);
            Ok(single.name.clone())
        }
        _ => Err(SunscreenError::UserInput(format!(
            "workspace has {} programs; specify one with `--program <NAME>`",
            ws.programs.len()
        ))),
    }
}

fn map_ws_err(e: WorkspaceError) -> SunscreenError {
    match e {
        WorkspaceError::NotFound => SunscreenError::WorkspaceMissing(e.to_string()),
        other => SunscreenError::from(other),
    }
}

fn ensure_anchor_scaffolding(ws: &workspace::WorkspaceRoot) -> Result<(), SunscreenError> {
    if matches!(ws.config.project.framework, ConfigFramework::Anchor) {
        return Ok(());
    }
    Err(SunscreenError::UserInput(format!(
        "built-in scaffolders currently target Anchor workspaces; workspace framework is `{}`. \
         Use `sunscreen chain new --framework pinocchio` for the base program and route Pinocchio-specific scaffolding through a plugin.",
        framework_name(ws.config.project.framework)
    )))
}

fn framework_name(framework: ConfigFramework) -> &'static str {
    match framework {
        ConfigFramework::Anchor => "anchor",
        ConfigFramework::Pinocchio => "pinocchio",
        ConfigFramework::Shank => "shank",
    }
}

fn map_tx_err(e: TxError) -> SunscreenError {
    match e {
        TxError::PathEscape(p) => SunscreenError::UserInput(format!("invalid path: {p}")),
        TxError::DestinationExists(p) => {
            SunscreenError::UserInput(format!("destination already exists: {}", p.display()))
        }
        TxError::DuplicateStage(p) => {
            SunscreenError::Other(anyhow::anyhow!("duplicate staged path: {p}"))
        }
        TxError::Io(e) => SunscreenError::Other(anyhow::anyhow!(e)),
    }
}

fn map_patch_err(e: RustpatchError) -> SunscreenError {
    SunscreenError::Other(anyhow::anyhow!("rustpatch: {e}"))
}

// ---------------------------------------------------------------------------
// Field parsing (shared by account / event)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ParsedField {
    name: String,
    ty: String,
}

fn parse_fields(raw: &str) -> Result<Vec<ParsedField>, SunscreenError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, ty) = entry.split_once(':').ok_or_else(|| {
            SunscreenError::UserInput(format!(
                "invalid --fields entry `{entry}` (expected `name:type`)"
            ))
        })?;
        let name = name.trim();
        let ty = ty.trim();
        validate_ident(name, "field name")?;
        if ty.is_empty() {
            return Err(SunscreenError::UserInput(format!(
                "field `{name}` has empty type"
            )));
        }
        out.push(ParsedField {
            name: name.to_string(),
            ty: ty.to_string(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// scaffold account
// ---------------------------------------------------------------------------

fn run_account(args: &AccountArgs, json: bool, quiet: bool) -> Result<i32, SunscreenError> {
    run_account_in_workspace(args, json, quiet, None)
}

fn run_account_in_workspace(
    args: &AccountArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "account name")?;
    let fields = parse_fields(&args.fields)?;
    let account_snake = args.name.to_snake_case();

    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program_name = resolve_program(args.program.as_deref(), &ws)?;
    let program_snake = program_name.to_snake_case();
    let program: &ProgramView = workspace::find_program(&ws, &program_name).map_err(map_ws_err)?;

    let state_dir = program.src_dir.join("state");
    let account_abs = state_dir.join(format!("{account_snake}.rs"));
    let mod_abs = state_dir.join("mod.rs");
    let account_rel = relative_to(&ws.root, &account_abs);
    let mod_rel = relative_to(&ws.root, &mod_abs);

    // Render the account file body.
    let ctx = AccountCtx {
        program_name: program_snake.clone(),
        account_name: account_snake.clone(),
        fields: fields
            .iter()
            .map(|f| crate::templates::account::FieldSpec {
                name: f.name.clone(),
                ty: f.ty.clone(),
            })
            .collect(),
    };
    let account_body = render_account_file(&ctx)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render account: {e}")))?;

    // Determine FileStatus for the per-account file. The per-account file is
    // either created fresh, or — when it already exists — either identical
    // (Unchanged) or different (in which case we error out below). There is
    // no in-place rewrite path; a future `--force` flag may add one.
    let account_status = plan_account_file(&account_abs, &account_rel, &account_body, &args.name)?;

    // Compute new state/mod.rs body merging existing accounts.
    let mut all_accounts = list_existing_accounts(&state_dir);
    if !all_accounts.contains(&account_snake) {
        all_accounts.push(account_snake.clone());
    }
    let mod_segment_body = render_account_mod_segment(&all_accounts);
    let (new_mod_contents, mod_action) = build_segment_host(
        &mod_abs,
        "accounts",
        &mod_segment_body,
        MOD_RS_HEADER,
        "account",
    )?;
    let mod_status = status_after_action(&mod_abs, &new_mod_contents, mod_action);

    let plan_files: Vec<String> = vec![to_fwd_path(&account_rel), to_fwd_path(&mod_rel)];
    let segments_patched: Vec<&str> = if mod_status == FileStatus::Unchanged {
        Vec::new()
    } else {
        vec!["accounts"]
    };

    // Ensure `pub mod state;` in lib.rs (idempotent — no-op if already present).
    let lib_new = ensure_lib_mod_decl(&program.lib_rs, "state")?;
    let lib_rel = relative_to(&ws.root, &program.lib_rs);
    let mut plan_files = plan_files;
    if lib_new.is_some() {
        plan_files.push(to_fwd_path(&lib_rel));
    }

    // `unchanged` covers the user-facing files and the lib.rs mod-decl patch.
    let unchanged = account_status == FileStatus::Unchanged
        && mod_status == FileStatus::Unchanged
        && lib_new.is_none();

    if args.dry_run {
        if !quiet {
            emit_noun_dry_run(json, "account", &args.name, &program_name, &plan_files);
        }
        return Ok(0);
    }

    commit_account_changes(AccountCommitPlan {
        root: &ws.root,
        account_rel: &account_rel,
        account_body: &account_body,
        account_status,
        mod_abs: &mod_abs,
        mod_rel: &mod_rel,
        mod_contents: &new_mod_contents,
        mod_action,
        mod_status,
        lib_abs: &program.lib_rs,
        lib_new: lib_new.as_ref(),
    })?;

    if !quiet {
        emit_noun_result(
            json,
            "account",
            &args.name,
            &program_name,
            &plan_files,
            &segments_patched,
            unchanged,
            &[
                ("account_file", account_status.as_str()),
                ("mod_file", mod_status.as_str()),
                (
                    "lib_rs",
                    if lib_new.is_some() {
                        "patched"
                    } else {
                        "unchanged"
                    },
                ),
            ],
        );
    }
    Ok(0)
}

fn plan_account_file(
    account_abs: &Path,
    account_rel: &Path,
    account_body: &str,
    account_name: &str,
) -> Result<FileStatus, SunscreenError> {
    if !account_abs.exists() {
        return Ok(FileStatus::Created);
    }

    let existing = std::fs::read_to_string(account_abs).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read {}: {e}", account_abs.display()))
    })?;
    let rendered_body = extract_segment_body(account_body, "file").ok_or_else(|| {
        SunscreenError::Other(anyhow::anyhow!(
            "rendered account missing `segment=file` markers"
        ))
    })?;
    match extract_segment_body(&existing, "file") {
        None => plan_unmarked_account_file(&existing, account_body, account_rel),
        Some(existing_body) if existing_body == rendered_body => Ok(FileStatus::Unchanged),
        Some(_) => Err(SunscreenError::UserInput(format!(
            "account `{}` already exists at {} with different fields; \
             edit the file manually or remove it to regenerate \
             (a `--force` flag may be added in a future release)",
            account_name,
            account_rel.display(),
        ))),
    }
}

fn plan_unmarked_account_file(
    existing: &str,
    account_body: &str,
    account_rel: &Path,
) -> Result<FileStatus, SunscreenError> {
    if existing == account_body {
        return Ok(FileStatus::Unchanged);
    }
    Err(SunscreenError::InstructionDrift {
        path: account_rel.to_string_lossy().into_owned(),
        hint:
            "account file has no auto-generated markers; restore from VCS or delete to regenerate"
                .to_string(),
    })
}

struct AccountCommitPlan<'a> {
    root: &'a Path,
    account_rel: &'a Path,
    account_body: &'a str,
    account_status: FileStatus,
    mod_abs: &'a Path,
    mod_rel: &'a Path,
    mod_contents: &'a str,
    mod_action: ModAction,
    mod_status: FileStatus,
    lib_abs: &'a Path,
    lib_new: Option<&'a String>,
}

fn commit_account_changes(plan: AccountCommitPlan<'_>) -> Result<Vec<PathBuf>, SunscreenError> {
    let mut tx = Transaction::new(plan.root).map_err(map_tx_err)?;
    if plan.account_status == FileStatus::Created {
        tx.stage(&to_fwd_path(plan.account_rel), plan.account_body.as_bytes())
            .map_err(map_tx_err)?;
    }
    stage_host_change(
        &mut tx,
        plan.mod_rel,
        plan.mod_abs,
        plan.mod_contents,
        plan.mod_action,
        plan.mod_status,
    )?;
    stage_optional_lib_change(&mut tx, plan.lib_abs, plan.lib_new)?;
    tx.commit().map_err(map_tx_err)
}

const MOD_RS_HEADER: &str = "//! Auto-generated by sunscreen. Edit user regions only.\n\n";

fn list_existing_accounts(state_dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(state_dir) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == "mod" {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        out.push(stem.to_string());
    }
    out.sort();
    out
}

/// Build (or update) a host file that owns a single auto-generated segment.
///
/// Used for `state/mod.rs` (`accounts` segment). If the file does not exist,
/// emit a fresh skeleton with the markers + body. If the file exists but lacks
/// the segment, splice the marker block at the end. Otherwise apply via
/// `rustpatch`.
fn build_segment_host(
    path: &std::path::Path,
    segment: &str,
    segment_body: &str,
    header: &str,
    generator: &str,
) -> Result<(String, ModAction), SunscreenError> {
    if !path.exists() {
        let mut out = String::new();
        out.push_str(header);
        out.push_str(&format!(
            "// === sunscreen:auto-generated:begin segment={segment} version=1 generator={generator} ===\n"
        ));
        out.push_str(segment_body);
        out.push_str(&format!(
            "// === sunscreen:auto-generated:end segment={segment} ===\n"
        ));
        return Ok((out, ModAction::Create));
    }
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    let markers = scan(&existing).map_err(map_patch_err)?;
    let has_segment = markers
        .iter()
        .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == segment);
    if !has_segment {
        let mut out = existing.clone();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!(
            "\n// === sunscreen:auto-generated:begin segment={segment} version=1 generator={generator} ===\n"
        ));
        out.push_str(segment_body);
        out.push_str(&format!(
            "// === sunscreen:auto-generated:end segment={segment} ===\n"
        ));
        return Ok((out, ModAction::Replace));
    }
    let body_lines: Vec<String> = segment_body
        .strip_suffix('\n')
        .unwrap_or(segment_body)
        .split('\n')
        .map(str::to_string)
        .collect();
    let patches = vec![Patch {
        segment: segment.to_string(),
        lines: body_lines,
    }];
    let patched = apply(&existing, &patches).map_err(map_patch_err)?;
    Ok((patched, ModAction::Replace))
}

// ---------------------------------------------------------------------------
// scaffold event
// ---------------------------------------------------------------------------

fn run_event(args: &EventArgs, json: bool, quiet: bool) -> Result<i32, SunscreenError> {
    run_event_in_workspace(args, json, quiet, None)
}

fn run_event_in_workspace(
    args: &EventArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "event name")?;
    let fields = parse_fields(&args.fields)?;
    let event_pascal = args.name.to_pascal_case();

    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program_name = resolve_program(args.program.as_deref(), &ws)?;
    let program_snake = program_name.to_snake_case();
    let program: &ProgramView = workspace::find_program(&ws, &program_name).map_err(map_ws_err)?;

    let events_abs = program.src_dir.join("events.rs");
    let events_rel = relative_to(&ws.root, &events_abs);

    let new_ctx = EventCtx {
        program_name: program_snake.clone(),
        event_name: args.name.clone(),
        fields: fields
            .iter()
            .map(|f| crate::templates::event::FieldSpec {
                name: f.name.clone(),
                ty: f.ty.clone(),
            })
            .collect(),
    };

    // Merge with existing entries.
    let existing_events = read_event_entries(&events_abs)?;

    // Decide what to render.
    let new_entry = render_event_entry(&new_ctx)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render event: {e}")))?;

    // Idempotency: if event name exists, compare raw text — if identical, no-op;
    // if different fields, error.
    let merged = merge_named_entries(
        &existing_events,
        &event_pascal,
        &new_entry,
        format!("event `{event_pascal}` already exists with different fields"),
    )?;
    let segment_body = event_segment_body(&merged);

    let (new_contents, action) = build_events_host(&events_abs, &segment_body)?;
    let file_status = status_for_rendered_path(&events_abs, &new_contents);

    let plan_files = vec![to_fwd_path(&events_rel)];
    let segments_patched: Vec<&str> = if file_status == FileStatus::Unchanged {
        Vec::new()
    } else {
        vec!["events"]
    };
    // Ensure `pub mod events;` in lib.rs (idempotent — no-op if already present).
    // This guarantees downstream `use crate::events::*` (e.g. from
    // `scaffold instruction --emit X`) resolves without manual edits, even if
    // events.rs already existed without being declared in lib.rs.
    let lib_new = ensure_lib_mod_decl(&program.lib_rs, "events")?;
    let lib_rel = relative_to(&ws.root, &program.lib_rs);
    let mut plan_files = plan_files;
    if lib_new.is_some() {
        plan_files.push(to_fwd_path(&lib_rel));
    }

    let unchanged = file_status == FileStatus::Unchanged && lib_new.is_none();

    if args.dry_run {
        if !quiet {
            emit_noun_dry_run(json, "event", &args.name, &program_name, &plan_files);
        }
        return Ok(0);
    }

    commit_noun_file_changes(NounFileCommitPlan {
        root: &ws.root,
        file_abs: &events_abs,
        file_rel: &events_rel,
        file_contents: &new_contents,
        file_action: action,
        file_status,
        lib_abs: &program.lib_rs,
        lib_new: lib_new.as_ref(),
    })?;

    if !quiet {
        emit_noun_result(
            json,
            "event",
            &args.name,
            &program_name,
            &plan_files,
            &segments_patched,
            unchanged,
            &[
                ("events_file", file_status.as_str()),
                (
                    "lib_rs",
                    if lib_new.is_some() {
                        "patched"
                    } else {
                        "unchanged"
                    },
                ),
            ],
        );
    }
    Ok(0)
}

fn read_event_entries(events_abs: &Path) -> Result<Vec<(String, String)>, SunscreenError> {
    if !events_abs.exists() {
        return Ok(Vec::new());
    }
    let existing = std::fs::read_to_string(events_abs).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read {}: {e}", events_abs.display()))
    })?;
    Ok(extract_segment_body(&existing, "events")
        .map(|body| parse_event_entries(&body))
        .unwrap_or_default())
}

fn merge_named_entries(
    existing: &[(String, String)],
    target_name: &str,
    new_entry: &str,
    conflict_message: String,
) -> Result<Vec<String>, SunscreenError> {
    let mut merged: Vec<String> = existing.iter().map(|(_, raw)| raw.clone()).collect();
    let Some(idx) = existing.iter().position(|(name, _)| name == target_name) else {
        merged.push(new_entry.to_string());
        return Ok(merged);
    };
    if normalize_entry(&existing[idx].1) != normalize_entry(new_entry) {
        return Err(SunscreenError::UserInput(conflict_message));
    }
    Ok(merged)
}

fn event_segment_body(entries: &[String]) -> String {
    let mut segment_body = String::new();
    for (idx, entry) in entries.iter().enumerate() {
        if idx > 0 {
            segment_body.push('\n');
        }
        segment_body.push_str(entry);
        if !entry.ends_with('\n') {
            segment_body.push('\n');
        }
    }
    segment_body
}

struct NounFileCommitPlan<'a> {
    root: &'a Path,
    file_abs: &'a Path,
    file_rel: &'a Path,
    file_contents: &'a str,
    file_action: ModAction,
    file_status: FileStatus,
    lib_abs: &'a Path,
    lib_new: Option<&'a String>,
}

fn commit_noun_file_changes(plan: NounFileCommitPlan<'_>) -> Result<Vec<PathBuf>, SunscreenError> {
    if plan.file_status == FileStatus::Unchanged && plan.lib_new.is_none() {
        return Ok(Vec::new());
    }
    let mut tx = Transaction::new(plan.root).map_err(map_tx_err)?;
    stage_host_change(
        &mut tx,
        plan.file_rel,
        plan.file_abs,
        plan.file_contents,
        plan.file_action,
        plan.file_status,
    )?;
    stage_optional_lib_change(&mut tx, plan.lib_abs, plan.lib_new)?;
    tx.commit().map_err(map_tx_err)
}

/// Build (or rewrite) `events.rs` content with the given segment body. If the
/// file doesn't exist, emit a canonical skeleton using template constants. If
/// it does, splice via `rustpatch` (creating the segment block if missing).
fn build_events_host(
    path: &std::path::Path,
    segment_body: &str,
) -> Result<(String, ModAction), SunscreenError> {
    if !path.exists() {
        let out = format!(
            "{header}{begin}\n{body}{end}\n",
            header = EVENTS_FILE_HEADER,
            begin = EVENTS_SEGMENT_BEGIN,
            body = segment_body,
            end = EVENTS_SEGMENT_END,
        );
        return Ok((out, ModAction::Create));
    }
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    let markers = scan(&existing).map_err(map_patch_err)?;
    let has_segment = markers
        .iter()
        .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == "events");
    if !has_segment {
        let mut out = existing.clone();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(EVENTS_SEGMENT_BEGIN);
        out.push('\n');
        out.push_str(segment_body);
        out.push_str(EVENTS_SEGMENT_END);
        out.push('\n');
        return Ok((out, ModAction::Replace));
    }
    let body_lines: Vec<String> = segment_body
        .strip_suffix('\n')
        .unwrap_or(segment_body)
        .split('\n')
        .map(str::to_string)
        .collect();
    let patches = vec![Patch {
        segment: "events".to_string(),
        lines: body_lines,
    }];
    let patched = apply(&existing, &patches).map_err(map_patch_err)?;
    Ok((patched, ModAction::Replace))
}

/// Parse `pub struct <Name> { ... }` blocks (each preceded by `#[event]`) from
/// the body of the `events` segment. Returns `(PascalName, raw_block_with_newline)`
/// pairs, in source order. Each raw block ends with the line containing `}` and
/// a trailing newline.
fn parse_event_entries(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("#[event]") {
            // Look for the next `pub struct <Name> {`.
            let mut j = i;
            let mut name: Option<String> = None;
            while j < lines.len() {
                let l = lines[j].trim_start();
                if let Some(rest) = l.strip_prefix("pub struct ") {
                    let end = rest
                        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .unwrap_or(rest.len());
                    name = Some(rest[..end].to_string());
                    break;
                }
                j += 1;
            }
            // Find closing `}` at column 0 of the trimmed line (heuristic).
            let mut k = j;
            while k < lines.len() {
                if lines[k].trim() == "}" {
                    break;
                }
                k += 1;
            }
            if let Some(n) = name {
                let mut raw = String::new();
                for l in &lines[i..=k.min(lines.len().saturating_sub(1))] {
                    raw.push_str(l);
                    raw.push('\n');
                }
                out.push((n, raw));
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn event_fields_for_emit(program: &ProgramView, emit: &str) -> Result<Vec<String>, SunscreenError> {
    let events_abs = program.src_dir.join("events.rs");
    if !events_abs.exists() {
        return Ok(Vec::new());
    }

    let existing = std::fs::read_to_string(&events_abs).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read {}: {e}", events_abs.display()))
    })?;
    let Some(body) = extract_segment_body(&existing, "events") else {
        return Ok(Vec::new());
    };
    let emit_pascal = emit.to_pascal_case();
    for (name, raw) in parse_event_entries(&body) {
        if name == emit_pascal {
            return Ok(parse_event_field_names(&raw));
        }
    }
    Ok(Vec::new())
}

fn parse_event_field_names(raw_event: &str) -> Vec<String> {
    let mut fields = Vec::new();
    for line in raw_event.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub ") else {
            continue;
        };
        let Some((name, _ty)) = rest.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if !name.is_empty() {
            fields.push(name.to_string());
        }
    }
    fields
}

fn normalize_entry(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// scaffold error
// ---------------------------------------------------------------------------

fn run_error(args: &ErrorArgs, json: bool, quiet: bool) -> Result<i32, SunscreenError> {
    run_error_in_workspace(args, json, quiet, None)
}

fn run_error_in_workspace(
    args: &ErrorArgs,
    json: bool,
    quiet: bool,
    workspace_root: Option<&Path>,
) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "error variant name")?;
    let variant_pascal = args.name.to_pascal_case();

    let ws = workspace::find_root(workspace_root).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let program_name = resolve_program(args.program.as_deref(), &ws)?;
    let program_snake = program_name.to_snake_case();
    let enum_name = format!("{}_error", program_snake).to_pascal_case();
    let program: &ProgramView = workspace::find_program(&ws, &program_name).map_err(map_ws_err)?;

    let errors_abs = program.src_dir.join("errors.rs");
    let errors_rel = relative_to(&ws.root, &errors_abs);

    let new_variant = ErrorVariant {
        name: args.name.clone(),
        message: if args.msg.is_empty() {
            variant_pascal.clone()
        } else {
            args.msg.clone()
        },
    };

    // Collect existing variants from segment body (if file exists).
    let existing_variants = read_error_variants(&errors_abs)?;

    let new_entry = render_error_variant(&new_variant)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render error variant: {e}")))?;

    let merged = merge_named_entries(
        &existing_variants,
        &variant_pascal,
        &new_entry,
        format!("error variant `{variant_pascal}` already exists with a different message"),
    )?;
    let segment_body = error_variants_segment_body(&merged);

    let (new_contents, action) = build_errors_host(&errors_abs, &enum_name, &segment_body)?;
    let file_status = status_for_rendered_path(&errors_abs, &new_contents);

    let plan_files = vec![to_fwd_path(&errors_rel)];
    let segments_patched: Vec<&str> = if file_status == FileStatus::Unchanged {
        Vec::new()
    } else {
        vec!["error_variants"]
    };
    // Ensure `pub mod errors;` in lib.rs (idempotent — no-op if already present).
    let lib_new = ensure_lib_mod_decl(&program.lib_rs, "errors")?;
    let lib_rel = relative_to(&ws.root, &program.lib_rs);
    let mut plan_files = plan_files;
    if lib_new.is_some() {
        plan_files.push(to_fwd_path(&lib_rel));
    }

    let unchanged = file_status == FileStatus::Unchanged && lib_new.is_none();

    if args.dry_run {
        if !quiet {
            emit_noun_dry_run(json, "error", &args.name, &program_name, &plan_files);
        }
        return Ok(0);
    }

    commit_noun_file_changes(NounFileCommitPlan {
        root: &ws.root,
        file_abs: &errors_abs,
        file_rel: &errors_rel,
        file_contents: &new_contents,
        file_action: action,
        file_status,
        lib_abs: &program.lib_rs,
        lib_new: lib_new.as_ref(),
    })?;

    if !quiet {
        emit_noun_result(
            json,
            "error",
            &args.name,
            &program_name,
            &plan_files,
            &segments_patched,
            unchanged,
            &[
                ("errors_file", file_status.as_str()),
                (
                    "lib_rs",
                    if lib_new.is_some() {
                        "patched"
                    } else {
                        "unchanged"
                    },
                ),
            ],
        );
    }
    Ok(0)
}

fn read_error_variants(errors_abs: &Path) -> Result<Vec<(String, String)>, SunscreenError> {
    if !errors_abs.exists() {
        return Ok(Vec::new());
    }
    let existing = std::fs::read_to_string(errors_abs).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read {}: {e}", errors_abs.display()))
    })?;
    Ok(extract_segment_body(&existing, "error_variants")
        .map(|body| parse_error_variants(&body))
        .unwrap_or_default())
}

fn error_variants_segment_body(entries: &[String]) -> String {
    let mut segment_body = String::new();
    for entry in entries {
        for line in entry.lines() {
            segment_body.push_str("    ");
            segment_body.push_str(line);
            segment_body.push('\n');
        }
    }
    segment_body
}

fn build_errors_host(
    path: &std::path::Path,
    enum_name: &str,
    segment_body: &str,
) -> Result<(String, ModAction), SunscreenError> {
    if !path.exists() {
        let out = format!(
            "use anchor_lang::prelude::*;\n\n#[error_code]\npub enum {enum_name} {{\n    {begin}\n{body}    {end}\n}}\n",
            enum_name = enum_name,
            begin = ERROR_VARIANTS_SEGMENT_BEGIN,
            body = segment_body,
            end = ERROR_VARIANTS_SEGMENT_END,
        );
        return Ok((out, ModAction::Create));
    }
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    let markers = scan(&existing).map_err(map_patch_err)?;
    let has_segment = markers
        .iter()
        .any(|m| m.kind == MarkerKind::AutoGenerated && m.segment == "error_variants");
    if !has_segment {
        return Err(SunscreenError::InstructionDrift {
            path: path.display().to_string(),
            hint: "errors.rs has no `error_variants` segment markers; \
                   restore from VCS or delete to regenerate"
                .to_string(),
        });
    }
    let body_lines: Vec<String> = segment_body
        .strip_suffix('\n')
        .unwrap_or(segment_body)
        .split('\n')
        .map(str::to_string)
        .collect();
    let patches = vec![Patch {
        segment: "error_variants".to_string(),
        lines: body_lines,
    }];
    let patched = apply(&existing, &patches).map_err(map_patch_err)?;
    Ok((patched, ModAction::Replace))
}

/// Parse `#[msg("…")]\n<Name>,` blocks from the body of the `error_variants`
/// segment. Returns `(PascalName, raw_block)` pairs in source order.
fn parse_error_variants(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_start();
        if line.starts_with("#[msg(") {
            // Next non-empty line should be the variant name + `,`.
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let raw_name = lines[j].trim().trim_end_matches(',').trim();
                if !raw_name.is_empty() {
                    let name = raw_name.to_string();
                    let mut raw = String::new();
                    for l in &lines[i..=j] {
                        let trimmed = l.trim_start();
                        raw.push_str(trimmed);
                        raw.push('\n');
                    }
                    out.push((name, raw));
                    i = j + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Output helpers shared by account / event / error
// ---------------------------------------------------------------------------

fn emit_noun_dry_run(json: bool, noun: &str, name: &str, program: &str, files: &[String]) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "noun": noun,
            "name": name,
            "program": program,
            "files": files,
        });
        println!("{payload}");
    } else {
        println!("dry-run: would scaffold {noun} `{name}` in `{program}`");
        for f in files {
            println!("  {f}");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_noun_result(
    json: bool,
    noun: &str,
    name: &str,
    program: &str,
    files: &[String],
    segments_patched: &[&str],
    unchanged: bool,
    file_statuses: &[(&str, &str)],
) {
    if json {
        let mut payload = serde_json::json!({
            "ok": true,
            "noun": noun,
            "name": name,
            "program": program,
            "files": files,
            "segments_patched": segments_patched,
            "unchanged": unchanged,
        });
        if let Some(obj) = payload.as_object_mut() {
            for (k, v) in file_statuses {
                obj.insert(
                    (*k).to_string(),
                    serde_json::Value::String((*v).to_string()),
                );
            }
        }
        println!("{payload}");
    } else if unchanged {
        println!("scaffold {noun} `{name}`: unchanged (idempotent no-op)");
    } else {
        println!(
            "scaffolded {noun} `{name}` for program `{program}` ({} files)",
            files.len()
        );
        for f in files {
            println!("  {f}");
        }
    }
}

// ---------------------------------------------------------------------------
// scaffold program
// ---------------------------------------------------------------------------

const DEFAULT_PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";

fn run_program(args: &ProgramArgs, json: bool, quiet: bool) -> Result<i32, SunscreenError> {
    use heck::ToKebabCase;

    validate_program_name(&args.name)?;
    let program_kebab = args.name.to_kebab_case();
    let program_snake = args.name.to_snake_case();
    let program_id = args.id.as_deref().unwrap_or(DEFAULT_PROGRAM_ID);
    validate_program_id(program_id)?;

    let ws = workspace::find_root(None).map_err(map_ws_err)?;
    ensure_anchor_scaffolding(&ws)?;
    let workspace_root = ws.root.clone();
    let project_name = ws.config.project.name.clone();
    let anchor_version = ws
        .config
        .project
        .anchor_version
        .clone()
        .unwrap_or_else(|| "0.30.1".to_string());
    let rust_edition = ws.config.project.rust_edition.clone();

    ensure_program_not_declared(&ws, &program_snake, &program_kebab)?;
    let program_dir_rel = format!("programs/{program_snake}");
    let program_dir_abs = workspace_root.join(&program_dir_rel);
    ensure_program_dir_absent(&program_dir_abs, &program_dir_rel)?;

    // Render program crate into a temp staging dir so we can adopt every
    // file into the transaction atomically.
    let staging_tmp = tempfile::tempdir().map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?;
    let ctx = serde_json::json!({
        "program_name": program_snake,
        "project_name": project_name,
        "anchor_version": anchor_version,
        "rust_edition": rust_edition,
        "program_id": program_id,
    });
    let rendered = render_program(&ctx, staging_tmp.path())
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render program: {e}")))?;

    // Planned file list (workspace-relative, forward-slash).
    let planned = planned_program_files(&rendered, staging_tmp.path());

    // Patches to existing manifests.
    let anchor_toml_abs = workspace_root.join("Anchor.toml");
    let sunscreen_yml_abs = workspace_root.join("sunscreen.yml");
    let manifest_patches = plan_program_manifest_patches(
        &planned,
        &anchor_toml_abs,
        &sunscreen_yml_abs,
        &program_kebab,
        &program_snake,
        program_id,
    )?;

    if args.dry_run {
        emit_program_dry_run(
            quiet,
            json,
            &program_kebab,
            program_id,
            &manifest_patches.all_files,
        );
        return Ok(0);
    }

    // Commit: stage all new files + Anchor.toml / sunscreen.yml replacements.
    let written = commit_program_changes(ProgramCommitPlan {
        workspace_root: &workspace_root,
        staging_root: staging_tmp.path(),
        rendered: &rendered,
        anchor_toml_abs: &anchor_toml_abs,
        sunscreen_yml_abs: &sunscreen_yml_abs,
        manifest_patches: &manifest_patches,
    })?;

    if quiet {
        return Ok(0);
    }

    emit_program_result(
        json,
        &program_kebab,
        program_id,
        &manifest_patches,
        written.len(),
    );
    Ok(0)
}

fn ensure_program_not_declared(
    ws: &workspace::WorkspaceRoot,
    program_snake: &str,
    program_kebab: &str,
) -> Result<(), SunscreenError> {
    if ws.config.programs.iter().any(|program| {
        program.name.to_snake_case() == program_snake
            || program.name.to_kebab_case() == program_kebab
    }) {
        return Err(SunscreenError::UserInput(format!(
            "program `{program_kebab}` already declared in sunscreen.yml; \
             pick a different name or remove the existing entry"
        )));
    }
    Ok(())
}

fn ensure_program_dir_absent(
    program_dir_abs: &Path,
    program_dir_rel: &str,
) -> Result<(), SunscreenError> {
    if program_dir_abs.exists() {
        return Err(SunscreenError::UserInput(format!(
            "program directory already exists at {}; \
             remove it manually before re-scaffolding",
            program_dir_rel
        )));
    }
    Ok(())
}

fn planned_program_files(rendered: &[PathBuf], staging_root: &Path) -> Vec<String> {
    rendered
        .iter()
        .map(|abs| {
            let rel = abs.strip_prefix(staging_root).unwrap_or(abs);
            to_fwd_path(rel)
        })
        .collect()
}

struct ProgramManifestPatches {
    anchor_new: Option<String>,
    sunscreen_new: Option<String>,
    all_files: Vec<String>,
}

fn plan_program_manifest_patches(
    planned: &[String],
    anchor_toml_abs: &Path,
    sunscreen_yml_abs: &Path,
    program_kebab: &str,
    program_snake: &str,
    program_id: &str,
) -> Result<ProgramManifestPatches, SunscreenError> {
    let anchor_new = changed_anchor_toml(anchor_toml_abs, program_snake, program_id)?;
    let sunscreen_new = changed_sunscreen_yml(sunscreen_yml_abs, program_kebab, program_snake)?;
    let all_files = program_all_files(planned, anchor_new.is_some(), sunscreen_new.is_some());
    Ok(ProgramManifestPatches {
        anchor_new,
        sunscreen_new,
        all_files,
    })
}

fn changed_anchor_toml(
    anchor_toml_abs: &Path,
    program_snake: &str,
    program_id: &str,
) -> Result<Option<String>, SunscreenError> {
    if !anchor_toml_abs.exists() {
        return Ok(None);
    }
    let patched = patch_anchor_toml(anchor_toml_abs, program_snake, program_id)?;
    Ok(changed_contents(anchor_toml_abs, patched))
}

fn changed_sunscreen_yml(
    sunscreen_yml_abs: &Path,
    program_kebab: &str,
    program_snake: &str,
) -> Result<Option<String>, SunscreenError> {
    let patched = patch_sunscreen_yml(sunscreen_yml_abs, program_kebab, program_snake)?;
    Ok(changed_contents(sunscreen_yml_abs, patched))
}

fn changed_contents(path: &Path, patched: String) -> Option<String> {
    let current = std::fs::read_to_string(path).unwrap_or_default();
    (patched != current).then_some(patched)
}

fn program_all_files(
    planned: &[String],
    anchor_patched: bool,
    sunscreen_patched: bool,
) -> Vec<String> {
    let mut files = planned.to_vec();
    if anchor_patched {
        files.push("Anchor.toml".to_string());
    }
    if sunscreen_patched {
        files.push("sunscreen.yml".to_string());
    }
    files
}

fn emit_program_dry_run(
    quiet: bool,
    json: bool,
    program_kebab: &str,
    program_id: &str,
    all_files: &[String],
) {
    if quiet {
        return;
    }
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "noun": "program",
            "name": program_kebab,
            "files": all_files,
            "program_id": program_id,
        });
        println!("{payload}");
    } else {
        println!("dry-run: would scaffold program `{program_kebab}`");
        for file in all_files {
            println!("  {file}");
        }
    }
}

struct ProgramCommitPlan<'a> {
    workspace_root: &'a Path,
    staging_root: &'a Path,
    rendered: &'a [PathBuf],
    anchor_toml_abs: &'a Path,
    sunscreen_yml_abs: &'a Path,
    manifest_patches: &'a ProgramManifestPatches,
}

fn commit_program_changes(plan: ProgramCommitPlan<'_>) -> Result<Vec<PathBuf>, SunscreenError> {
    let mut tx = Transaction::new(plan.workspace_root).map_err(map_tx_err)?;
    for abs in plan.rendered {
        stage_rendered_program_file(&mut tx, abs, plan.staging_root)?;
    }
    if let Some(ref new_toml) = plan.manifest_patches.anchor_new {
        tx.stage_replace(plan.anchor_toml_abs, new_toml.as_bytes())
            .map_err(map_tx_err)?;
    }
    if let Some(ref new_yml) = plan.manifest_patches.sunscreen_new {
        tx.stage_replace(plan.sunscreen_yml_abs, new_yml.as_bytes())
            .map_err(map_tx_err)?;
    }
    tx.commit().map_err(map_tx_err)
}

fn stage_rendered_program_file(
    tx: &mut Transaction,
    abs: &Path,
    staging_root: &Path,
) -> Result<(), SunscreenError> {
    let rel = abs.strip_prefix(staging_root).unwrap_or(abs);
    let bytes = std::fs::read(abs).map_err(|e| {
        SunscreenError::Other(anyhow::anyhow!("read staged {}: {e}", abs.display()))
    })?;
    tx.stage(&to_fwd_path(rel), &bytes).map_err(map_tx_err)
}

fn emit_program_result(
    json: bool,
    program_kebab: &str,
    program_id: &str,
    patches: &ProgramManifestPatches,
    written_count: usize,
) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "noun": "program",
            "name": program_kebab,
            "files": patches.all_files,
            "written": written_count
                + usize::from(patches.anchor_new.is_some())
                + usize::from(patches.sunscreen_new.is_some()),
            "program_id": program_id,
            "anchor_toml_patched": patches.anchor_new.is_some(),
            "sunscreen_yml_patched": patches.sunscreen_new.is_some(),
        });
        println!("{payload}");
    } else {
        println!(
            "scaffolded program `{program_kebab}` ({} files)",
            patches.all_files.len()
        );
        for file in &patches.all_files {
            println!("  {file}");
        }
    }
}

fn validate_program_name(name: &str) -> Result<(), SunscreenError> {
    if name.is_empty() {
        return Err(SunscreenError::UserInput("program name is empty".into()));
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err(SunscreenError::UserInput(format!(
            "program name `{name}` must start with an ASCII letter"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(SunscreenError::UserInput(format!(
            "program name `{name}` may only contain letters, digits, '-', and '_'"
        )));
    }
    Ok(())
}

fn validate_program_id(id: &str) -> Result<(), SunscreenError> {
    // A 32-byte ed25519 pubkey encodes to 32-44 base58 chars (32 = all-zero
    // System Program, 44 = max for 256-bit values). Anything outside that
    // range is not a real pubkey. Anchor will still re-verify at build time,
    // so we deliberately don't pull bs58 just to decode 32 bytes here.
    const B58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if !(32..=44).contains(&id.len()) {
        return Err(SunscreenError::UserInput(format!(
            "invalid --id pubkey length: {} chars (expected 32-44 base58 chars)",
            id.len()
        )));
    }
    if !id.chars().all(|c| B58.contains(c)) {
        return Err(SunscreenError::UserInput(
            "invalid --id: must be base58 (no 0OIl)".into(),
        ));
    }
    Ok(())
}

/// Detect the dominant line ending in `source`. Matches the convention used
/// by `rustpatch::marker` so generated patches round-trip CRLF on Windows
/// checkouts instead of silently normalising to LF.
pub(crate) fn detect_line_ending(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Inject a `<program> = "<id>"` entry under `[programs.localnet]` and
/// `[programs.devnet]` in `Anchor.toml`. Idempotent.
fn patch_anchor_toml(
    path: &std::path::Path,
    program_snake: &str,
    program_id: &str,
) -> Result<String, SunscreenError> {
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    let nl = detect_line_ending(&existing);
    let mut out = String::with_capacity(existing.len() + 128);
    let mut in_localnet = false;
    let mut in_devnet = false;
    let mut localnet_done = false;
    let mut devnet_done = false;
    let entry = format!("{program_snake} = \"{program_id}\"");

    let lines: Vec<&str> = existing.lines().collect();
    let needle_localnet = "[programs.localnet]";
    let needle_devnet = "[programs.devnet]";

    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push_str(nl);
        let trimmed = line.trim();
        // When we hit a new section header, close out whichever we were in.
        if trimmed.starts_with('[') {
            in_localnet = trimmed == needle_localnet;
            in_devnet = trimmed == needle_devnet;
        }
        // Once we're in a programs.* section, append the entry right after the
        // header — but only if no equivalent entry already exists in that
        // section (idempotent).
        if in_localnet && !localnet_done && trimmed == needle_localnet {
            if !section_contains_key(&lines, i + 1, program_snake) {
                out.push_str(&entry);
                out.push_str(nl);
            }
            localnet_done = true;
        }
        if in_devnet && !devnet_done && trimmed == needle_devnet {
            if !section_contains_key(&lines, i + 1, program_snake) {
                out.push_str(&entry);
                out.push_str(nl);
            }
            devnet_done = true;
        }
    }

    // If sections were missing entirely, append them.
    if !localnet_done {
        out.push_str(&format!("{nl}{needle_localnet}{nl}{entry}{nl}"));
    }
    if !devnet_done {
        out.push_str(&format!("{nl}{needle_devnet}{nl}{entry}{nl}"));
    }
    Ok(out)
}

fn section_contains_key(lines: &[&str], start: usize, key: &str) -> bool {
    for line in lines.iter().skip(start) {
        let t = line.trim();
        if t.starts_with('[') {
            return false;
        }
        if let Some(lhs) = t.split('=').next() {
            if lhs.trim() == key {
                return true;
            }
        }
    }
    false
}

/// Append a new `programs:` entry to `sunscreen.yml`. The loader will then
/// re-resolve the program on the next workspace lookup.
fn patch_sunscreen_yml(
    path: &std::path::Path,
    program_kebab: &str,
    program_snake: &str,
) -> Result<String, SunscreenError> {
    let existing = std::fs::read_to_string(path)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("read {}: {e}", path.display())))?;
    let nl = detect_line_ending(&existing);
    let lines: Vec<&str> = existing.lines().collect();
    // Find the column-0 `programs:` key. `trim_start` would also match an
    // indented `programs:` inside a nested map (e.g. `cluster.programs:`)
    // and patch the entry in the wrong place. `trim_end` strips CRLF and
    // any trailing whitespace so the match survives mixed line endings.
    let programs_idx = lines
        .iter()
        .position(|l| !l.starts_with(char::is_whitespace) && l.trim_end() == "programs:");
    let entry = format!("  - name: {program_kebab}{nl}    path: programs/{program_snake}{nl}");
    if let Some(idx) = programs_idx {
        // Find the end of the programs list (next top-level key, i.e. line
        // not starting with whitespace and not empty).
        let mut end = lines.len();
        for (j, l) in lines.iter().enumerate().skip(idx + 1) {
            if l.is_empty() {
                continue;
            }
            let starts_with_ws = l.starts_with(' ') || l.starts_with('\t');
            if !starts_with_ws {
                end = j;
                break;
            }
        }
        let mut out = String::with_capacity(existing.len() + entry.len());
        for line in &lines[..end] {
            out.push_str(line);
            out.push_str(nl);
        }
        out.push_str(&entry);
        for line in &lines[end..] {
            out.push_str(line);
            out.push_str(nl);
        }
        Ok(out)
    } else {
        let mut out = existing.clone();
        if !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str("programs:");
        out.push_str(nl);
        out.push_str(&entry);
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_basic() {
        let v = parse_args("amount:u64, memo:String").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "amount");
        assert_eq!(v[0].ty, "u64");
        assert_eq!(v[1].ty, "String");
    }

    #[test]
    fn parse_args_empty() {
        assert!(parse_args("").unwrap().is_empty());
    }

    #[test]
    fn parse_args_rejects_missing_type() {
        assert!(parse_args("amount").is_err());
    }

    #[test]
    fn parse_accounts_basic() {
        let v = parse_accounts("vault:mut|signer,system_program:system").unwrap();
        assert_eq!(v.len(), 2);
        assert!(v[0].mutable);
        assert!(v[0].signer);
        assert_eq!(v[1].kind, AccountKind::SystemProgram);
    }

    #[test]
    fn parse_accounts_accepts_adr_style_colon_flags_and_program_shorthands() {
        let v = parse_accounts(
            "vault:mut:signer:seeds=b\"vault\";payer.key().as_ref(),payer:signer:mut,system_program,token_program,associated_token_program,ata_program:ata",
        )
        .unwrap();
        assert_eq!(v.len(), 6);
        assert_vault_account(&v[0]);
        assert_mut_signer_account(&v[1]);
        assert_program_shorthands(&v[2..]);
    }

    fn assert_vault_account(account: &AccountSpec) {
        assert!(account.mutable);
        assert!(account.signer);
        assert_eq!(account.seeds.as_ref().unwrap().len(), 2);
    }

    fn assert_mut_signer_account(account: &AccountSpec) {
        assert!(account.mutable);
        assert!(account.signer);
    }

    fn assert_program_shorthands(accounts: &[AccountSpec]) {
        assert_eq!(accounts[0].kind, AccountKind::SystemProgram);
        assert_eq!(accounts[1].kind, AccountKind::TokenProgram);
        assert_eq!(accounts[2].kind, AccountKind::AssociatedTokenProgram);
        assert_eq!(accounts[3].kind, AccountKind::AssociatedTokenProgram);
    }

    #[test]
    fn parse_accounts_seeds() {
        let v = parse_accounts("pda:seeds=b\"vault\";user.key().as_ref()").unwrap();
        assert_eq!(v.len(), 1);
        let seeds = v[0].seeds.as_ref().unwrap();
        assert_eq!(seeds.len(), 2);
    }

    #[test]
    fn parse_accounts_rejects_unknown_flag() {
        assert!(parse_accounts("foo:weird_flag").is_err());
    }

    #[test]
    fn validate_ident_basic() {
        assert!(validate_ident("foo_bar", "x").is_ok());
        assert!(validate_ident("_under", "x").is_ok());
        assert!(validate_ident("1bad", "x").is_err());
        assert!(validate_ident("has-dash", "x").is_err());
        assert!(validate_ident("", "x").is_err());
    }
}
