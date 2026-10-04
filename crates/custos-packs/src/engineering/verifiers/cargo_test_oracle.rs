//! Cargo Test Oracle
//!
//! Preferred verifier for Engineering Pack (`cargo_test_oracle`).
//! Runs `cargo test` in an isolated process to independently verify
//! that changes introduce no test regressions before Gate 4 completion.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::process::SandboxedProcess;

use crate::verifier::{PackVerifier, VerificationContext, VerificationResult};

pub struct CargoTestOracle {
    timeout_ms: u64,
}

impl Default for CargoTestOracle {
    fn default() -> Self {
        Self {
            timeout_ms: 120_000, // 2 minutes
        }
    }
}

impl CargoTestOracle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[async_trait]
impl PackVerifier for CargoTestOracle {
    fn verifier_id(&self) -> &'static str {
        "cargo_test_oracle"
    }

    fn description(&self) -> &'static str {
        "Executes 'cargo test' in the target worktree and verifies 0 test failures"
    }

    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        let workspace = match &ctx.workspace_path {
            Some(w) => w.clone(),
            None => {
                return VerificationResult::Unknown {
                    reason: "missing workspace_path in verification context".into(),
                };
            }
        };

        // Extract optional test filter from metadata
        let filter = ctx
            .metadata
            .get("filter")
            .and_then(|f| f.as_str())
            .map(|s| s.to_string());

        let mut proc = SandboxedProcess::new("cargo")
            .arg("test")
            .arg("--")
            .arg("--nocapture")
            .cwd(&workspace)
            .timeout_ms(self.timeout_ms);

        if let Some(f) = filter {
            proc = proc.arg(f);
        }

        match proc.run().await {
            Ok(output) => {
                if output.succeeded() {
                    VerificationResult::Pass {
                        detail: format!(
                            "cargo test passed (duration {}ms): {}",
                            output.elapsed_ms,
                            output.stdout_tail(300)
                        ),
                    }
                } else {
                    VerificationResult::Fail {
                        reason: format!(
                            "cargo test failed with exit code {}: {}",
                            output.exit_code,
                            output.stderr_tail(500)
                        ),
                    }
                }
            }
            Err(e) => VerificationResult::Unknown {
                reason: format!("failed to execute cargo test sandbox: {}", e),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cargo_test_oracle_missing_workspace() {
        let oracle = CargoTestOracle::new();
        let ctx = VerificationContext::new("task_1");
        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Unknown { .. }));
    }
}
