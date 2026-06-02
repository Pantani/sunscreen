//! Composite scaffold recipes.
//!
//! Phase 5 recipes are data plans. The CLI layer owns execution so it can
//! reuse the existing marker-based scaffolders without duplicating Rust
//! rendering logic.

pub mod crud;
pub mod recipes;

use std::path::PathBuf;

/// A complete recipe expansion.
#[derive(Debug, Clone)]
pub struct RecipePlan {
    pub kind: RecipeKind,
    pub resource: String,
    pub steps: Vec<RecipeStep>,
    pub files: Vec<GeneratedFile>,
}

/// Stable recipe names for user-facing reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeKind {
    Crud,
    SplToken,
    MetaplexNft,
}

impl RecipeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Crud => "crud",
            Self::SplToken => "spl-token",
            Self::MetaplexNft => "metaplex-nft",
        }
    }
}

/// One primitive scaffolder invocation.
#[derive(Debug, Clone)]
pub enum RecipeStep {
    Account {
        name: String,
        fields: String,
    },
    Event {
        name: String,
        fields: String,
    },
    Error {
        name: String,
        message: String,
    },
    Instruction {
        name: String,
        args: String,
        accounts: String,
        emit: Option<String>,
    },
}

/// Additional file emitted by a recipe.
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

fn generated_file(path: impl Into<PathBuf>, contents: impl Into<String>) -> GeneratedFile {
    GeneratedFile {
        relative_path: path.into(),
        contents: contents.into(),
    }
}
