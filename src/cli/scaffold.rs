//! `sunscreen scaffold` subcommand group.
//!
//! Phase 2 R1 ships `instruction`. The remaining nouns (`account`, `event`,
//! `error`, `program`) are stubs reserved for R2+.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use heck::ToSnakeCase;

use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};
use crate::rustpatch::{apply, scan, MarkerKind, Patch, RustpatchError};
use crate::templates::{
    render_dispatch_segment, render_instruction, render_instructions_mod_segment, AccountKind,
    AccountSpec, ArgSpec, InstructionCtx, InstructionDispatch,
};
use crate::workspace::{self, ProgramView, WorkspaceError};

/// Subcommands grouped under `sunscreen scaffold`.
#[derive(Debug, Subcommand)]
pub enum ScaffoldCmd {
    /// Generate a new Anchor instruction handler.
    Instruction(InstructionArgs),
    /// Reserved (Phase 2 R2+).
    Account,
    /// Reserved (Phase 2 R2+).
    Event,
    /// Reserved (Phase 2 R2+).
    Error,
    /// Reserved (Phase 2 R2+).
    Program,
}

/// Flags for `sunscreen scaffold instruction`.
#[derive(Debug, Args)]
pub struct InstructionArgs {
    /// Instruction name (snake_case recommended; will be normalised).
    pub name: String,
    /// Parent program (must exist in `sunscreen.yml`).
    #[arg(long, value_name = "NAME")]
    pub program: String,
    /// Comma-separated handler args, e.g. `"amount:u64,memo:String"`.
    #[arg(long, value_name = "LIST", default_value = "")]
    pub args: String,
    /// Comma-separated accounts, e.g. `"vault:mut|signer,user:signer|seeds=b\"vault\"|user.key()"`.
    ///
    /// Per-account flags are pipe-separated. Supported flags:
    /// `mut`, `signer`, `system`, `token`, `assoc_token`, `seeds=<expr>[;<expr>...]`.
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
        ScaffoldCmd::Account | ScaffoldCmd::Event | ScaffoldCmd::Error | ScaffoldCmd::Program => {
            eprintln!("scaffold: noun reserved for Phase 2 R2+");
            Ok(0)
        }
    }
}

fn run_instruction(args: &InstructionArgs, json: bool) -> Result<i32, SunscreenError> {
    validate_ident(&args.name, "instruction name")?;
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
    let plan_files: Vec<String> = {
        let mut v = vec![
            ix_rel.to_string_lossy().into_owned(),
            mod_rel.to_string_lossy().into_owned(),
        ];
        if lib_status.patched {
            v.push(lib_rel.to_string_lossy().into_owned());
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

    // Two-phase commit: stage into a tx rooted at ws.root, then rename in.
    let mut tx = Transaction::new(&ws.root).map_err(map_tx_err)?;

    // Stage the new instruction file (only if not unchanged).
    if ix_status == FileStatus::Created {
        tx.stage(&ix_rel.to_string_lossy(), instruction_body.as_bytes())
            .map_err(map_tx_err)?;
    } else if ix_status == FileStatus::Updated {
        // Auto-generated region drifted: rewrite in place, preserving the
        // user-region content captured in `ix_existing_patched`.
        if let Some(patched) = &ix_existing_patched {
            replace_in_place(&ix_abs, patched)?;
        }
    }

    // Stage the updated mod.rs only if it actually changed.
    match (mod_action, mod_status) {
        (_, FileStatus::Unchanged) => {}
        (ModAction::Create, _) => {
            tx.stage(&mod_rel.to_string_lossy(), new_mod_contents.as_bytes())
                .map_err(map_tx_err)?;
        }
        (ModAction::Replace, _) => {
            replace_in_place(&program.instructions_mod_rs, &new_mod_contents)?;
        }
    }

    if lib_file_status == FileStatus::Updated {
        replace_in_place(&program.lib_rs, &lib_new)?;
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
            "written": written.len(),
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

#[derive(Debug, Clone, Copy)]
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
        let Some(close) = rest.rfind(')') else {
            continue;
        };
        if close <= paren {
            continue;
        }
        let params = &rest[paren + 1..close];
        let mut args = Vec::new();
        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }
            // Skip the leading `ctx: Context<...>` parameter.
            if param.starts_with("ctx") {
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

/// Atomic in-place file replacement (write sibling tempfile + rename).
fn replace_in_place(path: &std::path::Path, contents: &str) -> Result<(), SunscreenError> {
    let parent = path.parent().ok_or_else(|| {
        SunscreenError::Other(anyhow::anyhow!("no parent for {}", path.display()))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?;
    let tmp = parent.join(format!(
        ".sunscreen-edit-{}-{}.tmp",
        std::process::id(),
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("scratch")
    ));
    std::fs::write(&tmp, contents).map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?;
    std::fs::rename(&tmp, path).map_err(|e| SunscreenError::Other(anyhow::anyhow!(e)))?;
    Ok(())
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
