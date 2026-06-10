//! Friendly deployment wrapper.

use crate::error::SunscreenError;
use crate::onboarding::args::{ClusterArg, DeployArgs};
use crate::process::{CommandOutput, CommandSpec, ProcessError, ProcessRunner, SubprocessRunner};
use crate::workspace;

pub fn run(args: &DeployArgs, json: bool) -> Result<i32, SunscreenError> {
    run_with_runner(args, json, &SubprocessRunner)
}

fn run_with_runner<R: ProcessRunner>(
    args: &DeployArgs,
    json: bool,
    runner: &R,
) -> Result<i32, SunscreenError> {
    if args.target == ClusterArg::Mainnet && !args.yes_i_understand_cost {
        return Err(SunscreenError::UserInput(
            "mainnet deploy requires --yes-i-understand-cost".into(),
        ));
    }
    if args.verify && args.program.is_none() {
        return Err(SunscreenError::UserInput(
            "--verify requires --program <name>".into(),
        ));
    }
    let ws = workspace::find_root(None)?;
    let deploy = deploy_spec(args).cwd(&ws.root);
    if args.dry_run {
        emit_plan(json, args, &deploy);
        return Ok(0);
    }

    let deploy_output = runner.run(deploy).map_err(map_process_missing("anchor"))?;
    if !deploy_output.success() {
        return Err(SunscreenError::Network(format!(
            "deploy failed with exit {}: {}",
            deploy_output.exit_code, deploy_output.stderr
        )));
    }
    let verify_output = if args.verify {
        let verify = verify_spec(args).cwd(&ws.root);
        let output = runner.run(verify).map_err(map_process_missing("anchor"))?;
        if !output.success() {
            return Err(SunscreenError::Network(format!(
                "verify failed with exit {}: {}",
                output.exit_code, output.stderr
            )));
        }
        Some(output)
    } else {
        None
    };
    emit_result(json, args, &deploy_output, verify_output.as_ref());
    Ok(0)
}

fn deploy_spec(args: &DeployArgs) -> CommandSpec {
    let mut spec = CommandSpec::new("anchor")
        .arg("deploy")
        .arg("--provider.cluster")
        .arg(cluster_url(args.target));
    if let Some(program) = &args.program {
        spec = spec.arg("--program-name").arg(program);
    }
    spec
}

fn verify_spec(args: &DeployArgs) -> CommandSpec {
    CommandSpec::new("anchor")
        .arg("verify")
        .arg(args.program.as_deref().unwrap_or_default())
        .arg("--provider.cluster")
        .arg(cluster_url(args.target))
}

fn cluster_url(cluster: ClusterArg) -> &'static str {
    match cluster {
        ClusterArg::Localnet => "localnet",
        ClusterArg::Devnet => "devnet",
        ClusterArg::Mainnet => "mainnet",
    }
}

fn emit_plan(json: bool, args: &DeployArgs, spec: &CommandSpec) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "deploy",
                "target": args.target.as_str(),
                "program": args.program,
                "verify": args.verify,
                "dry_run": true,
                "argv": spec.display_argv(),
            })
        );
    } else {
        println!("dry-run: {}", spec.display_argv().join(" "));
    }
}

fn emit_result(
    json: bool,
    args: &DeployArgs,
    deploy: &CommandOutput,
    verify: Option<&CommandOutput>,
) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "deploy",
                "target": args.target.as_str(),
                "program": args.program,
                "verify": args.verify,
                "deploy": {
                    "stdout": deploy.stdout,
                    "stderr": deploy.stderr,
                },
                "verify_output": verify.map(|output| serde_json::json!({
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                })),
            })
        );
    } else {
        print!("{}", deploy.stdout);
        if let Some(verify) = verify {
            print!("{}", verify.stdout);
        }
        println!("deploy {}: ok", args.target.as_str());
    }
}

fn map_process_missing(tool: &'static str) -> impl FnOnce(ProcessError) -> SunscreenError {
    move |err| {
        if err.is_not_found() {
            SunscreenError::ToolchainMissing(format!(
                "{tool} not found on PATH; install Anchor before deploying"
            ))
        } else {
            SunscreenError::Other(anyhow::anyhow!("{tool}: {err}"))
        }
    }
}
