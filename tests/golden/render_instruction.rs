//! Golden snapshot tests for the Anchor instruction scaffold.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.

use sunscreen::templates::{render_instruction, AccountKind, AccountSpec, ArgSpec, InstructionCtx};

fn ctx_simple() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "initialize".into(),
        args: vec![],
        accounts: vec![AccountSpec {
            name: "payer".into(),
            mutable: true,
            signer: true,
            seeds: None,
            kind: AccountKind::Generic,
        }],
        emit: None,
        emit_fields: vec![],
    }
}

fn ctx_with_pda_and_args() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "create_vault".into(),
        args: vec![
            ArgSpec {
                name: "amount".into(),
                ty: "u64".into(),
            },
            ArgSpec {
                name: "label".into(),
                ty: "String".into(),
            },
        ],
        accounts: vec![
            AccountSpec {
                name: "vault".into(),
                mutable: true,
                signer: false,
                seeds: Some(vec!["b\"vault\"".into(), "payer.key().as_ref()".into()]),
                kind: AccountKind::Generic,
            },
            AccountSpec {
                name: "payer".into(),
                mutable: true,
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

fn ctx_with_emit() -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: "ping".into(),
        args: vec![ArgSpec {
            name: "nonce".into(),
            ty: "u64".into(),
        }],
        accounts: vec![AccountSpec {
            name: "user".into(),
            mutable: false,
            signer: true,
            seeds: None,
            kind: AccountKind::Generic,
        }],
        emit: Some("Pinged".into()),
        emit_fields: vec!["nonce".into()],
    }
}

#[test]
fn instruction_simple() {
    let out = render_instruction(&ctx_simple()).expect("render");
    insta::assert_snapshot!("instruction_simple", out);
}

#[test]
fn instruction_with_pda_seeds() {
    let out = render_instruction(&ctx_with_pda_and_args()).expect("render");
    insta::assert_snapshot!("instruction_with_pda_seeds", out);
}

#[test]
fn instruction_with_emit() {
    let out = render_instruction(&ctx_with_emit()).expect("render");
    insta::assert_snapshot!("instruction_with_emit", out);
}

#[test]
fn markers_are_verbatim() {
    let out = render_instruction(&ctx_simple()).expect("render");
    assert!(out.contains(
        "// === sunscreen:auto-generated:begin segment=file version=1 generator=instruction ==="
    ));
    assert!(out.contains("// === sunscreen:auto-generated:end segment=file ==="));
    assert!(out.contains("// === sunscreen:user-region:begin segment=handler ==="));
    assert!(out.contains("// === sunscreen:user-region:end segment=handler ==="));
}
