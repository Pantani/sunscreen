//! Two-phase filesystem transaction.
//!
//! Workflow:
//! 1. Construct with [`Transaction::new`] — a staging directory is created
//!    under the destination root.
//! 2. Call [`Transaction::stage`] one or more times to record files. Bytes
//!    are written immediately into the staging dir but never into the
//!    final destination.
//! 3. Inspect [`Transaction::plan`] (e.g. for `--dry-run`).
//! 4. Call [`Transaction::commit`] to atomically move files into place.
//!    On any error, every file already committed is removed (undo log).
//!
//! If the [`Transaction`] is dropped before `commit`, the staging
//! directory is removed.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

/// Errors emitted by [`Transaction`].
#[derive(Debug, Error)]
pub enum TxError {
    /// A staged path escapes the destination root (e.g. contains `..`).
    #[error("staged path escapes workspace: {0}")]
    PathEscape(String),
    /// A destination file already exists and would be overwritten.
    #[error("destination already exists: {0}")]
    DestinationExists(PathBuf),
    /// Attempted to stage the same logical path twice.
    #[error("path staged twice: {0}")]
    DuplicateStage(String),
    /// Generic IO error.
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// A staged entry: logical (workspace-relative) path + size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    /// Workspace-relative path (forward slashes).
    pub path: String,
    /// Size in bytes of the staged content.
    pub size: u64,
}

/// An in-place replacement of an existing file. The new bytes are written to
/// the staging dir up front; on [`Transaction::commit`] they atomically
/// overwrite `abs_dest`. The original bytes are captured eagerly so a failed
/// or rolled-back commit can restore the previous content.
struct ReplaceIntent {
    /// Absolute destination path of the file to overwrite.
    abs_dest: PathBuf,
    /// Staged file holding the new content.
    staged: PathBuf,
    /// Original bytes captured at stage time (for rollback/undo).
    original: Vec<u8>,
    /// Set once the staged content has been moved over `abs_dest`.
    replaced: bool,
}

/// Two-phase filesystem transaction.
pub struct Transaction {
    root: PathBuf,
    staging: PathBuf,
    planned: Vec<PlannedFile>,
    committed: Vec<PathBuf>,
    replaces: Vec<ReplaceIntent>,
    replace_counter: u64,
    finished: bool,
}

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Transaction {
    /// Create a new transaction rooted at `root`. `root` is created if
    /// it does not exist. A sibling staging directory is created inside
    /// `root` for atomic-rename writes.
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, TxError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let bump = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        let staging = root.join(format!(
            ".sunscreen-staging-{}-{}-{}",
            std::process::id(),
            nanos,
            bump
        ));
        fs::create_dir_all(&staging)?;
        Ok(Self {
            root,
            staging,
            planned: Vec::new(),
            committed: Vec::new(),
            replaces: Vec::new(),
            replace_counter: 0,
            finished: false,
        })
    }

    /// Destination root (the directory files will be committed into).
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to the staging directory (for tests / diagnostics).
    #[must_use]
    pub fn staging(&self) -> &Path {
        &self.staging
    }

    /// Stage a single file. `rel_path` must be workspace-relative,
    /// must not contain `..`, and must not collide with a previous
    /// stage call in the same transaction.
    pub fn stage(&mut self, rel_path: &str, contents: &[u8]) -> Result<(), TxError> {
        validate_rel(rel_path)?;
        if self.planned.iter().any(|p| p.path == rel_path) {
            return Err(TxError::DuplicateStage(rel_path.to_string()));
        }
        let staged_path = self.staging.join(rel_path);
        if let Some(parent) = staged_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&staged_path, contents)?;
        self.planned.push(PlannedFile {
            path: rel_path.to_string(),
            size: contents.len() as u64,
        });
        Ok(())
    }

    /// Stage an atomic in-place replacement of an **existing** file. Unlike
    /// [`Transaction::stage`] (which refuses to overwrite existing
    /// destinations), this records an intent to overwrite `path` with
    /// `new_bytes` when the transaction commits.
    ///
    /// `path` may be absolute or relative to the transaction root. The
    /// current contents of `path` are read immediately and retained so that a
    /// failed [`Transaction::commit`] or an explicit [`Transaction::rollback`]
    /// restores the file to its prior state. The new bytes are written into
    /// the staging dir now and moved over `path` at commit time. On most
    /// Unix filesystems this is an atomic `rename(2)`; on cross-device or
    /// Windows paths the implementation falls back to copy + remove, which
    /// is **not** atomic.
    pub fn stage_replace<P: AsRef<Path>>(
        &mut self,
        path: P,
        new_bytes: &[u8],
    ) -> Result<(), TxError> {
        let abs_dest = {
            let p = path.as_ref();
            if p.is_absolute() {
                // Absolute paths must be under the transaction root.
                if !p.starts_with(&self.root) {
                    return Err(TxError::PathEscape(p.to_string_lossy().into_owned()));
                }
                p.to_path_buf()
            } else {
                // Reject relative paths that escape the root.
                if p.components().any(|c| {
                    matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::Prefix(_)
                    )
                }) {
                    return Err(TxError::PathEscape(p.to_string_lossy().into_owned()));
                }
                self.root.join(p)
            }
        };
        // Capture original bytes now for rollback/undo.
        let original = fs::read(&abs_dest)?;
        // Write the new content into the staging dir under a unique name.
        let n = self.replace_counter;
        self.replace_counter += 1;
        let base = abs_dest
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("scratch");
        let staged = self.staging.join(format!("{base}.{n}.replace"));
        fs::write(&staged, new_bytes)?;
        self.replaces.push(ReplaceIntent {
            abs_dest,
            staged,
            original,
            replaced: false,
        });
        Ok(())
    }

    /// List of files that would be written by [`Transaction::commit`].
    /// Order matches stage order.
    #[must_use]
    pub fn plan(&self) -> &[PlannedFile] {
        &self.planned
    }

    /// Scan the staging directory for files written by an external
    /// renderer (e.g. `templates::render_workspace`) and register each
    /// one so [`Transaction::commit`] will move them into place. Paths
    /// already present in [`Transaction::plan`] are skipped (so it is
    /// safe to call after explicit [`Transaction::stage`] calls).
    pub fn adopt_staged_tree(&mut self) -> Result<(), TxError> {
        let mut found: Vec<(String, u64)> = Vec::new();
        collect(&self.staging, &self.staging, &mut found)?;
        found.sort_by(|a, b| a.0.cmp(&b.0));
        let known: std::collections::HashSet<String> =
            self.planned.iter().map(|p| p.path.clone()).collect();
        for (path, size) in found {
            if known.contains(&path) {
                continue;
            }
            self.planned.push(PlannedFile { path, size });
        }
        Ok(())
    }

    /// Atomically rename every staged file into place. On any per-file
    /// error, all files committed so far are removed and the staging
    /// dir is left for inspection (and cleaned by `Drop`).
    pub fn commit(mut self) -> Result<Vec<PathBuf>, TxError> {
        // Pre-flight: refuse to overwrite anything that already exists.
        for f in &self.planned {
            let dest = self.root.join(&f.path);
            if dest.exists() {
                return Err(TxError::DestinationExists(dest));
            }
        }

        // Move staged files into place.
        for f in &self.planned {
            let from = self.staging.join(&f.path);
            let to = self.root.join(&f.path);
            if let Err(e) = move_file(&from, &to) {
                self.undo();
                return Err(TxError::Io(e));
            }
            self.committed.push(to);
        }

        // Apply in-place replacements atomically. On any failure, undo the
        // new files and restore every already-replaced file's original bytes.
        for i in 0..self.replaces.len() {
            let from = self.replaces[i].staged.clone();
            let to = self.replaces[i].abs_dest.clone();
            if let Err(e) = move_file(&from, &to) {
                self.undo();
                return Err(TxError::Io(e));
            }
            self.replaces[i].replaced = true;
        }

        // Cleanup staging tree (best effort).
        let _ = fs::remove_dir_all(&self.staging);
        self.finished = true;
        Ok(std::mem::take(&mut self.committed))
    }

    /// Explicit rollback — discard staging dir without committing.
    pub fn rollback(mut self) -> Result<(), TxError> {
        self.cleanup_staging();
        self.finished = true;
        Ok(())
    }

    fn undo(&mut self) {
        for path in self.committed.drain(..) {
            let _ = fs::remove_file(&path);
        }
        // Restore originals for any replacement that was already applied.
        for r in &mut self.replaces {
            if r.replaced {
                let _ = fs::write(&r.abs_dest, &r.original);
                r.replaced = false;
            }
        }
    }

    fn cleanup_staging(&self) {
        let _ = fs::remove_dir_all(&self.staging);
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.finished {
            self.cleanup_staging();
        }
    }
}

fn validate_rel(rel: &str) -> Result<(), TxError> {
    if rel.is_empty() {
        return Err(TxError::PathEscape(rel.to_string()));
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(TxError::PathEscape(rel.to_string()));
    }
    for c in p.components() {
        match c {
            Component::Normal(_) => {}
            _ => return Err(TxError::PathEscape(rel.to_string())),
        }
    }
    Ok(())
}

/// Move `from` -> `to`, falling back to copy+remove across filesystems.
fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) -> Result<(), TxError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect(root, &p, out)?;
        } else {
            let rel = p
                .strip_prefix(root)
                .map_err(|e| TxError::Io(io::Error::other(e.to_string())))?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let size = entry.metadata()?.len();
            out.push((rel_str, size));
        }
    }
    Ok(())
}

fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(from, to)?;
            fs::remove_file(from)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn stage_and_commit_writes_files() {
        let dir = tempdir().unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage("a.txt", b"hello").unwrap();
        tx.stage("nested/b.txt", b"world").unwrap();

        let plan: Vec<_> = tx.plan().iter().map(|p| p.path.clone()).collect();
        assert_eq!(plan, vec!["a.txt", "nested/b.txt"]);

        let written = tx.commit().unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(dir.path().join("nested/b.txt")).unwrap(), b"world");
    }

    #[test]
    fn rollback_on_drop_removes_staging() {
        let dir = tempdir().unwrap();
        let staging_path;
        {
            let mut tx = Transaction::new(dir.path()).unwrap();
            tx.stage("a.txt", b"hello").unwrap();
            staging_path = tx.staging().to_path_buf();
            assert!(staging_path.exists());
        }
        assert!(!staging_path.exists(), "staging dir should be cleaned");
        assert!(!dir.path().join("a.txt").exists());
    }

    #[test]
    fn explicit_rollback_keeps_root_clean() {
        let dir = tempdir().unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage("a.txt", b"hello").unwrap();
        tx.rollback().unwrap();
        assert!(!dir.path().join("a.txt").exists());
    }

    #[test]
    fn refuses_to_overwrite_existing_destination() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"existing").unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage("a.txt", b"new").unwrap();
        let err = tx.commit().unwrap_err();
        assert!(matches!(err, TxError::DestinationExists(_)));
        assert_eq!(fs::read(dir.path().join("a.txt")).unwrap(), b"existing");
    }

    #[test]
    fn rejects_escaping_paths() {
        let dir = tempdir().unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        assert!(matches!(
            tx.stage("../escape.txt", b"x"),
            Err(TxError::PathEscape(_))
        ));
        assert!(matches!(
            tx.stage("/abs.txt", b"x"),
            Err(TxError::PathEscape(_))
        ));
        assert!(matches!(tx.stage("", b"x"), Err(TxError::PathEscape(_))));
    }

    #[test]
    fn rejects_duplicate_stage() {
        let dir = tempdir().unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage("a.txt", b"one").unwrap();
        let err = tx.stage("a.txt", b"two").unwrap_err();
        assert!(matches!(err, TxError::DuplicateStage(_)));
    }

    #[test]
    fn stage_replace_overwrites_existing_on_commit() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("lib.rs"), b"old").unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage("ix.rs", b"new file").unwrap();
        tx.stage_replace("lib.rs", b"patched").unwrap();
        tx.commit().unwrap();
        assert_eq!(fs::read(dir.path().join("lib.rs")).unwrap(), b"patched");
        assert_eq!(fs::read(dir.path().join("ix.rs")).unwrap(), b"new file");
    }

    #[test]
    fn stage_replace_restores_original_on_rollback() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("lib.rs"), b"original").unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage_replace("lib.rs", b"patched").unwrap();
        tx.rollback().unwrap();
        assert_eq!(fs::read(dir.path().join("lib.rs")).unwrap(), b"original");
    }

    #[test]
    fn stage_replace_undo_restores_on_new_file_collision() {
        // A `stage` destination collision aborts commit before replaces run,
        // so originals stay intact; a failure mid-replace must restore them.
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("lib.rs"), b"original").unwrap();
        fs::write(dir.path().join("a.txt"), b"exists").unwrap();
        let mut tx = Transaction::new(dir.path()).unwrap();
        tx.stage("a.txt", b"new").unwrap();
        tx.stage_replace("lib.rs", b"patched").unwrap();
        let err = tx.commit().unwrap_err();
        assert!(matches!(err, TxError::DestinationExists(_)));
        assert_eq!(fs::read(dir.path().join("lib.rs")).unwrap(), b"original");
    }

    #[test]
    fn idempotent_rerun_into_fresh_root() {
        // "Idempotency" at the transaction level = two sequential
        // transactions over fresh roots yield identical content.
        let make = |root: &Path| {
            let mut tx = Transaction::new(root).unwrap();
            tx.stage("a.txt", b"hello").unwrap();
            tx.stage("dir/b.txt", b"world").unwrap();
            tx.commit().unwrap();
        };
        let a = tempdir().unwrap();
        let b = tempdir().unwrap();
        make(a.path());
        make(b.path());
        assert_eq!(
            fs::read(a.path().join("a.txt")).unwrap(),
            fs::read(b.path().join("a.txt")).unwrap()
        );
        assert_eq!(
            fs::read(a.path().join("dir/b.txt")).unwrap(),
            fs::read(b.path().join("dir/b.txt")).unwrap()
        );
    }
}
