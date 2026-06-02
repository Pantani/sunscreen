//! Preflight checks executed before destructive scaffolding actions.
//!
//! `chain new` (and similar) call into [`preflight_chain_new`] to confirm
//! that the *minimum* set of tools required to render and validate a fresh
//! Anchor or Pinocchio workspace is present and at acceptable versions. This is a
//! tighter subset than `sunscreen doctor`, which surfaces every known
//! tool. The contract:
//!
//! - **Hard fail** if any required tool is missing, below its minimum, or
//!   yields an unparsable version.
//! - **Warn only** (returned via [`PreflightReport::warnings`]) when a
//!   detected major version diverges from the version pinned in
//!   `sunscreen.yml` (`project.anchor_version` / `project.solana_version`).
//!
//! The function is pure with respect to the injected [`CommandRunner`]
//! and therefore deterministically testable.

use std::collections::BTreeMap;

use semver::Version;
use thiserror::Error;

use super::detect::{detect_all, CommandRunner, Status, ToolReport};
use super::registry::{known, ToolSpec};
use crate::config::{Config, Framework};

/// Frontend selection forwarded by `chain new`. Determines whether
/// node/pnpm need to be present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frontend {
    /// No JS frontend — node/pnpm are not preflight-required.
    None,
    /// A frontend that depends on the JS toolchain.
    Js,
}

/// Errors surfaced by preflight. Each variant maps cleanly onto the
/// public CLI error taxonomy (typically `SunscreenError::ToolchainMissing`).
#[derive(Debug, Error)]
pub enum PreflightError {
    /// One or more required tools failed the gate.
    #[error("preflight failed: {0}")]
    Failed(String),
}

/// Successful preflight may still carry non-fatal version-drift warnings.
#[derive(Debug, Default, Clone)]
pub struct PreflightReport {
    /// Reports for every probed tool (in probe order).
    pub reports: Vec<ToolReport>,
    /// Non-fatal warnings (e.g. major-version divergence from `sunscreen.yml`).
    pub warnings: Vec<String>,
}

/// Convenience wrapper using [`super::detect::RealRunner`] and the
/// provided config. Anchor workspaces require `rustc`, `cargo`, `anchor`,
/// and `solana`. Pinocchio workspaces require `rustc`, `cargo`, and
/// `solana`; `cargo build-sbf` is supplied by the Solana CLI toolchain.
/// Frontends additionally require `node` and `pnpm`.
///
/// Returns [`PreflightReport`] on success (warnings may still be present)
/// or [`PreflightError::Failed`] on hard failure.
pub fn preflight_chain_new(
    config: &Config,
    frontend: Frontend,
) -> Result<PreflightReport, PreflightError> {
    preflight_chain_new_with(&super::detect::RealRunner, config, frontend)
}

/// Testable variant taking an injected [`CommandRunner`].
pub fn preflight_chain_new_with<R: CommandRunner>(
    runner: &R,
    config: &Config,
    frontend: Frontend,
) -> Result<PreflightReport, PreflightError> {
    let specs = required_specs(config.project.framework, frontend);
    let reports = detect_all(runner, &specs, &config.toolchain.required);

    let mut failures: Vec<String> = Vec::new();
    for r in &reports {
        match r.status {
            Status::Ok | Status::MissingOptional => {}
            Status::MissingRequired => failures.push(format!("{} missing", r.name)),
            Status::BelowMin => {
                let v = r
                    .version
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "?".into());
                let m = r
                    .min_version
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "?".into());
                failures.push(format!("{} {} below required {}", r.name, v, m));
            }
            Status::UnknownVersion => failures.push(format!("{} version unparsable", r.name)),
        }
    }

    if !failures.is_empty() {
        return Err(PreflightError::Failed(failures.join("; ")));
    }

    let warnings = collect_version_warnings(&reports, config);
    Ok(PreflightReport { reports, warnings })
}

/// Return the subset of [`known`] tools required for `chain new`.
fn required_specs(framework: Framework, frontend: Frontend) -> Vec<ToolSpec> {
    let needs_js = matches!(frontend, Frontend::Js);
    known()
        .into_iter()
        .filter(|s| match s.name {
            "rustc" | "cargo" | "solana" => true,
            "anchor" => matches!(framework, Framework::Anchor),
            "node" | "pnpm" => needs_js,
            _ => false,
        })
        .collect()
}

/// Compare detected versions against `project.anchor_version` /
/// `project.solana_version`. Emit a warning when the **major** component
/// diverges (minor/patch drift is tolerated silently).
fn collect_version_warnings(reports: &[ToolReport], config: &Config) -> Vec<String> {
    let mut warns = Vec::new();
    let pins: BTreeMap<&str, Option<&String>> = BTreeMap::from([
        ("anchor", config.project.anchor_version.as_ref()),
        ("solana", config.project.solana_version.as_ref()),
    ]);

    for r in reports {
        let Some(Some(pinned_raw)) = pins.get(r.name.as_str()).copied() else {
            continue;
        };
        let Ok(pinned) = Version::parse(pinned_raw) else {
            warns.push(format!(
                "config project.{}_version = {:?} is not a valid semver; ignoring",
                r.name, pinned_raw
            ));
            continue;
        };
        let Some(detected) = r.version.as_ref() else {
            continue;
        };
        if detected.major != pinned.major {
            warns.push(format!(
                "{} major version mismatch: detected {} but sunscreen.yml pins {}",
                r.name, detected, pinned
            ));
        }
    }
    warns
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::detect::CommandRunner;
    use std::collections::HashMap;
    use std::path::PathBuf;

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

    fn happy_runner() -> MockRunner {
        let mut responses = HashMap::new();
        let mut paths = HashMap::new();
        for bin in ["anchor", "solana", "rustc", "cargo", "node", "pnpm"] {
            paths.insert(bin.into(), PathBuf::from(format!("/usr/bin/{bin}")));
        }
        responses.insert("anchor".into(), "anchor-cli 1.5.0".into());
        responses.insert("solana".into(), "solana-cli 2.1.0 (src:abc)".into());
        responses.insert("rustc".into(), "rustc 1.80.0 (abc 2024-01-01)".into());
        responses.insert("cargo".into(), "cargo 1.80.0".into());
        responses.insert("node".into(), "v20.10.0".into());
        responses.insert("pnpm".into(), "9.4.0".into());
        MockRunner { responses, paths }
    }

    #[test]
    fn ok_with_no_frontend_skips_js_tools() {
        let mut runner = happy_runner();
        runner.paths.remove("node");
        runner.paths.remove("pnpm");
        runner.responses.remove("node");
        runner.responses.remove("pnpm");
        let cfg = Config::default();
        let report = preflight_chain_new_with(&runner, &cfg, Frontend::None)
            .expect("preflight should pass without node/pnpm when frontend=None");
        let names: Vec<_> = report.reports.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"anchor"));
        assert!(!names.contains(&"node"));
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn pinocchio_skips_anchor_preflight() {
        let mut runner = happy_runner();
        runner.paths.remove("anchor");
        runner.responses.remove("anchor");
        let mut cfg = Config::default();
        cfg.project.framework = Framework::Pinocchio;
        let report = preflight_chain_new_with(&runner, &cfg, Frontend::None)
            .expect("pinocchio preflight should not require anchor");
        let names: Vec<_> = report.reports.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"rustc"));
        assert!(names.contains(&"cargo"));
        assert!(names.contains(&"solana"));
        assert!(!names.contains(&"anchor"));
    }

    #[test]
    fn js_frontend_requires_node() {
        let mut runner = happy_runner();
        runner.paths.remove("node");
        runner.responses.remove("node");
        let cfg = Config::default();
        let err = preflight_chain_new_with(&runner, &cfg, Frontend::Js)
            .expect_err("missing node should fail JS preflight");
        assert!(err.to_string().contains("node"));
    }

    #[test]
    fn missing_anchor_fails() {
        let mut runner = happy_runner();
        runner.paths.remove("anchor");
        runner.responses.remove("anchor");
        let cfg = Config::default();
        let err = preflight_chain_new_with(&runner, &cfg, Frontend::None).unwrap_err();
        assert!(err.to_string().contains("anchor"));
    }

    #[test]
    fn warns_on_major_anchor_drift() {
        let runner = happy_runner();
        let mut cfg = Config::default();
        cfg.project.anchor_version = Some("2.0.0".into());
        let report = preflight_chain_new_with(&runner, &cfg, Frontend::None).unwrap();
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("anchor") && w.contains("major")),
            "expected major-drift warning, got {:?}",
            report.warnings
        );
    }

    #[test]
    fn no_warn_on_minor_drift() {
        let runner = happy_runner();
        let mut cfg = Config::default();
        cfg.project.anchor_version = Some("1.2.0".into()); // detected 1.5.0
        cfg.project.solana_version = Some("2.0.5".into()); // detected 2.1.0
        let report = preflight_chain_new_with(&runner, &cfg, Frontend::None).unwrap();
        assert!(
            report.warnings.is_empty(),
            "minor drift should not warn: {:?}",
            report.warnings
        );
    }

    #[test]
    fn invalid_pin_warns_but_doesnt_fail() {
        let runner = happy_runner();
        let mut cfg = Config::default();
        cfg.project.solana_version = Some("not-a-version".into());
        let report = preflight_chain_new_with(&runner, &cfg, Frontend::None).unwrap();
        assert!(report.warnings.iter().any(|w| w.contains("not a valid")));
    }
}
