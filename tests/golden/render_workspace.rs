//! Golden snapshot tests for `render_workspace`.
//!
//! Each test renders a workspace template into a tempdir, walks the resulting
//! tree in deterministic order, and snapshots the full corpus as a single
//! `=== <relpath> ===\n<contents>\n` block via `insta`.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.

use std::path::{Path, PathBuf};

use serde_json::json;
use sunscreen::templates::{render_workspace, TemplateError};

fn base_ctx() -> serde_json::Value {
    json!({
        "project_name": "MyDapp",
        "program_name": "my_dapp",
        "anchor_version": "0.30.1",
        "solana_version": "1.18.18",
        "rust_edition": "2021",
        "frontend": "none",
        "cluster": "localnet",
    })
}

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

fn render_one(name: &str, ctx: &serde_json::Value, dest: &Path) -> Result<(), TemplateError> {
    render_workspace(name, ctx, dest).map(|_| ())
}

#[test]
fn anchor_multiple_frontend_none() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ctx = base_ctx();
    ctx["frontend"] = json!("none");
    render_one("anchor-multiple", &ctx, tmp.path()).expect("anchor render");
    render_one("frontend-none", &ctx, tmp.path()).expect("frontend-none render");
    insta::assert_snapshot!("anchor_multiple_frontend_none", corpus(tmp.path()));
}

#[test]
fn anchor_multiple_frontend_next() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ctx = base_ctx();
    ctx["frontend"] = json!("next");
    render_one("anchor-multiple", &ctx, tmp.path()).expect("anchor render");
    let app_root = tmp.path().join("app");
    std::fs::create_dir_all(&app_root).unwrap();
    render_one("frontend-next", &ctx, &app_root).expect("frontend-next render");
    insta::assert_snapshot!("anchor_multiple_frontend_next", corpus(tmp.path()));
}

#[test]
fn anchor_multiple_frontend_vite() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ctx = base_ctx();
    ctx["frontend"] = json!("vite");
    render_one("anchor-multiple", &ctx, tmp.path()).expect("anchor render");
    let app_root = tmp.path().join("app");
    std::fs::create_dir_all(&app_root).unwrap();
    render_one("frontend-vite", &ctx, &app_root).expect("frontend-vite render");
    insta::assert_snapshot!("anchor_multiple_frontend_vite", corpus(tmp.path()));
}

#[test]
fn unknown_template_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = base_ctx();
    let err = render_workspace("does-not-exist", &ctx, tmp.path()).unwrap_err();
    assert!(matches!(err, TemplateError::NotFound { .. }));
}
