//! Stdio JSON-RPC transport using LSP-style `Content-Length` framing.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use serde_json::json;

use crate::error::SunscreenError;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[derive(Debug, Clone)]
pub struct StdioInvocation<'a> {
    pub plugin_name: &'a str,
    pub method: &'a str,
    pub command_kind: &'a str,
    pub command_name: &'a str,
    pub args: &'a [String],
    pub workspace_root: &'a Path,
    pub scratch_dir: &'a Path,
}

#[derive(Debug, Clone)]
pub struct StdioReport {
    pub result: serde_json::Value,
    pub duration_ms: u128,
}

pub fn run(
    plugin_dir: &Path,
    entrypoint: &[String],
    invocation: StdioInvocation<'_>,
) -> Result<StdioReport, SunscreenError> {
    let Some(program) = entrypoint.first() else {
        return Err(SunscreenError::PluginRuntime(format!(
            "plugin {} has no entrypoint",
            invocation.plugin_name
        )));
    };
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": invocation.method,
        "params": {
            "plugin": invocation.plugin_name,
            "kind": invocation.command_kind,
            "command": invocation.command_name,
            "args": invocation.args,
            "workspace": invocation.workspace_root,
            "scratch": invocation.scratch_dir,
        }
    });
    let body = serde_json::to_string(&request)
        .map_err(|err| SunscreenError::PluginRuntime(format!("serialize plugin request: {err}")))?;
    let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);

    let started = Instant::now();
    let mut command = Command::new(program);
    command
        .args(entrypoint.iter().skip(1))
        .current_dir(plugin_dir)
        .env_clear()
        .env(
            "PATH",
            std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()),
        )
        .env("NO_COLOR", "1")
        .env("SUNSCREEN_WORKSPACE_ROOT", invocation.workspace_root)
        .env("SUNSCREEN_PLUGIN_SCRATCH", invocation.scratch_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|err| {
        SunscreenError::PluginRuntime(format!(
            "start plugin {} ({program}): {err}",
            invocation.plugin_name
        ))
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(frame.as_bytes()).map_err(|err| {
            SunscreenError::PluginRuntime(format!(
                "write request to plugin {}: {err}",
                invocation.plugin_name
            ))
        })?;
    }
    let output = child.wait_with_output().map_err(|err| {
        SunscreenError::PluginRuntime(format!("wait for plugin {}: {err}", invocation.plugin_name))
    })?;
    let duration_ms = started.elapsed().as_millis();
    if !output.status.success() {
        return Err(SunscreenError::PluginRuntime(format!(
            "plugin {} exited with code {:?}: {}",
            invocation.plugin_name,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let response = parse_response(&stdout)?;
    if response.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Err(SunscreenError::PluginRuntime(format!(
            "plugin {} returned a non JSON-RPC response",
            invocation.plugin_name
        )));
    }
    if let Some(error) = response.get("error") {
        return Err(SunscreenError::PluginRuntime(format!(
            "plugin {} returned error: {error}",
            invocation.plugin_name
        )));
    }
    let result = response.get("result").cloned().ok_or_else(|| {
        SunscreenError::PluginRuntime(format!(
            "plugin {} response is missing `result`",
            invocation.plugin_name
        ))
    })?;
    Ok(StdioReport {
        result,
        duration_ms,
    })
}

fn parse_response(stdout: &str) -> Result<serde_json::Value, SunscreenError> {
    if stdout.trim_start().starts_with('{') {
        return serde_json::from_str(stdout.trim()).map_err(|err| {
            SunscreenError::PluginRuntime(format!("parse plugin JSON response: {err}"))
        });
    }

    let Some((header, body)) = stdout.split_once("\r\n\r\n") else {
        return Err(SunscreenError::PluginRuntime(
            "plugin response is missing Content-Length framing".to_string(),
        ));
    };
    let mut content_length = None;
    for line in header.lines() {
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().map_err(|err| {
                SunscreenError::PluginRuntime(format!("invalid plugin Content-Length: {err}"))
            })?);
        }
    }
    let Some(expected_len) = content_length else {
        return Err(SunscreenError::PluginRuntime(
            "plugin response is missing Content-Length".to_string(),
        ));
    };
    let actual_len = body.len();
    if actual_len < expected_len {
        return Err(SunscreenError::PluginRuntime(format!(
            "plugin response body is truncated: expected {expected_len} bytes, got {actual_len}"
        )));
    }
    let json_body = &body[..expected_len];
    serde_json::from_str(json_body).map_err(|err| {
        SunscreenError::PluginRuntime(format!("parse plugin JSON-RPC response: {err}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_accepts_content_length_frame() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let parsed = parse_response(&frame).expect("parse frame");
        assert_eq!(parsed["result"]["ok"], true);
    }

    #[test]
    fn parse_response_rejects_missing_frame() {
        let err = parse_response("hello").expect_err("missing frame should fail");
        assert_eq!(err.kind_str(), "plugin_runtime");
    }
}
