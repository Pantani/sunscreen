//! Auto-repair recipes for `sunscreen doctor --fix`.
//!
//! The recipes are intentionally small and explicit. They install or update
//! tools that the detector already knows how to verify, then the caller should
//! re-run detection before reporting a tool as fixed.

use crate::runtime::subprocess::{CommandSpec, ProcessRunner};

use super::{CommandRunner, Status, ToolReport};

/// Status of a toolchain repair attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFixStatus {
    /// No action was needed or selected.
    Skipped,
    /// Commands ran successfully and re-detection confirmed the tool.
    Fixed,
    /// Commands ran successfully, but the current process still cannot see the
    /// fixed tool. This usually means PATH needs to be reloaded.
    NeedsShellReload,
    /// Sunscreen does not know a safe automatic recipe for this tool.
    Unsupported,
    /// A repair command failed or could not be spawned.
    Failed,
    /// Internal transient state before the post-fix detector pass.
    Attempted,
}

/// Machine-readable result for one tool repair attempt.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolFixResult {
    pub name: String,
    pub required: bool,
    pub status: ToolFixStatus,
    pub message: String,
    pub commands: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// Progress events emitted while `doctor --fix` runs repair recipes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolFixLogEvent {
    /// A tool is being considered for repair.
    ToolStatus {
        name: String,
        status: Status,
        required: bool,
    },
    /// No command will be run for this tool.
    ToolSkipped { name: String, message: String },
    /// No automatic repair recipe exists for this tool.
    ToolUnsupported { name: String, message: String },
    /// A repair command is about to start.
    CommandStarted { name: String, command: Vec<String> },
    /// A repair command exited successfully.
    CommandSucceeded {
        name: String,
        command: Vec<String>,
        duration_ms: u128,
        stdout: String,
        stderr: String,
    },
    /// A repair command failed or could not be spawned.
    CommandFailed {
        name: String,
        command: Vec<String>,
        message: String,
        exit_code: Option<i32>,
    },
}

/// Execute fix recipes for the provided reports.
#[must_use = "fix results should be reported to users"]
pub fn fix_reports<R, P>(
    detector_runner: &R,
    process_runner: &P,
    reports: &[ToolReport],
    include_optional: bool,
) -> Vec<ToolFixResult>
where
    R: CommandRunner,
    P: ProcessRunner,
{
    fix_reports_with_logger(
        detector_runner,
        process_runner,
        reports,
        include_optional,
        |_| {},
    )
}

/// Execute fix recipes and emit progress events as commands run.
#[must_use = "fix results should be reported to users"]
pub fn fix_reports_with_logger<R, P, F>(
    detector_runner: &R,
    process_runner: &P,
    reports: &[ToolReport],
    include_optional: bool,
    mut log: F,
) -> Vec<ToolFixResult>
where
    R: CommandRunner,
    P: ProcessRunner,
    F: FnMut(ToolFixLogEvent),
{
    let mut ordered = reports.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|report| fix_order(&report.name));

    ordered
        .into_iter()
        .map(|report| {
            fix_one(
                detector_runner,
                process_runner,
                report,
                include_optional,
                &mut log,
            )
        })
        .collect()
}

/// Convert successful command attempts into final statuses after re-detection.
pub fn finalize_fix_results(results: &mut [ToolFixResult], reports_after: &[ToolReport]) {
    for result in results {
        if result.status != ToolFixStatus::Attempted {
            continue;
        }
        let available = reports_after
            .iter()
            .find(|report| report.name == result.name)
            .map(|report| report.available)
            .unwrap_or(false);
        if available {
            result.status = ToolFixStatus::Fixed;
            result.message = "tool is available after repair".to_string();
        } else {
            result.status = ToolFixStatus::NeedsShellReload;
            result.message = reports_after
                .iter()
                .find(|report| report.name == result.name)
                .map_or_else(|| reload_hint(&result.name), post_repair_hint);
        }
    }
}

fn fix_one<R, P, F>(
    detector_runner: &R,
    process_runner: &P,
    report: &ToolReport,
    include_optional: bool,
    log: &mut F,
) -> ToolFixResult
where
    R: CommandRunner,
    P: ProcessRunner,
    F: FnMut(ToolFixLogEvent),
{
    log(ToolFixLogEvent::ToolStatus {
        name: report.name.clone(),
        status: report.status,
        required: report.required,
    });

    if report.available {
        log(ToolFixLogEvent::ToolSkipped {
            name: report.name.clone(),
            message: "already available".to_string(),
        });
        return skipped(report, "already available");
    }

    if !report.required && !include_optional {
        let message =
            "optional tool was not targeted; run with `--component <tool> --fix` to install it"
                .to_string();
        log(ToolFixLogEvent::ToolSkipped {
            name: report.name.clone(),
            message: message.clone(),
        });
        return skipped(report, message);
    }

    let steps = match recipe(detector_runner, report) {
        Ok(steps) => steps,
        Err(message) => {
            log(ToolFixLogEvent::ToolUnsupported {
                name: report.name.clone(),
                message: message.clone(),
            });
            return ToolFixResult {
                name: report.name.clone(),
                required: report.required,
                status: ToolFixStatus::Unsupported,
                message,
                commands: Vec::new(),
                failed_command: None,
                exit_code: None,
            };
        }
    };

    let commands = steps.iter().map(FixStep::argv).collect::<Vec<_>>();
    for step in steps {
        let command = step.argv();
        log(ToolFixLogEvent::CommandStarted {
            name: report.name.clone(),
            command: command.clone(),
        });
        match process_runner.run(step.command_spec()) {
            Ok(output) if output.success() => {
                log(ToolFixLogEvent::CommandSucceeded {
                    name: report.name.clone(),
                    command,
                    duration_ms: output.duration_ms,
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(output) => {
                let message = first_non_empty(&output.stderr, &output.stdout)
                    .unwrap_or_else(|| format!("repair command exited {}", output.exit_code));
                log(ToolFixLogEvent::CommandFailed {
                    name: report.name.clone(),
                    command: command.clone(),
                    message: message.clone(),
                    exit_code: Some(output.exit_code),
                });
                return ToolFixResult {
                    name: report.name.clone(),
                    required: report.required,
                    status: ToolFixStatus::Failed,
                    message,
                    commands,
                    failed_command: Some(command),
                    exit_code: Some(output.exit_code),
                };
            }
            Err(err) => {
                let message = err.to_string();
                log(ToolFixLogEvent::CommandFailed {
                    name: report.name.clone(),
                    command: command.clone(),
                    message: message.clone(),
                    exit_code: None,
                });
                return ToolFixResult {
                    name: report.name.clone(),
                    required: report.required,
                    status: ToolFixStatus::Failed,
                    message,
                    commands,
                    failed_command: Some(command),
                    exit_code: None,
                };
            }
        }
    }

    ToolFixResult {
        name: report.name.clone(),
        required: report.required,
        status: ToolFixStatus::Attempted,
        message: "repair commands completed; verifying tool availability".to_string(),
        commands,
        failed_command: None,
        exit_code: None,
    }
}

fn skipped(report: &ToolReport, message: impl Into<String>) -> ToolFixResult {
    ToolFixResult {
        name: report.name.clone(),
        required: report.required,
        status: ToolFixStatus::Skipped,
        message: message.into(),
        commands: Vec::new(),
        failed_command: None,
        exit_code: None,
    }
}

fn recipe<R: CommandRunner>(runner: &R, report: &ToolReport) -> Result<Vec<FixStep>, String> {
    match report.name.as_str() {
        "anchor" => {
            let mut steps = Vec::new();
            if runner.which("avm").is_none() {
                steps.push(FixStep::new("cargo").args([
                    "install",
                    "--git",
                    "https://github.com/solana-foundation/anchor",
                    "avm",
                    "--force",
                ]));
            }
            steps.push(FixStep::new("avm").args(["install", "latest"]));
            steps.push(FixStep::new("avm").args(["use", "latest"]));
            Ok(steps)
        }
        "solana" => {
            if cfg!(windows) {
                Err("automatic Solana CLI installation is not supported on Windows yet".to_string())
            } else if runner.which("agave-install").is_some() {
                Ok(vec![FixStep::new("agave-install").arg("update")])
            } else {
                Ok(vec![FixStep::shell(
                    r#"sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)""#,
                )])
            }
        }
        "rustc" | "cargo" => {
            if runner.which("rustup").is_some() {
                Ok(vec![FixStep::new("rustup").args(["update", "stable"])])
            } else if cfg!(windows) {
                Err("install Rust with rustup, then run `sunscreen doctor` again".to_string())
            } else {
                Ok(vec![FixStep::shell(
                    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
                )])
            }
        }
        "node" => {
            Err("install Node.js with your OS package manager or version manager".to_string())
        }
        "pnpm" => {
            let version = report
                .min_version
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "latest".to_string());
            let pnpm_spec = format!("pnpm@{version}");
            if runner.which("corepack").is_some() {
                Ok(vec![
                    FixStep::corepack().args(["enable", "pnpm"]),
                    FixStep::corepack().args(["install", "-g", pnpm_spec.as_str()]),
                ])
            } else if runner.which("npm").is_some() {
                Ok(vec![
                    FixStep::new("npm").args(["install", "--global", "corepack@latest"]),
                    FixStep::corepack().args(["enable", "pnpm"]),
                    FixStep::corepack().args(["install", "-g", pnpm_spec.as_str()]),
                ])
            } else {
                Err(
                    "install Node.js/npm first, then run `sunscreen doctor --fix` again"
                        .to_string(),
                )
            }
        }
        "codama" => {
            if runner.which("pnpm").is_some() {
                Ok(vec![
                    FixStep::new("pnpm").args(["add", "--global", "codama"])
                ])
            } else {
                Err(
                    "install pnpm first, then run `sunscreen doctor --component codama --fix`"
                        .to_string(),
                )
            }
        }
        "surfpool" => {
            if cfg!(windows) {
                Err("automatic Surfpool installation is not supported on Windows yet".to_string())
            } else {
                Ok(vec![FixStep::shell(
                    "curl -sL https://run.surfpool.run/ | bash",
                )])
            }
        }
        "rustfmt" => {
            if runner.which("rustup").is_some() {
                Ok(vec![FixStep::new("rustup").args([
                    "component",
                    "add",
                    "rustfmt",
                ])])
            } else {
                Err("install rustup first, then run `rustup component add rustfmt`".to_string())
            }
        }
        _ => Err(format!(
            "no automatic repair recipe is registered for {}",
            report.name
        )),
    }
}

fn fix_order(name: &str) -> usize {
    match name {
        "rustc" => 0,
        "cargo" => 1,
        "node" => 2,
        "pnpm" => 3,
        "solana" => 4,
        "anchor" => 5,
        "rustfmt" => 6,
        "codama" => 7,
        "surfpool" => 8,
        _ => usize::MAX,
    }
}

fn first_non_empty(stderr: &str, stdout: &str) -> Option<String> {
    [stderr.trim(), stdout.trim()]
        .into_iter()
        .find(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn post_repair_hint(report: &ToolReport) -> String {
    match report.status {
        Status::MissingRequired | Status::MissingOptional => reload_hint(&report.name),
        Status::UnknownVersion => format!(
            "repair commands succeeded, but `{}` still reports an unparsable version; run `which {}` and `{} --version` to inspect the binary on PATH",
            report.name, report.name, report.name
        ),
        Status::BelowMin => format!(
            "repair commands succeeded, but `{}` is still below the required version; inspect the installer output and run `sunscreen doctor --component {}` again",
            report.name, report.name
        ),
        Status::Ok => "tool is available after repair".to_string(),
    }
}

fn reload_hint(name: &str) -> String {
    match name {
        "solana" => "repair commands succeeded, but `solana` is still not visible on PATH; open a new shell or run `export PATH=\"$HOME/.local/share/solana/install/active_release/bin:$PATH\"`, then run `sunscreen doctor --component solana` again".to_string(),
        "anchor" | "rustc" | "cargo" | "rustfmt" => format!(
            "repair commands succeeded, but `{name}` is still not visible on PATH; open a new shell or run `. \"$HOME/.cargo/env\"`, then run `sunscreen doctor --component {name}` again"
        ),
        "pnpm" => "repair commands succeeded, but `pnpm` is still not visible on PATH; open a new shell or run `corepack enable pnpm`, then run `sunscreen doctor --component pnpm` again".to_string(),
        other => format!(
            "repair commands succeeded, but `{other}` is still not visible on PATH; reload your shell and run `sunscreen doctor --component {other}` again"
        ),
    }
}

#[derive(Debug, Clone)]
struct FixStep {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl FixStep {
    fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    fn corepack() -> Self {
        Self::new("corepack").env("COREPACK_ENABLE_DOWNLOAD_PROMPT", "0")
    }

    fn shell(command: impl Into<String>) -> Self {
        Self::new("sh").args(["-c".to_string(), command.into()])
    }

    fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    fn argv(&self) -> Vec<String> {
        std::iter::once(self.program.clone())
            .chain(self.args.iter().cloned())
            .collect()
    }

    fn command_spec(self) -> CommandSpec {
        let mut spec = CommandSpec::new(self.program);
        for arg in self.args {
            spec = spec.arg(arg);
        }
        for (key, value) in self.env {
            spec = spec.env(key, value);
        }
        spec
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use semver::Version;

    use crate::runtime::subprocess::{CommandOutput, ProcessError};
    use crate::toolchain::{CommandRunner, Status};

    use super::*;

    #[derive(Default)]
    struct MockDetector {
        paths: HashMap<String, PathBuf>,
    }

    impl CommandRunner for MockDetector {
        fn run(&self, _bin: &str, _args: &[&str]) -> Option<String> {
            None
        }

        fn which(&self, bin: &str) -> Option<PathBuf> {
            self.paths.get(bin).cloned()
        }
    }

    #[derive(Default)]
    struct MockProcess {
        fail: Option<String>,
    }

    impl ProcessRunner for MockProcess {
        fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError> {
            let argv = spec.display_argv();
            if self.fail.as_ref() == argv.first() {
                Ok(CommandOutput {
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: "boom".to_string(),
                    duration_ms: 0,
                })
            } else {
                Ok(CommandOutput {
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                })
            }
        }
    }

    fn report(name: &str, required: bool) -> ToolReport {
        ToolReport {
            name: name.to_string(),
            found: false,
            available: false,
            path: None,
            version: None,
            required,
            min_version: Some(Version::new(9, 0, 0)),
            status: if required {
                Status::MissingRequired
            } else {
                Status::MissingOptional
            },
        }
    }

    #[test]
    fn optional_tools_are_skipped_unless_requested() {
        let result = fix_reports(
            &MockDetector::default(),
            &MockProcess::default(),
            &[report("codama", false)],
            false,
        );

        assert_eq!(result[0].status, ToolFixStatus::Skipped);
        assert!(result[0].commands.is_empty());
    }

    #[test]
    fn targeted_optional_codama_uses_pnpm() {
        let mut detector = MockDetector::default();
        detector
            .paths
            .insert("pnpm".to_string(), PathBuf::from("/bin/pnpm"));
        let result = fix_reports(
            &detector,
            &MockProcess::default(),
            &[report("codama", false)],
            true,
        );

        assert_eq!(result[0].status, ToolFixStatus::Attempted);
        assert_eq!(
            result[0].commands,
            vec![vec!["pnpm", "add", "--global", "codama"]]
        );
    }

    #[test]
    fn failed_step_is_reported() {
        let mut detector = MockDetector::default();
        detector
            .paths
            .insert("pnpm".to_string(), PathBuf::from("/bin/pnpm"));
        let process = MockProcess {
            fail: Some("pnpm".to_string()),
        };
        let result = fix_reports(&detector, &process, &[report("codama", false)], true);

        assert_eq!(result[0].status, ToolFixStatus::Failed);
        let expected = vec![
            "pnpm".to_string(),
            "add".to_string(),
            "--global".to_string(),
            "codama".to_string(),
        ];
        assert_eq!(
            result[0].failed_command.as_deref(),
            Some(expected.as_slice())
        );
        assert_eq!(result[0].exit_code, Some(1));
    }
}
