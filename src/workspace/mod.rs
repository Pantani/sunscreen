//! Workspace discovery and in-memory view.
//!
//! Walks the filesystem upward from a starting directory to locate a
//! `sunscreen.yml`, parses+validates it via the existing
//! [`crate::config`] loader, and cross-references the declared
//! `programs[]` against what actually exists on disk under
//! `programs/<snake_name>/`.
//!
//! Consumers (notably `scaffold instruction`) use [`find_root`] +
//! [`find_program`] to resolve concrete paths without re-implementing
//! the layout conventions.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use heck::ToSnakeCase;

use crate::config::{self, Config, ConfigError};
use crate::error::SunscreenError;

/// File name of the workspace manifest. Constant so callers do not
/// hard-code the literal in multiple places.
pub const MANIFEST_FILE: &str = "sunscreen.yml";

/// Fully resolved view of a sunscreen workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceRoot {
    /// Directory containing `sunscreen.yml`.
    pub root: PathBuf,
    /// Absolute path to the manifest file itself.
    pub config_path: PathBuf,
    /// Parsed + validated config.
    pub config: Config,
    /// Programs declared in config that also exist on disk, in config
    /// declaration order.
    pub programs: Vec<ProgramView>,
}

/// Resolved layout for one Anchor program inside the workspace.
#[derive(Debug, Clone)]
pub struct ProgramView {
    /// Program name as declared in `config.programs[].name` (kebab-case).
    pub name: String,
    /// Directory of the program crate (`<root>/<program.path>`).
    pub root: PathBuf,
    /// `<program.root>/src`.
    pub src_dir: PathBuf,
    /// `<program.root>/src/lib.rs`.
    pub lib_rs: PathBuf,
    /// `<program.root>/src/instructions`.
    pub instructions_dir: PathBuf,
    /// `<program.root>/src/instructions/mod.rs`.
    pub instructions_mod_rs: PathBuf,
}

/// Errors surfaced during workspace discovery / resolution.
#[derive(Debug)]
pub enum WorkspaceError {
    /// No `sunscreen.yml` was found walking up from the start path.
    NotFound,
    /// Config existed but failed to parse / validate.
    ConfigParse(ConfigError),
    /// A program declared in config has no matching directory on disk.
    OrphanProgram { name: String },
    /// `find_program` lookup miss.
    ProgramNotFound { name: String },
    /// I/O error during traversal / stat.
    Io(io::Error),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(
                f,
                "not inside a sunscreen workspace (no `{MANIFEST_FILE}` found in any parent)"
            ),
            Self::ConfigParse(e) => write!(f, "workspace config invalid: {e}"),
            Self::OrphanProgram { name } => write!(
                f,
                "program {name:?} declared in sunscreen.yml has no directory on disk"
            ),
            Self::ProgramNotFound { name } => write!(f, "no program named {name:?} in workspace"),
            Self::Io(e) => write!(f, "workspace io error: {e}"),
        }
    }
}

impl std::error::Error for WorkspaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConfigParse(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ConfigError> for WorkspaceError {
    fn from(e: ConfigError) -> Self {
        Self::ConfigParse(e)
    }
}

impl From<io::Error> for WorkspaceError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<WorkspaceError> for SunscreenError {
    fn from(e: WorkspaceError) -> Self {
        match e {
            WorkspaceError::NotFound => SunscreenError::UserInput(e.to_string()),
            WorkspaceError::ConfigParse(_) => SunscreenError::ConfigInvalid(e.to_string()),
            WorkspaceError::OrphanProgram { .. } => SunscreenError::ConfigInvalid(e.to_string()),
            WorkspaceError::ProgramNotFound { .. } => SunscreenError::UserInput(e.to_string()),
            WorkspaceError::Io(io_err) => {
                SunscreenError::Other(anyhow::anyhow!("workspace io: {io_err}"))
            }
        }
    }
}

/// Walk upward from `start` (or the current working directory) to find
/// the nearest `sunscreen.yml`, load it, and build a [`WorkspaceRoot`].
pub fn find_root(start: Option<&Path>) -> Result<WorkspaceRoot, WorkspaceError> {
    let start_buf: PathBuf = match start {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()?,
    };

    let manifest = walk_up_for_manifest(&start_buf).ok_or(WorkspaceError::NotFound)?;
    let root = manifest
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let cfg = config::load(Some(&manifest))?;
    cfg.validate()
        .map_err(|e| WorkspaceError::ConfigParse(ConfigError::Validation { msg: e.to_string() }))?;

    let programs = resolve_programs(&root, &cfg)?;

    Ok(WorkspaceRoot {
        root,
        config_path: manifest,
        config: cfg,
        programs,
    })
}

/// Look up a program by name (case-insensitive on its snake_case form).
pub fn find_program<'a>(
    ws: &'a WorkspaceRoot,
    name: &str,
) -> Result<&'a ProgramView, WorkspaceError> {
    let needle = name.to_snake_case().to_lowercase();
    ws.programs
        .iter()
        .find(|p| p.name.to_snake_case().to_lowercase() == needle)
        .ok_or_else(|| WorkspaceError::ProgramNotFound {
            name: name.to_string(),
        })
}

fn walk_up_for_manifest(start: &Path) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join(MANIFEST_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

fn resolve_programs(root: &Path, cfg: &Config) -> Result<Vec<ProgramView>, WorkspaceError> {
    let mut out = Vec::with_capacity(cfg.programs.len());
    for prog in &cfg.programs {
        let snake = prog.name.to_snake_case();
        let prog_root = root.join(&prog.path);
        // Also accept the alternative location `programs/<snake_name>/`
        // when the configured path does not exist, so configs that store
        // kebab-named paths still resolve cleanly.
        let resolved_root = if prog_root.is_dir() {
            prog_root
        } else {
            let alt = root.join("programs").join(&snake);
            if alt.is_dir() {
                alt
            } else {
                return Err(WorkspaceError::OrphanProgram {
                    name: prog.name.clone(),
                });
            }
        };

        let src_dir = resolved_root.join("src");
        let instructions_dir = src_dir.join("instructions");
        out.push(ProgramView {
            name: prog.name.clone(),
            lib_rs: src_dir.join("lib.rs"),
            instructions_mod_rs: instructions_dir.join("mod.rs"),
            instructions_dir,
            src_dir,
            root: resolved_root,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const MIN_YML: &str = r#"version: 1
project:
  name: my-proto
  framework: anchor
programs:
  - name: my-proto
    path: programs/my-proto
"#;

    fn seed_workspace(root: &Path) {
        fs::write(root.join(MANIFEST_FILE), MIN_YML).unwrap();
        let prog = root.join("programs/my-proto/src/instructions");
        fs::create_dir_all(&prog).unwrap();
        fs::write(prog.join("mod.rs"), "// noop\n").unwrap();
        fs::write(prog.parent().unwrap().join("lib.rs"), "// noop\n").unwrap();
    }

    #[test]
    fn find_root_locates_yml_in_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        seed_workspace(tmp.path());
        let ws = find_root(Some(tmp.path())).expect("workspace");
        assert_eq!(ws.root, tmp.path());
        assert_eq!(ws.config_path, tmp.path().join(MANIFEST_FILE));
        assert_eq!(ws.programs.len(), 1);
    }

    #[test]
    fn find_root_walks_up_to_parent() {
        let tmp = tempfile::tempdir().unwrap();
        seed_workspace(tmp.path());
        let nested = tmp.path().join("programs/my-proto/src");
        let ws = find_root(Some(&nested)).expect("workspace");
        assert_eq!(ws.root, tmp.path());
    }

    #[test]
    fn find_root_returns_not_found_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        match find_root(Some(tmp.path())) {
            Err(WorkspaceError::NotFound) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn programs_are_resolved_from_config_and_disk() {
        let tmp = tempfile::tempdir().unwrap();
        seed_workspace(tmp.path());
        let ws = find_root(Some(tmp.path())).expect("workspace");
        let p = find_program(&ws, "my-proto").expect("lookup");
        assert_eq!(p.name, "my-proto");
        assert_eq!(p.root, tmp.path().join("programs/my-proto"));
        assert_eq!(p.lib_rs, tmp.path().join("programs/my-proto/src/lib.rs"));
        assert_eq!(
            p.instructions_mod_rs,
            tmp.path().join("programs/my-proto/src/instructions/mod.rs")
        );
        // Lookup is case-insensitive on the snake_case form.
        assert!(find_program(&ws, "MY_PROTO").is_ok());
        assert!(matches!(
            find_program(&ws, "ghost"),
            Err(WorkspaceError::ProgramNotFound { .. })
        ));
    }

    #[test]
    fn orphan_program_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(MANIFEST_FILE), MIN_YML).unwrap();
        // intentionally do NOT create programs/my-proto/
        match find_root(Some(tmp.path())) {
            Err(WorkspaceError::OrphanProgram { name }) => assert_eq!(name, "my-proto"),
            other => panic!("expected OrphanProgram, got {other:?}"),
        }
    }
}
