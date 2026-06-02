//! Offline compile checks for generated workspaces.
//!
//! These do not require Anchor/Solana binaries. The generated workspace is
//! patched to a tiny local `anchor-lang` shim so `cargo check` can validate the
//! Rust module graph and scaffolded source shape.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sunscreen_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sunscreen")
}

fn run_sunscreen(workspace: Option<&Path>, args: &[&str]) {
    let mut cmd = Command::new(sunscreen_bin());
    cmd.env("SUNSCREEN_SKIP_PREFLIGHT", "1").args(args);
    if let Some(workspace) = workspace {
        cmd.current_dir(workspace);
    }
    let out = cmd.output().expect("invoke sunscreen");
    assert!(
        out.status.success(),
        "sunscreen {args:?} failed: code={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn discover_program(ws: &Path) -> String {
    let programs_dir = ws.join("programs");
    let entries: Vec<_> = std::fs::read_dir(&programs_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(entries.len(), 1, "expected exactly one program dir");
    entries[0].file_name().to_string_lossy().into_owned()
}

fn new_workspace(tmp: &Path, name: &str, frontend: &str) -> (PathBuf, String) {
    let ws = tmp.join(name);
    run_sunscreen(
        None,
        &[
            "chain",
            "new",
            name,
            "--frontend",
            frontend,
            "--path",
            ws.to_str().unwrap(),
        ],
    );
    let program = discover_program(&ws);
    (ws, program)
}

fn new_pinocchio_workspace(tmp: &Path, name: &str) -> (PathBuf, String) {
    let ws = tmp.join(name);
    run_sunscreen(
        None,
        &[
            "chain",
            "new",
            name,
            "--framework",
            "pinocchio",
            "--frontend",
            "none",
            "--path",
            ws.to_str().unwrap(),
        ],
    );
    let program = discover_program(&ws);
    (ws, program)
}

fn write_anchor_shim(root: &Path) -> PathBuf {
    let shim_root = root.join("anchor-shim");
    let macros = shim_root.join("anchor-lang-macros");
    let anchor = shim_root.join("anchor-lang");
    std::fs::create_dir_all(macros.join("src")).unwrap();
    std::fs::create_dir_all(anchor.join("src")).unwrap();

    std::fs::write(
        macros.join("Cargo.toml"),
        r#"[package]
name = "anchor-lang-macros"
version = "0.30.1"
edition = "2021"

[lib]
proc-macro = true
"#,
    )
    .unwrap();
    std::fs::write(
        macros.join("src/lib.rs"),
        r#"extern crate proc_macro;

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_attribute]
pub fn program(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn account(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn error_code(_attr: TokenStream, item: TokenStream) -> TokenStream {
    strip_msg_attrs(item)
}

#[proc_macro_derive(Accounts, attributes(account, instruction))]
pub fn derive_accounts(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}

fn strip_msg_attrs(input: TokenStream) -> TokenStream {
    let mut out = TokenStream::new();
    let mut iter = input.into_iter().peekable();
    while let Some(token) = iter.next() {
        if is_hash(&token) {
            if let Some(TokenTree::Group(group)) = iter.peek() {
                if group.delimiter() == Delimiter::Bracket && starts_with_msg(group.stream()) {
                    iter.next();
                    continue;
                }
            }
        }
        match token {
            TokenTree::Group(group) => {
                let mut rebuilt = Group::new(group.delimiter(), strip_msg_attrs(group.stream()));
                rebuilt.set_span(group.span());
                out.extend([TokenTree::Group(rebuilt)]);
            }
            other => out.extend([other]),
        }
    }
    out
}

fn is_hash(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(p) if p.as_char() == '#')
}

fn starts_with_msg(stream: TokenStream) -> bool {
    matches!(stream.into_iter().next(), Some(TokenTree::Ident(ident)) if ident.to_string() == "msg")
}
"#,
    )
    .unwrap();

    std::fs::write(
        anchor.join("Cargo.toml"),
        r#"[package]
name = "anchor-lang"
version = "0.30.1"
edition = "2021"

[dependencies]
anchor-lang-macros = { path = "../anchor-lang-macros" }

[features]
default = []
idl-build = []
"#,
    )
    .unwrap();
    std::fs::write(
        anchor.join("src/lib.rs"),
        r#"pub use anchor_lang_macros::{account, error_code, event, program, Accounts};

pub mod prelude {
    pub use crate::{
        declare_id, emit, account, error_code, event, program, AccountInfo, Accounts, Context,
        Program, Pubkey, Result, Signer, System,
    };
}

#[macro_export]
macro_rules! declare_id {
    ($id:literal) => {
        pub const ID: &str = $id;
    };
}

#[macro_export]
macro_rules! emit {
    ($event:expr) => {{
        let _ = &$event;
    }};
}

pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error;

pub struct Context<T>(pub T);
pub struct Pubkey;
pub struct System;

pub struct Signer<'info> {
    _marker: std::marker::PhantomData<&'info ()>,
}

pub struct AccountInfo<'info> {
    _marker: std::marker::PhantomData<&'info ()>,
}

pub struct Program<'info, T> {
    _marker: std::marker::PhantomData<(&'info (), T)>,
}
"#,
    )
    .unwrap();

    anchor
}

fn write_pinocchio_shim(root: &Path) -> PathBuf {
    let pinocchio = root.join("pinocchio-shim");
    std::fs::create_dir_all(pinocchio.join("src")).unwrap();
    std::fs::write(
        pinocchio.join("Cargo.toml"),
        r#"[package]
name = "pinocchio"
version = "0.11.1"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    std::fs::write(
        pinocchio.join("src/lib.rs"),
        r#"pub struct AccountView;
pub struct Address;

#[derive(Debug, Clone, Copy)]
pub struct ProgramError;

pub type ProgramResult = Result<(), ProgramError>;
"#,
    )
    .unwrap();
    pinocchio
}

fn write_anchor_spl_shim(root: &Path) -> PathBuf {
    let anchor_spl = root.join("anchor-shim").join("anchor-spl");
    std::fs::create_dir_all(anchor_spl.join("src")).unwrap();

    std::fs::write(
        anchor_spl.join("Cargo.toml"),
        r#"[package]
name = "anchor-spl"
version = "0.30.1"
edition = "2021"
"#,
    )
    .unwrap();
    std::fs::write(
        anchor_spl.join("src/lib.rs"),
        r#"pub mod token {
    pub struct Token;
}

pub mod associated_token {
    pub struct AssociatedToken;
}
"#,
    )
    .unwrap();

    anchor_spl
}

fn patch_anchor_deps(ws: &Path, anchor_shim: &Path, anchor_spl_shim: &Path) {
    let cargo_toml = ws.join("Cargo.toml");
    let mut contents = std::fs::read_to_string(&cargo_toml).unwrap();
    contents.push_str(&format!(
        "\n[patch.crates-io]\nanchor-lang = {{ path = {:?} }}\nanchor-spl = {{ path = {:?} }}\n",
        anchor_shim, anchor_spl_shim
    ));
    std::fs::write(cargo_toml, contents).unwrap();
}

fn patch_pinocchio_dep(ws: &Path, pinocchio_shim: &Path) {
    let cargo_toml = ws.join("Cargo.toml");
    let mut contents = std::fs::read_to_string(&cargo_toml).unwrap();
    contents.push_str(&format!(
        "\n[patch.crates-io]\npinocchio = {{ path = {:?} }}\n",
        pinocchio_shim
    ));
    std::fs::write(cargo_toml, contents).unwrap();
}

fn cargo_check_workspace(ws: &Path) {
    let out = Command::new("cargo")
        .current_dir(ws)
        .args(["check", "--workspace", "--all-targets", "--offline"])
        .output()
        .expect("run cargo check");
    assert!(
        out.status.success(),
        "cargo check failed in {}\nstdout={}\nstderr={}",
        ws.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn patch_and_check(tmp: &Path, ws: &Path) {
    let anchor_shim = write_anchor_shim(tmp);
    let anchor_spl_shim = write_anchor_spl_shim(tmp);
    patch_anchor_deps(ws, &anchor_shim, &anchor_spl_shim);
    cargo_check_workspace(ws);
}

#[test]
fn generated_pinocchio_workspace_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_pinocchio_workspace(tmp.path(), "pinocchio_compile_app");
    assert_eq!(program, "pinocchio_compile_app");
    assert!(!ws.join("Anchor.toml").exists());
    let pinocchio_shim = write_pinocchio_shim(tmp.path());
    patch_pinocchio_dep(&ws, &pinocchio_shim);
    cargo_check_workspace(&ws);
}

#[test]
fn generated_workspace_with_all_phase2_scaffolders_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "compile_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "account",
            "Vault",
            "--program",
            &program,
            "--fields",
            "owner:Pubkey,total:u64",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "Ping",
            "--program",
            &program,
            "--fields",
            "",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "error",
            "Unauthorized",
            "--program",
            &program,
            "--msg",
            "caller is not authorized",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            &program,
            "--args",
            "amount:u64",
            "--emit",
            "Ping",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_token_program_accounts_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "token_accounts_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "transfer_tokens",
            "--program",
            &program,
            "--accounts",
            "token_program:token,associated_token_program:assoc_token,payer:signer,system_program:system",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_generic_accounts_and_pda_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "pda_accounts_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "Initialized",
            "--program",
            &program,
            "--fields",
            "authority:Pubkey,amount:u64",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "initialize_vault",
            "--program",
            &program,
            "--args",
            "amount:u64,memo:String",
            "--accounts",
            "vault:mut|seeds=b\"vault\";authority.key().as_ref(),authority:signer,system_program:system",
            "--emit",
            "Initialized",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_adr_style_account_syntax_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "adr_accounts_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "deposit_with_adr_accounts",
            "--program",
            &program,
            "--args",
            "amount:u64",
            "--accounts",
            "vault:mut:signer:seeds=b\"vault\";payer.key().as_ref(),payer:signer:mut,system_program,token_program,associated_token_program,ata_program:ata",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_multiple_instructions_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "multiple_ix_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "initialize",
            "--program",
            &program,
            "--accounts",
            "payer:signer:mut,system_program",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "deposit",
            "--program",
            &program,
            "--args",
            "amount:u64,memo:String",
            "--accounts",
            "vault:mut,payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_after_idempotent_rescaffolds_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "idempotent_app", "none");

    for _ in 0..2 {
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "account",
                "Vault",
                "--program",
                &program,
                "--fields",
                "owner:Pubkey,total:u64",
            ],
        );
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "event",
                "Deposited",
                "--program",
                &program,
                "--fields",
                "amount:u64",
            ],
        );
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "error",
                "Unauthorized",
                "--program",
                &program,
                "--msg",
                "caller is not authorized",
            ],
        );
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "instruction",
                "deposit",
                "--program",
                &program,
                "--args",
                "amount:u64",
                "--emit",
                "Deposited",
            ],
        );
    }

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_empty_scaffold_payloads_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "empty_payloads_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "account",
            "Marker",
            "--program",
            &program,
            "--fields",
            "",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "Ping",
            "--program",
            &program,
            "--fields",
            "",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "error",
            "Unknown",
            "--program",
            &program,
            "--msg",
            "",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_after_dry_run_then_real_scaffold_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "dry_run_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "preview_only",
            "--program",
            &program,
            "--args",
            "amount:u64",
            "--accounts",
            "payer:signer:mut,system_program",
            "--dry-run",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "preview_only",
            "--program",
            &program,
            "--args",
            "amount:u64",
            "--accounts",
            "payer:signer:mut,system_program",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_cased_identifiers_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "cased_identifiers_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "account",
            "UserProfile",
            "--program",
            &program,
            "--fields",
            "displayName:String,ownerPubkey:Pubkey",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "updateProfile",
            "--program",
            &program,
            "--args",
            "displayName:String",
            "--accounts",
            "userProfile:mut,payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_multiple_events_and_errors_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "events_errors_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "Opened",
            "--program",
            &program,
            "--fields",
            "authority:Pubkey",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "Closed",
            "--program",
            &program,
            "--fields",
            "authority:Pubkey,reason:String",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "error",
            "AlreadyOpen",
            "--program",
            &program,
            "--msg",
            "vault is already open",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "error",
            "AlreadyClosed",
            "--program",
            &program,
            "--msg",
            "vault is already closed",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "close_vault",
            "--program",
            &program,
            "--accounts",
            "authority:signer",
            "--emit",
            "Closed",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_full_scaffolds_in_two_programs_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "two_full_programs_app", "none");

    run_sunscreen(Some(&ws), &["scaffold", "program", "treasury"]);
    for program_name in [&program, "treasury"] {
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "account",
                "Vault",
                "--program",
                program_name,
                "--fields",
                "owner:Pubkey,total:u64",
            ],
        );
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "event",
                "Deposited",
                "--program",
                program_name,
                "--fields",
                "amount:u64",
            ],
        );
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "error",
                "Unauthorized",
                "--program",
                program_name,
                "--msg",
                "caller is not authorized",
            ],
        );
        run_sunscreen(
            Some(&ws),
            &[
                "scaffold",
                "instruction",
                "deposit",
                "--program",
                program_name,
                "--args",
                "amount:u64",
                "--accounts",
                "payer:signer",
                "--emit",
                "Deposited",
            ],
        );
    }

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_varied_argument_types_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "varied_args_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "configure",
            "--program",
            &program,
            "--args",
            "enabled:bool,count:u64,label:String,owner:Pubkey",
            "--accounts",
            "admin:signer:mut",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_only_account_scaffold_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "account_only_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "account",
            "Profile",
            "--program",
            &program,
            "--fields",
            "owner:Pubkey,active:bool,data:Vec<u8>",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_only_event_scaffold_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "event_only_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "ProfileUpdated",
            "--program",
            &program,
            "--fields",
            "owner:Pubkey,label:String,active:bool",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_only_error_scaffold_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "error_only_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "error",
            "MathOverflow",
            "--program",
            &program,
            "--msg",
            "calculation overflowed",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_dry_run_noun_scaffold_then_real_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "dry_run_nouns_app", "none");

    for noun_args in [
        ["account", "PreviewAccount", "--fields", "owner:Pubkey"].as_slice(),
        ["event", "PreviewEvent", "--fields", "owner:Pubkey"].as_slice(),
        ["error", "PreviewError", "--msg", "preview error"].as_slice(),
    ] {
        let mut dry_run_args = vec![
            "scaffold",
            noun_args[0],
            noun_args[1],
            "--program",
            &program,
        ];
        dry_run_args.extend_from_slice(&noun_args[2..]);
        dry_run_args.push("--dry-run");
        run_sunscreen(Some(&ws), &dry_run_args);

        let mut real_args = vec![
            "scaffold",
            noun_args[0],
            noun_args[1],
            "--program",
            &program,
        ];
        real_args.extend_from_slice(&noun_args[2..]);
        run_sunscreen(Some(&ws), &real_args);
    }

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_custom_program_id_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, _) = new_workspace(tmp.path(), "custom_program_id_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "program",
            "rewards",
            "--id",
            "11111111111111111111111111111111",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "claim",
            "--program",
            "rewards",
            "--accounts",
            "payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_global_json_scaffolds_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "json_scaffolds_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "--json",
            "scaffold",
            "account",
            "JsonVault",
            "--program",
            &program,
            "--fields",
            "owner:Pubkey",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "--json",
            "scaffold",
            "instruction",
            "json_ping",
            "--program",
            &program,
            "--accounts",
            "payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_global_json_chain_new_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("json_chain_new_app");

    run_sunscreen(
        None,
        &[
            "--json",
            "chain",
            "new",
            "json_chain_new_app",
            "--frontend",
            "none",
            "--path",
            ws.to_str().unwrap(),
        ],
    );
    let program = discover_program(&ws);
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "ping",
            "--program",
            &program,
            "--accounts",
            "payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_fielded_event_emit_types_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "fielded_emit_types_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "event",
            "Configured",
            "--program",
            &program,
            "--fields",
            "enabled:bool,count:u64,label:String,owner:Pubkey",
        ],
    );
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "configure",
            "--program",
            &program,
            "--args",
            "enabled:bool,count:u64,label:String,owner:Pubkey",
            "--accounts",
            "admin:signer",
            "--emit",
            "Configured",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_mixed_account_flag_styles_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, program) = new_workspace(tmp.path(), "mixed_account_flags_app", "none");

    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "mix_accounts",
            "--program",
            &program,
            "--accounts",
            "vault:mut|seeds=b\"vault\";payer.key().as_ref(),payer:signer:mut,token_program:token,ata_program:ata,system_program",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_normalized_project_name_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("normalized_name_app");

    run_sunscreen(
        None,
        &[
            "chain",
            "new",
            "NormalizedNameApp",
            "--frontend",
            "none",
            "--path",
            ws.to_str().unwrap(),
        ],
    );
    let program = discover_program(&ws);
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "normalized_ping",
            "--program",
            &program,
            "--accounts",
            "payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_next_frontend_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, _) = new_workspace(tmp.path(), "next_frontend_app", "next");

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_vite_frontend_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, _) = new_workspace(tmp.path(), "vite_frontend_app", "vite");

    patch_and_check(tmp.path(), &ws);
}

#[test]
fn generated_workspace_with_second_program_cargo_checks_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let (ws, _) = new_workspace(tmp.path(), "multi_program_app", "none");

    run_sunscreen(Some(&ws), &["scaffold", "program", "treasury"]);
    run_sunscreen(
        Some(&ws),
        &[
            "scaffold",
            "instruction",
            "ping",
            "--program",
            "treasury",
            "--accounts",
            "payer:signer",
        ],
    );

    patch_and_check(tmp.path(), &ws);
}
