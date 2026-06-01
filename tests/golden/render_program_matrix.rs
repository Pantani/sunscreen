//! Extra golden matrix for `render_program` covering name normalisation,
//! custom program_id, and idempotency.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.

use std::path::{Path, PathBuf};

use serde_json::json;
use sunscreen::templates::render_program;

fn walk_sorted(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_inner(root, &mut out);
    out.sort();
    out
}

fn walk_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_inner(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn corpus(root: &Path) -> String {
    let mut s = String::new();
    for path in walk_sorted(root) {
        let rel = path.strip_prefix(root).unwrap();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        s.push_str("=== ");
        s.push_str(&rel_str);
        s.push_str(" ===\n");
        match std::fs::read_to_string(&path) {
            Ok(c) => s.push_str(&c),
            Err(_) => s.push_str("<binary>\n"),
        }
        if !s.ends_with('\n') {
            s.push('\n');
        }
    }
    s
}

#[test]
fn scaffold_program_pascal_input_normalised_snake() {
    // program_name passed in PascalCase — path placeholder should be snake_case.
    let tmp = tempfile::tempdir().unwrap();
    let ctx = json!({
        "project_name": "MyDapp",
        "program_name": "TokenSale",
        "anchor_version": "0.30.1",
        "solana_version": "1.18.18",
        "rust_edition": "2021",
    });
    render_program(&ctx, tmp.path()).expect("render program");
    insta::assert_snapshot!(
        "scaffold_program_pascal_input_normalised_snake",
        corpus(tmp.path())
    );
}

#[test]
fn scaffold_program_with_custom_program_id() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = json!({
        "project_name": "MyDapp",
        "program_name": "swap",
        "anchor_version": "0.30.1",
        "solana_version": "1.18.18",
        "rust_edition": "2021",
        "program_id": "Sw4pNotReal11111111111111111111111111111111",
    });
    render_program(&ctx, tmp.path()).expect("render program");
    insta::assert_snapshot!(
        "scaffold_program_with_custom_program_id",
        corpus(tmp.path())
    );
}

#[test]
fn scaffold_program_render_is_idempotent() {
    let tmp_a = tempfile::tempdir().unwrap();
    let tmp_b = tempfile::tempdir().unwrap();
    let ctx = json!({
        "project_name": "MyDapp",
        "program_name": "voting",
        "anchor_version": "0.30.1",
        "solana_version": "1.18.18",
        "rust_edition": "2021",
    });
    render_program(&ctx, tmp_a.path()).expect("render a");
    render_program(&ctx, tmp_b.path()).expect("render b");
    assert_eq!(corpus(tmp_a.path()), corpus(tmp_b.path()));
}

#[test]
fn scaffold_program_missing_program_name_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = json!({ "project_name": "MyDapp" });
    let res = render_program(&ctx, tmp.path());
    assert!(res.is_err(), "missing program_name must error");
}

#[test]
fn scaffold_program_missing_project_name_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = json!({ "program_name": "voting" });
    let res = render_program(&ctx, tmp.path());
    assert!(res.is_err(), "missing project_name must error");
}
