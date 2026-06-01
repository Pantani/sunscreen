//! Tool probing and parallel detection.
//!
//! The [`CommandRunner`] trait abstracts over `std::process::Command` so
//! tests can substitute a deterministic mock without touching the
//! filesystem or spawning subprocesses.

use std::collections::BTreeMap;
use std::path::PathBuf;

use regex::Regex;
use semver::Version;
use serde::{Serialize, Serializer};

use super::registry::ToolSpec;

/// Strategy for resolving and invoking external tools.
///
/// `Send + Sync` so detection can fan out across `std::thread::scope`.
pub trait CommandRunner: Send + Sync {
    /// Execute `<bin> <args...>` and return combined stdout+stderr on
    /// success. Returns `None` if the process cannot be spawned or exits
    /// with a non-zero status with no usable output.
    fn run(&self, bin: &str, args: &[&str]) -> Option<String>;

    /// Resolve the absolute path of `bin` on `$PATH`.
    fn which(&self, bin: &str) -> Option<PathBuf>;
}

/// Production runner backed by `std::process::Command` and the `which` crate.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealRunner;

impl CommandRunner for RealRunner {
    fn run(&self, bin: &str, args: &[&str]) -> Option<String> {
        let out = std::process::Command::new(bin).args(args).output().ok()?;
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        if s.trim().is_empty() {
            s = String::from_utf8_lossy(&out.stderr).into_owned();
        }
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }

    fn which(&self, bin: &str) -> Option<PathBuf> {
        which::which(bin).ok()
    }
}

/// Result of a single tool probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Tool present and (if required) at or above minimum.
    Ok,
    /// Required tool not on `$PATH`.
    MissingRequired,
    /// Optional tool not on `$PATH` — not a failure.
    MissingOptional,
    /// Tool present but version is below the configured minimum.
    BelowMin,
    /// Tool present but version output could not be parsed.
    UnknownVersion,
}

/// Per-tool detection report.
///
/// `available` is a coarse boolean intended for downstream skip-logic
/// (e.g. integration tests that need `anchor build` or `codama`): it is
/// `true` iff the binary was found on `$PATH` *and* the probe yielded a
/// usable version at or above the configured minimum. Tools whose
/// version could not be parsed are treated as unavailable for skip
/// purposes even though `found` is `true`.
#[derive(Debug, Clone, Serialize)]
pub struct ToolReport {
    pub name: String,
    pub found: bool,
    pub available: bool,
    pub path: Option<PathBuf>,
    #[serde(serialize_with = "ser_version")]
    pub version: Option<Version>,
    pub required: bool,
    #[serde(serialize_with = "ser_version")]
    pub min_version: Option<Version>,
    pub status: Status,
}

impl ToolReport {
    /// Returns true when the tool is on `$PATH` and meets the minimum
    /// version requirement (or has no minimum). Convenient for callers
    /// that just want a yes/no.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available
    }
}

fn ser_version<S: Serializer>(v: &Option<Version>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(v) => s.serialize_some(&v.to_string()),
        None => s.serialize_none(),
    }
}

/// Run detection for every spec in parallel.
///
/// `overrides` maps tool name -> minimum semver string and replaces the
/// `default_min` defined in the [`ToolSpec`]. Invalid override strings are
/// silently ignored (falling back to the spec default).
#[must_use]
pub fn detect_all<R: CommandRunner>(
    runner: &R,
    specs: &[ToolSpec],
    overrides: &BTreeMap<String, String>,
) -> Vec<ToolReport> {
    let mut reports: Vec<Option<ToolReport>> = (0..specs.len()).map(|_| None).collect();

    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(specs.len());
        for spec in specs {
            let min = overrides
                .get(spec.name)
                .and_then(|s| Version::parse(s).ok())
                .or_else(|| spec.default_min.and_then(|s| Version::parse(s).ok()));
            handles.push(scope.spawn(move || probe_one(runner, spec, min)));
        }
        for (slot, h) in reports.iter_mut().zip(handles) {
            *slot = Some(h.join().expect("toolchain probe panicked"));
        }
    });

    reports.into_iter().map(Option::unwrap).collect()
}

fn probe_one<R: CommandRunner>(runner: &R, spec: &ToolSpec, min: Option<Version>) -> ToolReport {
    let path = runner.which(spec.bin);
    let Some(path) = path else {
        let status = if spec.required {
            Status::MissingRequired
        } else {
            Status::MissingOptional
        };
        return ToolReport {
            name: spec.name.to_string(),
            found: false,
            available: false,
            path: None,
            version: None,
            required: spec.required,
            min_version: min,
            status,
        };
    };

    let raw = runner.run(spec.bin, &[spec.version_arg]);
    let version = raw
        .as_deref()
        .and_then(|s| parse_version(s, spec.version_regex));

    let status = match (&version, &min) {
        (None, _) => Status::UnknownVersion,
        (Some(v), Some(m)) if v < m => Status::BelowMin,
        (Some(_), _) => Status::Ok,
    };

    let available = matches!(status, Status::Ok);

    ToolReport {
        name: spec.name.to_string(),
        found: true,
        available,
        path: Some(path),
        version,
        required: spec.required,
        min_version: min,
        status,
    }
}

/// Probe a single tool by registry name. Returns `None` when `name`
/// does not match any known [`ToolSpec`].
#[must_use]
pub fn detect_one<R: CommandRunner>(
    runner: &R,
    name: &str,
    overrides: &BTreeMap<String, String>,
) -> Option<ToolReport> {
    let specs = super::registry::known();
    let spec = specs.iter().find(|s| s.name == name)?;
    let min = overrides
        .get(spec.name)
        .and_then(|s| Version::parse(s).ok())
        .or_else(|| spec.default_min.and_then(|s| Version::parse(s).ok()));
    Some(probe_one(runner, spec, min))
}

/// Convenience: is `name` available on the current `$PATH` and meeting
/// its minimum version? Uses [`RealRunner`] with no overrides.
///
/// Intended for integration tests that need to skip when an external
/// toolchain dependency is absent (e.g. `anchor build`, `codama`).
#[must_use]
pub fn is_available(name: &str) -> bool {
    detect_one(&RealRunner, name, &BTreeMap::new())
        .map(|r| r.is_available())
        .unwrap_or(false)
}

/// Shorthand: `anchor` available?
#[must_use]
pub fn detect_anchor() -> bool {
    is_available("anchor")
}

/// Shorthand: `codama` available?
#[must_use]
pub fn detect_codama() -> bool {
    is_available("codama")
}

/// Shorthand: `solana` available?
#[must_use]
pub fn detect_solana() -> bool {
    is_available("solana")
}

/// Shorthand: `surfpool` available?
#[must_use]
pub fn detect_surfpool() -> bool {
    is_available("surfpool")
}

/// Shorthand: `rustfmt` available?
#[must_use]
pub fn detect_rustfmt() -> bool {
    is_available("rustfmt")
}

fn parse_version(haystack: &str, pattern: &str) -> Option<Version> {
    let re = Regex::new(pattern).ok()?;
    let caps = re.captures(haystack)?;
    let raw = caps.get(1)?.as_str();
    Version::parse(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockRunner {
        responses: HashMap<String, String>,
        paths: HashMap<String, PathBuf>,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, bin: &str, _args: &[&str]) -> Option<String> {
            self.responses.get(bin).cloned()
        }
        fn which(&self, bin: &str) -> Option<PathBuf> {
            self.paths.get(bin).cloned()
        }
    }

    fn specs() -> Vec<ToolSpec> {
        crate::toolchain::registry::known()
    }

    fn all_ok_runner() -> MockRunner {
        let mut responses = HashMap::new();
        let mut paths = HashMap::new();
        for s in specs() {
            paths.insert(
                s.bin.to_string(),
                PathBuf::from(format!("/usr/bin/{}", s.bin)),
            );
        }
        responses.insert("anchor".into(), "anchor-cli 1.5.0\n".into());
        responses.insert(
            "solana".into(),
            "solana-cli 2.1.0 (src:abc; feat:1)\n".into(),
        );
        responses.insert("rustc".into(), "rustc 1.80.0 (abc 2024-01-01)\n".into());
        responses.insert("cargo".into(), "cargo 1.80.0\n".into());
        responses.insert("node".into(), "v20.10.0\n".into());
        responses.insert("pnpm".into(), "9.4.0\n".into());
        responses.insert("codama".into(), "0.1.0\n".into());
        responses.insert("surfpool".into(), "0.2.0\n".into());
        responses.insert(
            "rustfmt".into(),
            "rustfmt 1.7.0-stable (abc 2024-01-01)\n".into(),
        );
        MockRunner { responses, paths }
    }

    fn any_required_failed(reports: &[ToolReport]) -> bool {
        reports.iter().any(|r| {
            r.required
                && matches!(
                    r.status,
                    Status::MissingRequired | Status::BelowMin | Status::UnknownVersion
                )
        })
    }

    #[test]
    fn test_all_ok() {
        let runner = all_ok_runner();
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        for r in &reports {
            assert!(
                matches!(r.status, Status::Ok),
                "tool {} unexpectedly {:?}",
                r.name,
                r.status
            );
        }
        assert!(!any_required_failed(&reports));
    }

    #[test]
    fn test_missing_required() {
        let mut runner = all_ok_runner();
        runner.paths.remove("anchor");
        runner.responses.remove("anchor");
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        let anchor = reports.iter().find(|r| r.name == "anchor").unwrap();
        assert_eq!(anchor.status, Status::MissingRequired);
        assert!(!anchor.found);
        assert!(any_required_failed(&reports));
    }

    #[test]
    fn test_below_min() {
        let mut runner = all_ok_runner();
        runner
            .responses
            .insert("rustc".into(), "rustc 1.60.0 (abc 2022-01-01)\n".into());
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        let rustc = reports.iter().find(|r| r.name == "rustc").unwrap();
        assert_eq!(rustc.status, Status::BelowMin);
        assert!(any_required_failed(&reports));
    }

    #[test]
    fn test_unknown_version() {
        let mut runner = all_ok_runner();
        runner
            .responses
            .insert("anchor".into(), "no version here\n".into());
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        let anchor = reports.iter().find(|r| r.name == "anchor").unwrap();
        assert_eq!(anchor.status, Status::UnknownVersion);
        assert!(anchor.found);
        assert!(any_required_failed(&reports));
    }

    #[test]
    fn test_optional_missing_doesnt_fail() {
        let mut runner = all_ok_runner();
        runner.paths.remove("codama");
        runner.paths.remove("surfpool");
        runner.responses.remove("codama");
        runner.responses.remove("surfpool");
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        let codama = reports.iter().find(|r| r.name == "codama").unwrap();
        assert_eq!(codama.status, Status::MissingOptional);
        assert!(!any_required_failed(&reports));
    }

    #[test]
    fn test_report_available_flag_when_present() {
        let runner = all_ok_runner();
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        for r in &reports {
            assert!(
                r.available,
                "tool {} should be available (status={:?})",
                r.name, r.status
            );
            assert!(r.version.is_some(), "tool {} should have version", r.name);
        }
    }

    #[test]
    fn test_report_available_false_when_missing() {
        let mut runner = all_ok_runner();
        runner.paths.remove("anchor");
        runner.responses.remove("anchor");
        runner.paths.remove("codama");
        runner.responses.remove("codama");
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        let anchor = reports.iter().find(|r| r.name == "anchor").unwrap();
        let codama = reports.iter().find(|r| r.name == "codama").unwrap();
        assert!(!anchor.available, "missing anchor must be unavailable");
        assert!(!codama.available, "missing codama must be unavailable");
    }

    #[test]
    fn test_report_available_false_when_below_min() {
        let mut runner = all_ok_runner();
        runner
            .responses
            .insert("rustc".into(), "rustc 1.60.0 (abc 2022-01-01)\n".into());
        let reports = detect_all(&runner, &specs(), &BTreeMap::new());
        let rustc = reports.iter().find(|r| r.name == "rustc").unwrap();
        assert!(
            !rustc.available,
            "below-min rustc must not report available=true"
        );
    }

    #[test]
    fn test_detect_one_unknown_returns_none() {
        let runner = all_ok_runner();
        assert!(detect_one(&runner, "nonexistent-tool", &BTreeMap::new()).is_none());
    }

    #[test]
    fn test_detect_one_returns_single_report() {
        let runner = all_ok_runner();
        let r = detect_one(&runner, "anchor", &BTreeMap::new()).expect("anchor spec exists");
        assert_eq!(r.name, "anchor");
        assert!(r.available);
        assert!(r.version.is_some());
    }

    #[test]
    fn test_override_min_version() {
        let runner = all_ok_runner();
        let mut overrides = BTreeMap::new();
        overrides.insert("node".to_string(), "22.0.0".to_string());
        let reports = detect_all(&runner, &specs(), &overrides);
        let node = reports.iter().find(|r| r.name == "node").unwrap();
        assert_eq!(node.status, Status::BelowMin);
    }
}
