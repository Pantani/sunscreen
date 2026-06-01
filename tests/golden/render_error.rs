//! Golden snapshot tests for the `error` scaffold renderer.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.
//!
//! Matrix covered:
//! - single variant
//! - many variants
//! - empty variants (well-formed file with empty segment)
//! - variant message escaping (quotes, backslashes)
//! - PascalCase normalisation of enum + variant names
//! - idempotency

use sunscreen::templates::{
    escape_msg, render_error_variant, render_errors_file, ErrorCtx, ErrorVariant,
};

fn variants_three() -> Vec<ErrorVariant> {
    vec![
        ErrorVariant {
            name: "insufficient_funds".into(),
            message: "not enough lamports".into(),
        },
        ErrorVariant {
            name: "unauthorized".into(),
            message: "caller is not authorized".into(),
        },
        ErrorVariant {
            name: "overflow".into(),
            message: "arithmetic overflow".into(),
        },
    ]
}

#[test]
fn error_variant_single() {
    let out = render_error_variant(&ErrorVariant {
        name: "boom".into(),
        message: "kaboom".into(),
    })
    .expect("render");
    insta::assert_snapshot!("error_variant_single", out);
}

#[test]
fn error_variant_with_escapable_message() {
    let out = render_error_variant(&ErrorVariant {
        name: "bad_input".into(),
        message: r#"value "x" not allowed; use \n"#.into(),
    })
    .expect("render");
    insta::assert_snapshot!("error_variant_with_escapable_message", out);
}

#[test]
fn errors_file_single_variant() {
    let out = render_errors_file(&ErrorCtx {
        program_name: "demo".into(),
        enum_name: "demo_error".into(),
        variants: vec![ErrorVariant {
            name: "boom".into(),
            message: "kaboom".into(),
        }],
    })
    .expect("render");
    insta::assert_snapshot!("errors_file_single_variant", out);
}

#[test]
fn errors_file_many_variants() {
    let out = render_errors_file(&ErrorCtx {
        program_name: "demo".into(),
        enum_name: "demo_error".into(),
        variants: variants_three(),
    })
    .expect("render");
    insta::assert_snapshot!("errors_file_many_variants", out);
}

#[test]
fn errors_file_empty_variants() {
    let out = render_errors_file(&ErrorCtx {
        program_name: "demo".into(),
        enum_name: "demo_error".into(),
        variants: vec![],
    })
    .expect("render");
    insta::assert_snapshot!("errors_file_empty_variants", out);
}

#[test]
fn errors_file_pascal_case_enum_name() {
    let out = render_errors_file(&ErrorCtx {
        program_name: "demo".into(),
        enum_name: "my_program_error".into(),
        variants: vec![ErrorVariant {
            name: "x".into(),
            message: "m".into(),
        }],
    })
    .expect("render");
    insta::assert_snapshot!("errors_file_pascal_case_enum_name", out);
}

#[test]
fn errors_file_idempotent() {
    let ctx = ErrorCtx {
        program_name: "demo".into(),
        enum_name: "demo_error".into(),
        variants: variants_three(),
    };
    let a = render_errors_file(&ctx).expect("render");
    let b = render_errors_file(&ctx).expect("render");
    assert_eq!(a, b);
}

#[test]
fn escape_msg_handles_quotes_and_backslashes() {
    assert_eq!(escape_msg(r#"a"b\c"#), r#"a\"b\\c"#);
}
