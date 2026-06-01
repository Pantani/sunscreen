//! Tests for the Phase 3 watcher debounce core.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use sunscreen::runtime::watcher::{WatchDebouncer, WatchKind};

#[test]
fn watcher_batches_changes_after_quiet_period() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(100));

    assert!(watcher.observe("programs/demo/src/lib.rs", start).is_none());
    assert!(watcher
        .observe(
            "programs/demo/src/instructions/deposit.rs",
            start + Duration::from_millis(50)
        )
        .is_none());
    assert!(watcher
        .flush_due(start + Duration::from_millis(149))
        .is_none());

    let batch = watcher
        .flush_due(start + Duration::from_millis(150))
        .expect("debounced batch");
    assert_eq!(
        batch.paths,
        [
            PathBuf::from("programs/demo/src/instructions/deposit.rs"),
            PathBuf::from("programs/demo/src/lib.rs"),
        ]
    );
    assert_eq!(batch.kind, WatchKind::Pipeline);
    assert!(watcher
        .flush_due(start + Duration::from_millis(300))
        .is_none());
}

#[test]
fn watcher_deduplicates_paths_and_extends_deadline() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(100));

    watcher.observe("programs/demo/src/lib.rs", start);
    watcher.observe(
        "programs/demo/src/lib.rs",
        start + Duration::from_millis(90),
    );
    assert!(watcher
        .flush_due(start + Duration::from_millis(150))
        .is_none());

    let batch = watcher
        .flush_due(start + Duration::from_millis(190))
        .expect("debounced batch");
    assert_eq!(batch.paths, [PathBuf::from("programs/demo/src/lib.rs")]);
}

#[test]
fn watcher_ignores_generated_and_unrelated_paths() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(100));

    assert!(watcher.observe("target/idl/demo.json", start).is_none());
    assert!(watcher
        .observe("node_modules/pkg/index.js", start)
        .is_none());
    assert!(watcher.observe(".git/index", start).is_none());
    assert!(watcher.observe("README.md", start).is_none());
    assert!(watcher.flush_due(start + Duration::from_secs(1)).is_none());
}

#[test]
fn watcher_does_not_flush_due_batch_on_irrelevant_path() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(100));

    watcher.observe("programs/demo/src/lib.rs", start);
    assert!(watcher
        .observe("README.md", start + Duration::from_millis(100))
        .is_none());

    let batch = watcher
        .flush_due(start + Duration::from_millis(100))
        .expect("irrelevant path must not clear due batch");
    assert_eq!(batch.paths, [PathBuf::from("programs/demo/src/lib.rs")]);
}

#[test]
fn watcher_tracks_workspace_config_changes() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(25));

    watcher.observe("Anchor.toml", start);
    watcher.observe("sunscreen.yml", start + Duration::from_millis(5));
    watcher.observe("codama.json", start + Duration::from_millis(10));

    let batch = watcher
        .flush_due(start + Duration::from_millis(35))
        .expect("config batch");
    assert_eq!(
        batch.paths,
        [
            PathBuf::from("Anchor.toml"),
            PathBuf::from("codama.json"),
            PathBuf::from("sunscreen.yml"),
        ]
    );
    assert_eq!(batch.kind, WatchKind::Pipeline);
}

#[test]
fn watcher_tracks_cargo_manifest_changes() {
    let start = Instant::now();
    let mut watcher = WatchDebouncer::new(Duration::from_millis(25));

    watcher.observe("Cargo.toml", start);
    watcher.observe("Cargo.lock", start + Duration::from_millis(5));
    watcher.observe(
        "programs/demo/Cargo.toml",
        start + Duration::from_millis(10),
    );

    let batch = watcher
        .flush_due(start + Duration::from_millis(35))
        .expect("cargo manifest batch");
    assert_eq!(
        batch.paths,
        [
            PathBuf::from("Cargo.lock"),
            PathBuf::from("Cargo.toml"),
            PathBuf::from("programs/demo/Cargo.toml"),
        ]
    );
    assert_eq!(batch.kind, WatchKind::Pipeline);
}
