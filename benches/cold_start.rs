//! Cold-start benchmark for `sunscreen --help`.
//!
//! Not a criterion bench — intentionally tiny so it can run in CI without
//! extra dev-deps. Invoke with `cargo bench --bench cold_start` after a
//! `cargo build --release`. Reports mean / p95 to stdout and never fails
//! the build (cold start is too environment-sensitive to gate CI on).

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

const RUNS: usize = 10;
const TARGET_MS: f64 = 50.0;

fn binary_path() -> PathBuf {
    if let Ok(p) = std::env::var("SUNSCREEN_BIN") {
        return PathBuf::from(p);
    }
    let mut p = std::env::current_exe().expect("current_exe");
    // bench binaries live at target/release/deps/<name>-<hash>
    // walk up to target/<profile>/.
    p.pop();
    if p.file_name().and_then(|s| s.to_str()) == Some("deps") {
        p.pop();
    }
    p.push("sunscreen");
    p
}

fn main() {
    let bin = binary_path();
    if !bin.exists() {
        eprintln!(
            "cold_start: binary not found at {} — run `cargo build --release` first",
            bin.display()
        );
        return;
    }

    let mut samples_ns: Vec<u128> = Vec::with_capacity(RUNS);
    for i in 0..RUNS {
        let start = Instant::now();
        let status = Command::new(&bin)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let elapsed = start.elapsed().as_nanos();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                eprintln!("cold_start: run {i} exited with {s}");
                return;
            }
            Err(e) => {
                eprintln!("cold_start: spawn failed: {e}");
                return;
            }
        }
        samples_ns.push(elapsed);
        println!("  run {i}: {:.2} ms", elapsed as f64 / 1e6);
    }

    samples_ns.sort_unstable();
    let n = samples_ns.len();
    let mean_ms = samples_ns.iter().sum::<u128>() as f64 / n as f64 / 1e6;
    let p95_idx = ((0.95 * n as f64).round() as usize)
        .saturating_sub(1)
        .min(n - 1);
    let p95_ms = samples_ns[p95_idx] as f64 / 1e6;
    let min_ms = samples_ns[0] as f64 / 1e6;
    let max_ms = samples_ns[n - 1] as f64 / 1e6;

    println!(
        "cold_start: mean={mean_ms:.2}ms p95={p95_ms:.2}ms min={min_ms:.2}ms max={max_ms:.2}ms (n={n})"
    );
    if p95_ms > TARGET_MS {
        println!("cold_start: WARN p95 {p95_ms:.2}ms exceeds {TARGET_MS}ms target (non-blocking)");
    }
}
