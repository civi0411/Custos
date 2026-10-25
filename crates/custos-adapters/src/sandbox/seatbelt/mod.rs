//! macOS sandbox-exec / Seatbelt profile containment

use async_trait::async_trait;
use custos_core::contracts::sandbox::{verify_permit_binding, SandboxCommand, SandboxPort};
use custos_domain::{digest, new_id, ActionIntent, DomainError, ExecutionReceipt, Permit, ReceiptStatus};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct SeatbeltSandbox {
    workspace_root: PathBuf,
    allow_network: bool,
}

impl SeatbeltSandbox {
    pub fn new(workspace_root: impl Into<PathBuf>, allow_network: bool) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            allow_network,
        }
    }

    /// Generates a Seatbelt S-expression profile strictly scoping execution to workspace.
    pub fn generate_profile(&self) -> String {
        let ws = self.workspace_root.to_string_lossy();
        let net_rule = if self.allow_network {
            "(allow network*)"
        } else {
            "(deny network*)"
        };

        format!(
            r#"(version 1)
(deny default)
(allow process-exec)
(allow process-fork)
(allow sysctl-read)
(allow file-read*)
(allow file-write*
    (subpath "/dev")
    (subpath "/private/tmp")
    (subpath "/tmp")
    (subpath "{ws}"))
{net_rule}
"#
        )
    }

    pub fn wrap_command(&self, program: &str, args: &[String]) -> tokio::process::Command {
        #[cfg(target_os = "macos")]
        {
            if Path::new("/usr/bin/sandbox-exec").exists() {
                let mut cmd = tokio::process::Command::new("/usr/bin/sandbox-exec");
                cmd.arg("-p").arg(self.generate_profile());
                cmd.arg(program);
                cmd.args(args);
                return cmd;
            }
        }

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args);
        cmd
    }
}

impl Default for SeatbeltSandbox {
    fn default() -> Self {
        Self::new(".", false)
    }
}

#[async_trait]
impl SandboxPort for SeatbeltSandbox {
    fn driver_id(&self) -> &str {
        "seatbelt"
    }

    async fn execute(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        verify_permit_binding(permit, intent)?;
        let sandbox_cmd = SandboxCommand::from_intent(intent)?;

        let start = Instant::now();
        let mut cmd = self.wrap_command(&sandbox_cmd.program, &sandbox_cmd.args);
        cmd.current_dir(if sandbox_cmd.cwd == "." {
            self.workspace_root.clone()
        } else {
            PathBuf::from(&sandbox_cmd.cwd)
        });
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let output = cmd.spawn().map_err(|e| {
            DomainError::InvariantViolation(format!("Failed to spawn sandboxed process: {e}"))
        })?.wait_with_output().await.map_err(|e| {
            DomainError::InvariantViolation(format!("Sandbox process execution failed: {e}"))
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let is_failure = !output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let out_json = serde_json::json!({
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": stdout,
            "stderr": stderr,
        });
        let json_bytes = serde_json::to_vec(&out_json).unwrap_or_default();
        let output_digest = format!("sha256:{}", digest(&json_bytes));

        Ok(ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit.id.clone(),
            action_id: intent.id.clone(),
            status: if is_failure { ReceiptStatus::Failure } else { ReceiptStatus::Success },
            output_digest,
            output_data: Some(out_json),
            error_message: if is_failure { Some(stderr) } else { None },
            duration_ms: Some(duration_ms),
            executed_at: chrono::Utc::now(),
            assurance: custos_domain::Assurance::CustosMediated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seatbelt_profile_generation() {
        let sb = SeatbeltSandbox::new("/Users/test/workspace", false);
        let profile = sb.generate_profile();
        assert!(profile.contains("(deny network*)"));
        assert!(profile.contains("/Users/test/workspace"));
        assert!(profile.contains("(deny default)"));

        let sb_net = SeatbeltSandbox::new("/Users/test/workspace", true);
        assert!(sb_net.generate_profile().contains("(allow network*)"));
    }
}
