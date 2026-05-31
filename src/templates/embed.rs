//! Compile-time embedded template assets.
//!
//! All files under `templates/assets/` are baked into the binary via
//! [`rust_embed::RustEmbed`], giving us hermetic, single-binary distribution.

use rust_embed::RustEmbed;

/// Embedded asset bundle rooted at `templates/assets/`.
#[derive(RustEmbed)]
#[folder = "templates/assets/"]
pub struct Assets;
