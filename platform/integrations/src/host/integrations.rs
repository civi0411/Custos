//! Host drivers for browser CLI (`agent-browser`), headless shell, and remote SSH execution.

use base64::Engine as _;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;

pub struct AgentBrowserOutput {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// Runs `agent-browser --session <session> <args...>` as a bounded subprocess.
pub async fn run_agent_browser(
    agent_bin: &Path,
    session: &str,
    args: &[&str],
) -> Result<AgentBrowserOutput, std::io::Error> {
    let mut cmd = Command::new(agent_bin);
    cmd.arg("--session").arg(session).args(args);
    cmd.kill_on_drop(true);
    let out = cmd.output().await?;
    Ok(AgentBrowserOutput {
        success: out.status.success(),
        stdout: out.stdout,
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// Runs `agent-browser --session <session> --json <args...>` and parses JSON response.
pub async fn run_agent_browser_json(
    agent_bin: &Path,
    session: &str,
    args: &[&str],
) -> Result<serde_json::Value, String> {
    let mut full_args = Vec::with_capacity(args.len() + 1);
    full_args.push("--json");
    full_args.extend_from_slice(args);
    let out = run_agent_browser(agent_bin, session, &full_args)
        .await
        .map_err(|e| format!("Failed to spawn agent-browser: {e}"))?;
    if !out.success {
        return Err(out.stderr.trim().to_string());
    }
    serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .map_err(|e| format!("Invalid JSON from agent-browser: {e}"))
}

/// Reads a screenshot file from disk and encodes it as a base64 PNG data URL.
pub fn read_screenshot_base64(path: &str) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        )
    })
}

fn build_ssh_command(
    ssh_bin: &Path,
    host: &str,
    user: &str,
    port: u16,
    connect_timeout_secs: u64,
    key_path: Option<&str>,
    remote_cmd: &str,
) -> Command {
    let mut cmd = Command::new(ssh_bin);
    cmd.kill_on_drop(true)
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg(format!("ConnectTimeout={connect_timeout_secs}"))
        .arg("-o")
        .arg("StrictHostKeyChecking=yes")
        .arg("-p")
        .arg(port.to_string());
    if let Some(key) = key_path {
        cmd.arg("-i").arg(key);
    }
    cmd.arg(format!("{user}@{host}")).arg(remote_cmd);
    cmd
}

/// Probes SSH host reachability, returning round-trip latency in milliseconds.
pub async fn ping_ssh_host(
    ssh_bin: &Path,
    host: &str,
    user: &str,
    port: u16,
    key_path: Option<&str>,
) -> Result<u64, String> {
    let start = Instant::now();
    let mut cmd = build_ssh_command(ssh_bin, host, user, port, 3, key_path, "echo -n ping");
    let out = cmd.output().await.map_err(|e| e.to_string())?;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    if out.status.success() {
        Ok(elapsed_ms)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Runs a bounded remote command over SSH, returning `(exit_code, stdout, stderr, duration_ms)`.
pub async fn exec_ssh_command(
    ssh_bin: &Path,
    host: &str,
    user: &str,
    port: u16,
    key_path: Option<&str>,
    command: &str,
) -> Result<(Option<i32>, String, String, u64), std::io::Error> {
    let start = Instant::now();
    let mut cmd = build_ssh_command(ssh_bin, host, user, port, 10, key_path, command);
    let out = tokio::time::timeout(Duration::from_secs(60), cmd.output())
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "SSH command timed out"))??;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    Ok((
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        elapsed_ms,
    ))
}

