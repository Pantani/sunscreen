//! Golden snapshot tests for the `account` scaffold renderer.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.
//!
//! Matrix covered:
//! - happy path (single + multi-field accounts)
//! - empty struct (zero fields)
//! - complex types (Pubkey, fixed arrays, Vec, Option)
//! - mod-segment entry rendering
//! - multi-account mod segment (sort + dedup determinism)
//! - idempotency: rendering twice yields identical bytes

use sunscreen::templates::account::FieldSpec;
use sunscreen::templates::{
    render_account_file, render_account_mod_entry, render_account_mod_segment, AccountCtx,
};

fn ctx_vault() -> AccountCtx {
    AccountCtx {
        program_name: "demo".into(),
        account_name: "vault".into(),
        fields: vec![
            FieldSpec {
                name: "owner".into(),
                ty: "Pubkey".into(),
            },
            FieldSpec {
                name: "total".into(),
                ty: "u64".into(),
            },
        ],
    }
}

#[test]
fn account_file_vault_basic() {
    let out = render_account_file(&ctx_vault()).expect("render");
    insta::assert_snapshot!("account_file_vault_basic", out);
}

#[test]
fn account_file_empty_fields() {
    let ctx = AccountCtx {
        program_name: "demo".into(),
        account_name: "marker".into(),
        fields: vec![],
    };
    let out = render_account_file(&ctx).expect("render");
    insta::assert_snapshot!("account_file_empty_fields", out);
}

#[test]
fn account_file_many_field_types() {
    let ctx = AccountCtx {
        program_name: "demo".into(),
        account_name: "treasury".into(),
        fields: vec![
            FieldSpec {
                name: "authority".into(),
                ty: "Pubkey".into(),
            },
            FieldSpec {
                name: "bump".into(),
                ty: "u8".into(),
            },
            FieldSpec {
                name: "seed".into(),
                ty: "[u8; 32]".into(),
            },
            FieldSpec {
                name: "name".into(),
                ty: "String".into(),
            },
            FieldSpec {
                name: "members".into(),
                ty: "Vec<Pubkey>".into(),
            },
            FieldSpec {
                name: "delegate".into(),
                ty: "Option<Pubkey>".into(),
            },
        ],
    };
    let out = render_account_file(&ctx).expect("render");
    insta::assert_snapshot!("account_file_many_field_types", out);
}

#[test]
fn account_file_pascal_case_normalization() {
    // Caller passes a non-snake account_name; template should pascal-case it.
    let ctx = AccountCtx {
        program_name: "demo".into(),
        account_name: "user_profile".into(),
        fields: vec![FieldSpec {
            name: "level".into(),
            ty: "u32".into(),
        }],
    };
    let out = render_account_file(&ctx).expect("render");
    insta::assert_snapshot!("account_file_pascal_case_normalization", out);
}

#[test]
fn account_mod_entry_single() {
    let out = render_account_mod_entry("vault").expect("render");
    insta::assert_snapshot!("account_mod_entry_single", out);
}

#[test]
fn account_mod_segment_single() {
    let out = render_account_mod_segment(&["vault".into()]);
    insta::assert_snapshot!("account_mod_segment_single", out);
}

#[test]
fn account_mod_segment_many_sorted_dedup() {
    let out = render_account_mod_segment(&[
        "Vault".into(),
        "Treasury".into(),
        "vault".into(),
        "UserProfile".into(),
        "audit_log".into(),
    ]);
    insta::assert_snapshot!("account_mod_segment_many_sorted_dedup", out);
}

#[test]
fn account_mod_segment_empty() {
    let out = render_account_mod_segment(&[]);
    assert_eq!(out, "");
}

#[test]
fn account_file_idempotent_render() {
    let a = render_account_file(&ctx_vault()).expect("render");
    let b = render_account_file(&ctx_vault()).expect("render");
    assert_eq!(a, b, "render should be deterministic");
}

#[test]
fn account_file_has_marker_contract() {
    let out = render_account_file(&ctx_vault()).expect("render");
    assert!(out.contains("segment=file version=1 generator=account"));
    assert!(out.contains("// === sunscreen:auto-generated:end segment=file ==="));
}
