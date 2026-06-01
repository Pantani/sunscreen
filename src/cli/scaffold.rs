//! `sunscreen scaffold` subcommand group.
//!
//! Phase 2 R1 shipped `instruction`. R3 adds `account`, `event`, and `error`
//! using the same architecture (Args struct → `run_*` → Transaction commit).
//! `program` remains reserved for R4.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use heck::{ToPascalCase, ToSnakeCase};

use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::rustpatch::{apply, scan, MarkerKind, Patch, RustpatchError};
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
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
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
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
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
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
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
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
    /// Comma-separated handler args, e.g. `"amount:u64,memo:String"`.
    #[arg(long, value_name = "LIST", default_value = "")]
    pub args: String,
    /// Comma-separated accounts, e.g. `"vault:mut|signer,user:signer|seeds=b\"vault\";user.key()"`.
    ///
    /// Per-account flags are pipe-separated. Supported flags:
    /// `mut`, `signer`, `system`, `token`, `assoc_token`, `seeds=<expr>[;<expr>...]`
    /// (multiple seed expressions separated by `;` inside `seeds=`).
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
        ScaffoldCmd::Instruction(args) => run_instruction(args, json),
        ScaffoldCmd::Account(args) => run_account(args, json),
        ScaffoldCmd::Event(args) => run_event(args, json),
        ScaffoldCmd::Error(args) => run_error(args, json),
        ScaffoldCmd::Program(args) => run_program(args, json),
    }
}

fn run_instruction(args: &InstructionArgs, json: bool) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "instruction name")?;
    if let Some(emit) = &args.emit {
        validate_ident(emit, "--emit event name")?;
    }
    let ix_snake = args.name.to_snake_case();
    let program_snake = args.program.to_snake_case();

    let parsed_args = parse_args(&args.args)?;
    let parsed_accounts = parse_accounts(&args.accounts)?;

    // 1. Locate workspace.
    let ws = workspace::find_root(None).map_err(map_ws_err)?;
    let program: &ProgramView = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;

    // 2. Render instruction body.
    let ctx = InstructionCtx {
        program_name: program_snake.clone(),
        instruction_name: ix_snake.clone(),
        args: parsed_args.clone(),
        accounts: parsed_accounts,
        emit: args.emit.clone(),
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
    let mut ix_existing_patched: Option<String> = None;
    let ix_status: FileStatus = if ix_abs.exists() {
        let existing = std::fs::read_to_string(&ix_abs).map_err(|e| {
            SunscreenError::Other(anyhow::anyhow!("read {}: {e}", ix_abs.display()))
        })?;
        // The freshly-rendered template carries both the auto-generated and
        // the (stub) user regions. Extract the auto-generated `file` segment
        // body from each side and compare only that.
        let rendered_file_body =
            extract_segment_body(&instruction_body, "file").ok_or_else(|| {
                SunscreenError::Other(anyhow::anyhow!(
                    "rendered instruction missing `segment=file` markers"
                ))
            })?;
        let existing_file_body = extract_segment_body(&existing, "file");

        match existing_file_body {
            // Existing file has no recognisable `segment=file` marker — fall
            // back to a strict byte comparison and surface drift if mismatched.
            None => {
                if existing == instruction_body {
                    FileStatus::Unchanged
                } else {
                    return Err(SunscreenError::InstructionDrift {
                        path: ix_rel.to_string_lossy().into_owned(),
                        hint: "instruction file has no auto-generated markers; \
                               restore from VCS or delete to regenerate"
                            .to_string(),
                    });
                }
            }
            Some(existing_body) if existing_body == rendered_file_body => FileStatus::Unchanged,
            Some(_) => {
                // Auto-generated region drifted: re-apply ONLY the `file`
                // segment, leaving the user-region (`segment=handler`)
                // untouched.
                let body_lines: Vec<String> = rendered_file_body
                    .strip_suffix('\n')
                    .unwrap_or(&rendered_file_body)
                    .split('\n')
                    .map(str::to_string)
                    .collect();
                let patches = vec![Patch {
                    segment: "file".to_string(),
                    lines: body_lines,
                }];
                let patched = apply(&existing, &patches).map_err(map_patch_err)?;
                ix_existing_patched = Some(patched);
                FileStatus::Updated
            }
        }
    } else {
        FileStatus::Created
    };

    // 4. Compute new instructions/mod.rs content.
    let existing_instructions = list_existing_instructions(program);
    let mut all_instructions = existing_instructions.clone();
    if !all_instructions.contains(&ix_snake) {
        all_instructions.push(ix_snake.clone());
    }
    let mod_segment_body = render_instructions_mod_segment(&all_instructions);
    let (new_mod_contents, mod_action) =
        build_mod_rs(&program.instructions_mod_rs, &mod_segment_body)?;

    // Detect mod.rs no-op: if file exists and would be identical, skip.
    let mod_status: FileStatus = match mod_action {
        ModAction::Create => FileStatus::Created,
        ModAction::Replace => {
            let current = std::fs::read_to_string(&program.instructions_mod_rs).unwrap_or_default();
            if current == new_mod_contents {
                FileStatus::Unchanged
            } else {
                FileStatus::Updated
            }
        }
    };

    // 5. Compute new lib.rs content (best-effort; skip if marker missing).
    //
    // Recover existing dispatch entries (with their args) from the current
    // lib.rs `dispatch` segment so previously-scaffolded instructions keep
    // their argument lists. Then merge in the new instruction, deduplicating
    // by name (the new entry's args win for that name).
    let existing_dispatches = read_existing_dispatches(&program.lib_rs)?;
    let mut dispatches: Vec<InstructionDispatch> = Vec::new();
    for n in &all_instructions {
        let args = if n == &ix_snake {
            parsed_args.clone()
        } else if let Some(d) = existing_dispatches.iter().find(|d| &d.name == n) {
            d.args.clone()
        } else {
            Vec::new()
        };
        dispatches.push(InstructionDispatch {
            name: n.clone(),
            args,
        });
    }
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
    let to_fwd = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
    let plan_files: Vec<String> = {
        let mut v = vec![to_fwd(&ix_rel), to_fwd(&mod_rel)];
        if lib_status.patched {
            v.push(to_fwd(&lib_rel));
        }
        v
    };

    let unchanged = ix_status == FileStatus::Unchanged
        && mod_status == FileStatus::Unchanged
        && lib_file_status != FileStatus::Updated;

    if args.dry_run {
        emit_dry_run(json, &args.name, &args.program, &plan_files, &lib_status);
        return Ok(0);
    }

    // Atomic commit: stage all three file changes (new instruction file +
    // in-place mod.rs / lib.rs patches) into one transaction so they land
    // together or not at all. New files use `stage`; in-place rewrites use
    // `stage_replace` (which captures originals for rollback).
    let mut tx = Transaction::new(&ws.root).map_err(map_tx_err)?;

    // Stage the new instruction file (only if not unchanged).
    if ix_status == FileStatus::Created {
        tx.stage(&to_fwd(&ix_rel), instruction_body.as_bytes())
            .map_err(map_tx_err)?;
    } else if ix_status == FileStatus::Updated {
        // Auto-generated region drifted: rewrite in place, preserving the
        // user-region content captured in `ix_existing_patched`.
        if let Some(patched) = &ix_existing_patched {
            tx.stage_replace(&ix_abs, patched.as_bytes())
                .map_err(map_tx_err)?;
        }
    }

    // Stage the updated mod.rs only if it actually changed.
    match (mod_action, mod_status) {
        (_, FileStatus::Unchanged) => {}
        (ModAction::Create, _) => {
            tx.stage(&to_fwd(&mod_rel), new_mod_contents.as_bytes())
                .map_err(map_tx_err)?;
        }
        (ModAction::Replace, _) => {
            tx.stage_replace(&program.instructions_mod_rs, new_mod_contents.as_bytes())
                .map_err(map_tx_err)?;
        }
    }

    if lib_file_status == FileStatus::Updated {
        tx.stage_replace(&program.lib_rs, lib_new.as_bytes())
            .map_err(map_tx_err)?;
    }

    let written = tx.commit().map_err(map_tx_err)?;

    if json {
        let payload = serde_json::json!({
            "ok": true,
            "instruction": ix_snake,
            "program": args.program,
            "files": plan_files,
            "lib_rs_patched": lib_status.patched,
            "unchanged": unchanged,
            "instruction_file": ix_status.as_str(),
            "mod_file": mod_status.as_str(),
            "lib_file": lib_file_status.as_str(),
            // `written` covers new files; add replacements (mod.rs / lib.rs
            // updated in-place) so the count reflects all changed files.
            "written": written.len()
                + usize::from(mod_status == FileStatus::Updated)
                + usize::from(lib_file_status == FileStatus::Updated)
                + usize::from(ix_status == FileStatus::Updated),
        });
        println!("{payload}");
    } else if unchanged {
        println!("scaffold instruction `{ix_snake}`: unchanged (idempotent no-op)");
    } else {
        println!(
            "scaffolded instruction `{ix_snake}` for program `{}` ({} files)",
            args.program,
            plan_files.len()
        );
        for f in &plan_files {
            println!("  {f}");
        }
        if !lib_status.patched {
            println!(
                "warning: {} has no `dispatch` segment marker — skipped",
                lib_rel.display()
            );
        }
    }
    Ok(0)
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
        let parts = flag_str.split('|');

        let mut spec = AccountSpec {
            name: name.to_string(),
            mutable: false,
            signer: false,
            seeds: None,
            kind: AccountKind::Generic,
        };

        for flag in parts {
            let flag = flag.trim();
            if flag.is_empty() {
                continue;
            }
            match flag {
                "mut" => spec.mutable = true,
                "signer" => spec.signer = true,
                "system" => spec.kind = AccountKind::SystemProgram,
                "token" => spec.kind = AccountKind::TokenProgram,
                "assoc_token" => spec.kind = AccountKind::AssociatedTokenProgram,
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
        // Extract the parenthesised parameter list (match the closing paren of
        // the signature — wrappers keep the whole signature on one line).
        // Find the closing ')' of the param list by scanning forward from
        // `paren`, tracking angle-bracket depth so we skip over
        // `Context<Foo>` but stop at the real close-paren.
        let close = {
            let chars: Vec<char> = rest[paren + 1..].chars().collect();
            let mut depth = 0usize;
            let mut found = None;
            for (i, &ch) in chars.iter().enumerate() {
                match ch {
                    '<' => depth += 1,
                    '>' if depth > 0 => depth -= 1,
                    ')' if depth == 0 => {
                        found = Some(paren + 1 + i);
                        break;
                    }
                    _ => {}
                }
            }
            let Some(pos) = found else { continue };
            pos
        };
        let params = &rest[paren + 1..close];
        let mut args = Vec::new();
        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }
            // Skip the actual `ctx: Context<...>` parameter (not e.g. `ctx_seed: u64`).
            if param.starts_with("ctx:") || param.starts_with("ctx :") {
                continue;
            }
            let Some((arg_name, ty)) = param.split_once(':') else {
                continue;
            };
            let arg_name = arg_name.trim();
            let ty = ty.trim();
            if arg_name.is_empty() || ty.is_empty() {
                continue;
            }
            args.push(ArgSpec {
                name: arg_name.to_string(),
                ty: ty.to_string(),
            });
        }
        out.push(InstructionDispatch { name, args });
    }
    out
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

fn map_ws_err(e: WorkspaceError) -> SunscreenError {
    match e {
        WorkspaceError::NotFound => SunscreenError::WorkspaceMissing(e.to_string()),
        other => SunscreenError::from(other),
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

fn run_account(args: &AccountArgs, json: bool) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "account name")?;
    let fields = parse_fields(&args.fields)?;
    let account_snake = args.name.to_snake_case();
    let program_snake = args.program.to_snake_case();

    let ws = workspace::find_root(None).map_err(map_ws_err)?;
    let program: &ProgramView = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;

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
    let account_status: FileStatus = if account_abs.exists() {
        let existing = std::fs::read_to_string(&account_abs).map_err(|e| {
            SunscreenError::Other(anyhow::anyhow!("read {}: {e}", account_abs.display()))
        })?;
        let rendered_body = extract_segment_body(&account_body, "file").ok_or_else(|| {
            SunscreenError::Other(anyhow::anyhow!(
                "rendered account missing `segment=file` markers"
            ))
        })?;
        match extract_segment_body(&existing, "file") {
            None => {
                if existing == account_body {
                    FileStatus::Unchanged
                } else {
                    return Err(SunscreenError::InstructionDrift {
                        path: account_rel.to_string_lossy().into_owned(),
                        hint: "account file has no auto-generated markers; \
                               restore from VCS or delete to regenerate"
                            .to_string(),
                    });
                }
            }
            Some(existing_body) if existing_body == rendered_body => FileStatus::Unchanged,
            Some(_) => {
                return Err(SunscreenError::UserInput(format!(
                    "account `{}` already exists at {} with different fields; \
                     edit the file manually or remove it to regenerate \
                     (a `--force` flag may be added in a future release)",
                    args.name,
                    account_rel.display(),
                )));
            }
        }
    } else {
        FileStatus::Created
    };

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
    let mod_status: FileStatus = match mod_action {
        ModAction::Create => FileStatus::Created,
        ModAction::Replace => {
            let current = std::fs::read_to_string(&mod_abs).unwrap_or_default();
            if current == new_mod_contents {
                FileStatus::Unchanged
            } else {
                FileStatus::Updated
            }
        }
    };

    let to_fwd = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
    let plan_files: Vec<String> = vec![to_fwd(&account_rel), to_fwd(&mod_rel)];
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
        plan_files.push(to_fwd(&lib_rel));
    }

    // `unchanged` covers the user-facing files and the lib.rs mod-decl patch.
    let unchanged = account_status == FileStatus::Unchanged
        && mod_status == FileStatus::Unchanged
        && lib_new.is_none();

    if args.dry_run {
        emit_noun_dry_run(json, "account", &args.name, &args.program, &plan_files);
        return Ok(0);
    }

    let mut tx = Transaction::new(&ws.root).map_err(map_tx_err)?;
    // Only `Created` and `Unchanged` are reachable here (see comment on
    // `account_status` above). `Unchanged` is a no-op.
    if account_status == FileStatus::Created {
        tx.stage(&to_fwd(&account_rel), account_body.as_bytes())
            .map_err(map_tx_err)?;
    }
    match (mod_action, mod_status) {
        (_, FileStatus::Unchanged) => {}
        (ModAction::Create, _) => {
            tx.stage(&to_fwd(&mod_rel), new_mod_contents.as_bytes())
                .map_err(map_tx_err)?;
        }
        (ModAction::Replace, _) => {
            tx.stage_replace(&mod_abs, new_mod_contents.as_bytes())
                .map_err(map_tx_err)?;
        }
    }
    if let Some(ref contents) = lib_new {
        tx.stage_replace(&program.lib_rs, contents.as_bytes())
            .map_err(map_tx_err)?;
    }
    let _written = tx.commit().map_err(map_tx_err)?;

    emit_noun_result(
        json,
        "account",
        &args.name,
        &args.program,
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
    Ok(0)
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

fn run_event(args: &EventArgs, json: bool) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "event name")?;
    let fields = parse_fields(&args.fields)?;
    let event_pascal = args.name.to_pascal_case();
    let program_snake = args.program.to_snake_case();

    let ws = workspace::find_root(None).map_err(map_ws_err)?;
    let program: &ProgramView = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;

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
    let mut existing_events: Vec<(String, String)> = Vec::new(); // (pascal_name, raw_entry)
    if events_abs.exists() {
        let existing = std::fs::read_to_string(&events_abs).map_err(|e| {
            SunscreenError::Other(anyhow::anyhow!("read {}: {e}", events_abs.display()))
        })?;
        if let Some(body) = extract_segment_body(&existing, "events") {
            existing_events = parse_event_entries(&body);
        }
    }

    // Decide what to render.
    let new_entry = render_event_entry(&new_ctx)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render event: {e}")))?;

    // Idempotency: if event name exists, compare raw text — if identical, no-op;
    // if different fields, error.
    let mut merged: Vec<String> = Vec::new();
    let mut already_present = false;
    for (name, raw) in &existing_events {
        if name == &event_pascal {
            already_present = true;
            let normalized_new = normalize_entry(&new_entry);
            let normalized_old = normalize_entry(raw);
            if normalized_new != normalized_old {
                return Err(SunscreenError::UserInput(format!(
                    "event `{event_pascal}` already exists with different fields"
                )));
            }
            merged.push(raw.clone());
        } else {
            merged.push(raw.clone());
        }
    }
    if !already_present {
        merged.push(new_entry.clone());
    }

    // Build segment body: concat entries separated by blank line for readability.
    let mut segment_body = String::new();
    for (i, entry) in merged.iter().enumerate() {
        if i > 0 {
            segment_body.push('\n');
        }
        segment_body.push_str(entry);
        if !entry.ends_with('\n') {
            segment_body.push('\n');
        }
    }

    let (new_contents, action) = build_events_host(&events_abs, &segment_body)?;
    let file_status: FileStatus = if !events_abs.exists() {
        FileStatus::Created
    } else {
        let current = std::fs::read_to_string(&events_abs).unwrap_or_default();
        if current == new_contents {
            FileStatus::Unchanged
        } else {
            FileStatus::Updated
        }
    };

    let to_fwd = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
    let plan_files = vec![to_fwd(&events_rel)];
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
        plan_files.push(to_fwd(&lib_rel));
    }

    let unchanged = file_status == FileStatus::Unchanged && lib_new.is_none();

    if args.dry_run {
        emit_noun_dry_run(json, "event", &args.name, &args.program, &plan_files);
        return Ok(0);
    }

    if file_status != FileStatus::Unchanged || lib_new.is_some() {
        let mut tx = Transaction::new(&ws.root).map_err(map_tx_err)?;
        if file_status != FileStatus::Unchanged {
            match action {
                ModAction::Create => {
                    tx.stage(&to_fwd(&events_rel), new_contents.as_bytes())
                        .map_err(map_tx_err)?;
                }
                ModAction::Replace => {
                    tx.stage_replace(&events_abs, new_contents.as_bytes())
                        .map_err(map_tx_err)?;
                }
            }
        }
        if let Some(ref contents) = lib_new {
            tx.stage_replace(&program.lib_rs, contents.as_bytes())
                .map_err(map_tx_err)?;
        }
        let _ = tx.commit().map_err(map_tx_err)?;
    }

    emit_noun_result(
        json,
        "event",
        &args.name,
        &args.program,
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
    Ok(0)
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

fn run_error(args: &ErrorArgs, json: bool) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "error variant name")?;
    let variant_pascal = args.name.to_pascal_case();
    let program_snake = args.program.to_snake_case();
    let enum_name = format!("{}_error", program_snake).to_pascal_case();

    let ws = workspace::find_root(None).map_err(map_ws_err)?;
    let program: &ProgramView = workspace::find_program(&ws, &args.program).map_err(map_ws_err)?;

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
    let mut existing_variants: Vec<(String, String)> = Vec::new(); // (PascalName, raw block)
    if errors_abs.exists() {
        let existing = std::fs::read_to_string(&errors_abs).map_err(|e| {
            SunscreenError::Other(anyhow::anyhow!("read {}: {e}", errors_abs.display()))
        })?;
        if let Some(body) = extract_segment_body(&existing, "error_variants") {
            existing_variants = parse_error_variants(&body);
        }
    }

    let new_entry = render_error_variant(&new_variant)
        .map_err(|e| SunscreenError::Other(anyhow::anyhow!("render error variant: {e}")))?;

    let mut merged: Vec<String> = Vec::new();
    let mut already_present = false;
    for (name, raw) in &existing_variants {
        if name == &variant_pascal {
            already_present = true;
            if normalize_entry(raw) != normalize_entry(&new_entry) {
                return Err(SunscreenError::UserInput(format!(
                    "error variant `{variant_pascal}` already exists with a different message"
                )));
            }
            merged.push(raw.clone());
        } else {
            merged.push(raw.clone());
        }
    }
    if !already_present {
        merged.push(new_entry.clone());
    }

    // Build segment body: each entry indented by 4 spaces (nested inside enum).
    let mut segment_body = String::new();
    for entry in &merged {
        for line in entry.lines() {
            segment_body.push_str("    ");
            segment_body.push_str(line);
            segment_body.push('\n');
        }
    }

    let (new_contents, action) = build_errors_host(&errors_abs, &enum_name, &segment_body)?;
    let file_status: FileStatus = if !errors_abs.exists() {
        FileStatus::Created
    } else {
        let current = std::fs::read_to_string(&errors_abs).unwrap_or_default();
        if current == new_contents {
            FileStatus::Unchanged
        } else {
            FileStatus::Updated
        }
    };

    let to_fwd = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
    let plan_files = vec![to_fwd(&errors_rel)];
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
        plan_files.push(to_fwd(&lib_rel));
    }

    let unchanged = file_status == FileStatus::Unchanged && lib_new.is_none();

    if args.dry_run {
        emit_noun_dry_run(json, "error", &args.name, &args.program, &plan_files);
        return Ok(0);
    }

    if file_status != FileStatus::Unchanged || lib_new.is_some() {
        let mut tx = Transaction::new(&ws.root).map_err(map_tx_err)?;
        if file_status != FileStatus::Unchanged {
            match action {
                ModAction::Create => {
                    tx.stage(&to_fwd(&errors_rel), new_contents.as_bytes())
                        .map_err(map_tx_err)?;
                }
                ModAction::Replace => {
                    tx.stage_replace(&errors_abs, new_contents.as_bytes())
                        .map_err(map_tx_err)?;
                }
            }
        }
        if let Some(ref contents) = lib_new {
            tx.stage_replace(&program.lib_rs, contents.as_bytes())
                .map_err(map_tx_err)?;
        }
        let _ = tx.commit().map_err(map_tx_err)?;
    }

    emit_noun_result(
        json,
        "error",
        &args.name,
        &args.program,
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
    Ok(0)
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

fn run_program(args: &ProgramArgs, json: bool) -> Result<i32, SunscreenError> {
    use heck::ToKebabCase;

    validate_program_name(&args.name)?;
    let program_kebab = args.name.to_kebab_case();
    let program_snake = args.name.to_snake_case();
    let program_id = args.id.as_deref().unwrap_or(DEFAULT_PROGRAM_ID);
    validate_program_id(program_id)?;

    let ws = workspace::find_root(None).map_err(map_ws_err)?;
    let workspace_root = ws.root.clone();
    let project_name = ws.config.project.name.clone();
    let anchor_version = ws
        .config
        .project
        .anchor_version
        .clone()
        .unwrap_or_else(|| "0.30.1".to_string());
    let rust_edition = ws.config.project.rust_edition.clone();

    // Idempotency check #1: program already declared in sunscreen.yml.
    if ws
        .config
        .programs
        .iter()
        .any(|p| p.name.to_snake_case() == program_snake || p.name.to_kebab_case() == program_kebab)
    {
        return Err(SunscreenError::UserInput(format!(
            "program `{program_kebab}` already declared in sunscreen.yml; \
             pick a different name or remove the existing entry"
        )));
    }

    // Idempotency check #2: directory already exists on disk.
    let program_dir_rel = format!("programs/{program_snake}");
    let program_dir_abs = workspace_root.join(&program_dir_rel);
    if program_dir_abs.exists() {
        return Err(SunscreenError::UserInput(format!(
            "program directory already exists at {}; \
             remove it manually before re-scaffolding",
            program_dir_rel
        )));
    }

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
    let to_fwd = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
    let mut planned: Vec<String> = Vec::new();
    for abs in &rendered {
        let rel = abs
            .strip_prefix(staging_tmp.path())
            .unwrap_or(abs)
            .to_path_buf();
        // Files are already emitted under `programs/<program_snake>/...`
        // because `render_program` substitutes `__program__` at render time.
        // The relative path is workspace-relative as-is.
        let rel_str = to_fwd(&rel);
        planned.push(rel_str);
    }

    // Patches to existing manifests.
    let anchor_toml_abs = workspace_root.join("Anchor.toml");
    let sunscreen_yml_abs = workspace_root.join("sunscreen.yml");
    let anchor_toml_rel = "Anchor.toml".to_string();
    let sunscreen_yml_rel = "sunscreen.yml".to_string();

    // Only treat manifests as "patched" when their content actually changes
    // — idempotent re-runs (and `--dry-run` plans) shouldn't claim edits.
    let anchor_new = if anchor_toml_abs.exists() {
        let patched = patch_anchor_toml(&anchor_toml_abs, &program_snake, program_id)?;
        let current = std::fs::read_to_string(&anchor_toml_abs).unwrap_or_default();
        if patched == current {
            None
        } else {
            Some(patched)
        }
    } else {
        None
    };
    let sunscreen_patched =
        patch_sunscreen_yml(&sunscreen_yml_abs, &program_kebab, &program_snake)?;
    let sunscreen_current = std::fs::read_to_string(&sunscreen_yml_abs).unwrap_or_default();
    let sunscreen_new = if sunscreen_patched == sunscreen_current {
        None
    } else {
        Some(sunscreen_patched)
    };

    let mut all_files = planned.clone();
    if anchor_new.is_some() {
        all_files.push(anchor_toml_rel.clone());
    }
    if sunscreen_new.is_some() {
        all_files.push(sunscreen_yml_rel.clone());
    }

    if args.dry_run {
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
            for f in &all_files {
                println!("  {f}");
            }
        }
        return Ok(0);
    }

    // Commit: stage all new files + Anchor.toml / sunscreen.yml replacements.
    let mut tx = Transaction::new(&workspace_root).map_err(map_tx_err)?;
    for abs in &rendered {
        let rel = abs
            .strip_prefix(staging_tmp.path())
            .unwrap_or(abs)
            .to_path_buf();
        let bytes = std::fs::read(abs).map_err(|e| {
            SunscreenError::Other(anyhow::anyhow!("read staged {}: {e}", abs.display()))
        })?;
        tx.stage(&to_fwd(&rel), &bytes).map_err(map_tx_err)?;
    }
    if let Some(ref new_toml) = anchor_new {
        tx.stage_replace(&anchor_toml_abs, new_toml.as_bytes())
            .map_err(map_tx_err)?;
    }
    if let Some(ref new_yml) = sunscreen_new {
        tx.stage_replace(&sunscreen_yml_abs, new_yml.as_bytes())
            .map_err(map_tx_err)?;
    }
    let written = tx.commit().map_err(map_tx_err)?;

    if json {
        let payload = serde_json::json!({
            "ok": true,
            "noun": "program",
            "name": program_kebab,
            "files": all_files,
            "written": written.len()
                + usize::from(anchor_new.is_some())
                + usize::from(sunscreen_new.is_some()),
            "program_id": program_id,
            "anchor_toml_patched": anchor_new.is_some(),
            "sunscreen_yml_patched": sunscreen_new.is_some(),
        });
        println!("{payload}");
    } else {
        println!(
            "scaffolded program `{program_kebab}` ({} files)",
            all_files.len()
        );
        for f in &all_files {
            println!("  {f}");
        }
    }
    Ok(0)
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
