//! Additional golden snapshot matrix for `render_instruction` and friends.
//!
//! Complements `render_instruction.rs` with broader variant coverage:
//! - many args, zero accounts
//! - multi-account + multi-arg
//! - token-program / associated-token-program account kinds
//! - PDA with multi-component seeds
//! - dispatch segment (zero / one / many)
//! - instructions mod segment (sort + dedup)
//! - idempotency
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.

use sunscreen::templates::{
    render_dispatch_segment, render_instruction, render_instructions_mod_segment, AccountKind,
    AccountSpec, ArgSpec, InstructionCtx, InstructionDispatch,
};

fn many_args_no_accounts() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "configure".into(),
        args: vec![
            ArgSpec {
                name: "fee_bps".into(),
                ty: "u16".into(),
            },
            ArgSpec {
                name: "max_supply".into(),
                ty: "u64".into(),
            },
            ArgSpec {
                name: "name".into(),
                ty: "String".into(),
            },
            ArgSpec {
                name: "authority".into(),
                ty: "Pubkey".into(),
            },
        ],
        accounts: vec![],
        emit: None,
        emit_fields: vec![],
    }
}

fn ctx_token_program() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "mint_to_user".into(),
        args: vec![ArgSpec {
            name: "amount".into(),
            ty: "u64".into(),
        }],
        accounts: vec![
            AccountSpec {
                name: "mint".into(),
                mutable: true,
                signer: false,
                seeds: None,
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "to".into(),
                mutable: true,
                signer: false,
                seeds: None,
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "authority".into(),
                mutable: false,
                signer: true,
                seeds: None,
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "token_program".into(),
                mutable: false,
                signer: false,
                seeds: None,
                kind: AccountKind::TokenProgram,
            },
            AccountSpec {
                name: "associated_token_program".into(),
                mutable: false,
                signer: false,
                seeds: None,
                kind: AccountKind::AssociatedTokenProgram,
            },
            AccountSpec {
                name: "system_program".into(),
                mutable: false,
                signer: false,
                seeds: None,
                kind: AccountKind::SystemProgram,
            },
        ],
        emit: None,
        emit_fields: vec![],
    }
}

fn ctx_pda_multi_seeds() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "init_user_record".into(),
        args: vec![],
        accounts: vec![
            AccountSpec {
                name: "record".into(),
                mutable: true,
                signer: false,
                seeds: Some(vec![
                    "b\"record\"".into(),
                    "user.key().as_ref()".into(),
                    "&epoch.to_le_bytes()".into(),
                ]),
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "user".into(),
                mutable: false,
                signer: true,
                seeds: None,
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "system_program".into(),
                mutable: false,
                signer: false,
                seeds: None,
                kind: AccountKind::SystemProgram,
            },
        ],
        emit: None,
        emit_fields: vec![],
    }
}

fn ctx_emit_with_args_and_accounts() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "deposit".into(),
        args: vec![ArgSpec {
            name: "amount".into(),
            ty: "u64".into(),
        }],
        accounts: vec![
            AccountSpec {
                name: "user".into(),
                mutable: true,
                signer: true,
                seeds: None,
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "vault".into(),
                mutable: true,
                signer: false,
                seeds: None,
                kind: AccountKind::Generic,
            },
        ],
        emit: Some("Deposited".into()),
        emit_fields: vec![],
    }
}

#[test]
fn instruction_many_args_no_accounts() {
    let out = render_instruction(&many_args_no_accounts()).expect("render");
    insta::assert_snapshot!("instruction_many_args_no_accounts", out);
}

#[test]
fn instruction_token_program_accounts() {
    let out = render_instruction(&ctx_token_program()).expect("render");
    insta::assert_snapshot!("instruction_token_program_accounts", out);
}

#[test]
fn instruction_pda_multi_component_seeds() {
    let out = render_instruction(&ctx_pda_multi_seeds()).expect("render");
    insta::assert_snapshot!("instruction_pda_multi_component_seeds", out);
}

#[test]
fn instruction_emit_with_args_and_accounts() {
    let out = render_instruction(&ctx_emit_with_args_and_accounts()).expect("render");
    insta::assert_snapshot!("instruction_emit_with_args_and_accounts", out);
}

#[test]
fn instruction_render_is_idempotent() {
    let a = render_instruction(&ctx_token_program()).expect("render");
    let b = render_instruction(&ctx_token_program()).expect("render");
    assert_eq!(a, b);
}

#[test]
fn dispatch_segment_empty() {
    let out = render_dispatch_segment("demo", &[]);
    assert_eq!(out, "");
}

#[test]
fn dispatch_segment_single_no_args() {
    let out = render_dispatch_segment(
        "demo",
        &[InstructionDispatch {
            name: "ping".into(),
            args: vec![],
        }],
    );
    insta::assert_snapshot!("dispatch_segment_single_no_args", out);
}

#[test]
fn dispatch_segment_many_with_args() {
    let out = render_dispatch_segment(
        "demo",
        &[
            InstructionDispatch {
                name: "initialize".into(),
                args: vec![ArgSpec {
                    name: "seed".into(),
                    ty: "u64".into(),
                }],
            },
            InstructionDispatch {
                name: "deposit".into(),
                args: vec![
                    ArgSpec {
                        name: "amount".into(),
                        ty: "u64".into(),
                    },
                    ArgSpec {
                        name: "memo".into(),
                        ty: "String".into(),
                    },
                ],
            },
            InstructionDispatch {
                name: "close".into(),
                args: vec![],
            },
        ],
    );
    insta::assert_snapshot!("dispatch_segment_many_with_args", out);
}

#[test]
fn instructions_mod_segment_single() {
    let out = render_instructions_mod_segment(&["initialize".into()]);
    insta::assert_snapshot!("instructions_mod_segment_single", out);
}

#[test]
fn instructions_mod_segment_many_sorted_dedup() {
    let out = render_instructions_mod_segment(&[
        "deposit".into(),
        "initialize".into(),
        "close".into(),
        "deposit".into(),
        "withdraw".into(),
    ]);
    insta::assert_snapshot!("instructions_mod_segment_many_sorted_dedup", out);
}

#[test]
fn instructions_mod_segment_empty() {
    let out = render_instructions_mod_segment(&[]);
    assert_eq!(out, "");
}
