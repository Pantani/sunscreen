//! Golden snapshot tests for the embedded template engine.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.

use serde_json::json;
use sunscreen::templates::render;

#[test]
fn renders_version_template() {
    let ctx = json!({
        "version": "0.1.0",
        "name": "MyProject",
    });
    let out = render("version.txt.jinja", &ctx).expect("render");
    insta::assert_snapshot!("version_basic", out);
}
