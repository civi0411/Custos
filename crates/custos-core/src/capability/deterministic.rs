use super::traits::{ExecutionResult, ToolGate};
use crate::authority::AuthorityEngine;
use crate::sandbox::PathSandbox;
use async_trait::async_trait;
use custos_core_domain::{
    digest, new_id, Action, DomainError, ExecutionReceipt, ReceiptStatus, VerificationClaim,
};
use std::path::PathBuf;
use std::sync::Arc;

pub struct DeterministicGate {
    pub authority: Arc<AuthorityEngine>,
    pub sandbox: PathSandbox,
}

impl DeterministicGate {
    pub fn new(authority: Arc<AuthorityEngine>, workspace_root: PathBuf) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self { authority, sandbox }
    }

    pub fn with_default_authority() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let sandbox = PathSandbox::new(current_dir).expect("Valid current directory");
        Self {
            authority: Arc::new(AuthorityEngine::default()),
            sandbox,
        }
    }

    pub fn with_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self {
            authority: Arc::new(AuthorityEngine::default()),
            sandbox,
        }
    }

    pub fn with_authority_and_workspace(
        authority: Arc<AuthorityEngine>,
        workspace_root: impl Into<PathBuf>,
    ) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self { authority, sandbox }
    }

    /// Creates a verified or unverified VerificationClaim directly from an ExecutionResult
    pub fn create_verification_claim(
        &self,
        task_id: &str,
        verifier_id: &str,
        claim_statement: &str,
        result: &ExecutionResult,
    ) -> VerificationClaim {
        VerificationClaim::new(
            task_id.to_string(),
            claim_statement.to_string(),
            verifier_id.to_string(),
            result.success,
            serde_json::json!({
                "output": result.output,
                "permit_id": result.permit_id,
                "evidence": result.evidence,
                "receipt": result.receipt,
            }),
        )
    }

    async fn execute_read_file(
        &self,
        action: &Action,
        permit_id: &str,
    ) -> Result<ExecutionResult, DomainError> {
        let target_str = action
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&action.target);

        // Enforce sandbox containment
        let resolved_path = self.sandbox.resolve_and_contain(target_str)?;

        if !resolved_path.exists() || !resolved_path.is_file() {
            let output = serde_json::json!({
                "status": "failed",
                "action": "read_file",
                "target": target_str,
                "error": format!("File not found or not a regular file: {}", resolved_path.display()),
            });

            let receipt = ExecutionReceipt {
                receipt_id: new_id("rcpt"),
                permit_id: permit_id.to_string(),
                action_id: action.id.clone(),
                status: ReceiptStatus::Failure,
                output_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                output_data: Some(output.clone()),
                error_message: Some(format!("File not found: {}", resolved_path.display())),
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
            };

            return Ok(ExecutionResult {
                success: false,
                output,
                permit_id: Some(permit_id.to_string()),
                evidence: None,
                receipt: Some(receipt),
            });
        }

        let bytes = match std::fs::read(&resolved_path) {
            Ok(b) => b,
            Err(e) => {
                let output = serde_json::json!({
                    "status": "failed",
                    "action": "read_file",
                    "target": target_str,
                    "error": format!("Failed to read file {}: {e}", resolved_path.display()),
                });

                let receipt = ExecutionReceipt {
                    receipt_id: new_id("rcpt"),
                    permit_id: permit_id.to_string(),
                    action_id: action.id.clone(),
                    status: ReceiptStatus::Failure,
                    output_digest:
                        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                            .to_string(),
                    output_data: Some(output.clone()),
                    error_message: Some(format!("Failed to read file: {e}")),
                    duration_ms: Some(1),
                    executed_at: chrono::Utc::now(),
                };

                return Ok(ExecutionResult {
                    success: false,
                    output,
                    permit_id: Some(permit_id.to_string()),
                    evidence: None,
                    receipt: Some(receipt),
                });
            }
        };

        let digest_val = format!("sha256:{}", digest(&bytes));
        let content_str = String::from_utf8_lossy(&bytes).to_string();

        let output = serde_json::json!({
            "status": "executed",
            "action": "read_file",
            "target": target_str,
            "bytes_read": bytes.len(),
            "digest": digest_val,
            "content": content_str,
        });

        let receipt = ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit_id.to_string(),
            action_id: action.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: digest_val.clone(),
            output_data: Some(output.clone()),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
        };

        Ok(ExecutionResult {
            success: true,
            output,
            permit_id: Some(permit_id.to_string()),
            evidence: Some(digest_val),
            receipt: Some(receipt),
        })
    }

    async fn execute_list_files(
        &self,
        action: &Action,
        permit_id: &str,
    ) -> Result<ExecutionResult, DomainError> {
        let target_str = action
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(if action.target.is_empty() {
                "."
            } else {
                &action.target
            });

        // Enforce sandbox containment
        let resolved_path = self.sandbox.resolve_and_contain(target_str)?;

        if !resolved_path.exists() || !resolved_path.is_dir() {
            let output = serde_json::json!({
                "status": "failed",
                "action": "list_files",
                "target": target_str,
                "error": format!("Directory not found: {}", resolved_path.display()),
            });

            let receipt = ExecutionReceipt {
                receipt_id: new_id("rcpt"),
                permit_id: permit_id.to_string(),
                action_id: action.id.clone(),
                status: ReceiptStatus::Failure,
                output_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                output_data: Some(output.clone()),
                error_message: Some(format!("Directory not found: {}", resolved_path.display())),
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
            };

            return Ok(ExecutionResult {
                success: false,
                output,
                permit_id: Some(permit_id.to_string()),
                evidence: None,
                receipt: Some(receipt),
            });
        }

        let mut entries = Vec::new();
        let read_dir = match std::fs::read_dir(&resolved_path) {
            Ok(rd) => rd,
            Err(e) => {
                let output = serde_json::json!({
                    "status": "failed",
                    "action": "list_files",
                    "target": target_str,
                    "error": format!("Failed to read directory {}: {e}", resolved_path.display()),
                });

                let receipt = ExecutionReceipt {
                    receipt_id: new_id("rcpt"),
                    permit_id: permit_id.to_string(),
                    action_id: action.id.clone(),
                    status: ReceiptStatus::Failure,
                    output_digest:
                        "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                            .to_string(),
                    output_data: Some(output.clone()),
                    error_message: Some(format!("Failed to read directory: {e}")),
                    duration_ms: Some(1),
                    executed_at: chrono::Utc::now(),
                };

                return Ok(ExecutionResult {
                    success: false,
                    output,
                    permit_id: Some(permit_id.to_string()),
                    evidence: None,
                    receipt: Some(receipt),
                });
            }
        };

        for entry in read_dir.flatten() {
            entries.push(entry.file_name().to_string_lossy().to_string());
        }
        entries.sort();

        let listing_json = serde_json::json!({
            "target": target_str,
            "entries": entries,
            "count": entries.len(),
        });
        let digest_val = format!("sha256:{}", digest(listing_json.to_string().as_bytes()));

        let output = serde_json::json!({
            "status": "executed",
            "action": "list_files",
            "target": target_str,
            "entries": listing_json["entries"],
            "count": listing_json["count"],
            "digest": digest_val,
        });

        let receipt = ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit_id.to_string(),
            action_id: action.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: digest_val.clone(),
            output_data: Some(output.clone()),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
        };

        Ok(ExecutionResult {
            success: true,
            output,
            permit_id: Some(permit_id.to_string()),
            evidence: Some(digest_val),
            receipt: Some(receipt),
        })
    }

    async fn execute_patch_preview(
        &self,
        action: &Action,
        permit_id: &str,
    ) -> Result<ExecutionResult, DomainError> {
        let target_str = action
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&action.target);

        // Enforce sandbox containment
        let resolved_path = self.sandbox.resolve_and_contain(target_str)?;

        let patch_content = action
            .parameters
            .get("patch")
            .or_else(|| action.parameters.get("diff"))
            .or_else(|| action.parameters.get("replacement"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Simulated preview: Read existing file if present, but do NOT modify disk
        let original_content = if resolved_path.exists() && resolved_path.is_file() {
            std::fs::read_to_string(&resolved_path).unwrap_or_default()
        } else {
            String::new()
        };

        let diff_preview = format!(
            "--- a/{target_str}\n+++ b/{target_str}\n@@ -1,{} +1,{} @@ (simulated preview)\n{patch_content}",
            original_content.lines().count().max(1),
            patch_content.lines().count().max(1)
        );

        let diff_digest = format!("sha256:{}", digest(diff_preview.as_bytes()));

        let output = serde_json::json!({
            "status": "executed",
            "action": "patch_preview",
            "target": target_str,
            "simulated": true,
            "preview": diff_preview,
            "diff_digest": diff_digest,
            "original_bytes": original_content.len(),
        });

        let receipt = ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit_id.to_string(),
            action_id: action.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: diff_digest.clone(),
            output_data: Some(output.clone()),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
        };

        Ok(ExecutionResult {
            success: true,
            output,
            permit_id: Some(permit_id.to_string()),
            evidence: Some(diff_digest),
            receipt: Some(receipt),
        })
    }

    async fn execute_generic(
        &self,
        action: &Action,
        permit_id: &str,
    ) -> Result<ExecutionResult, DomainError> {
        let result_json = serde_json::json!({
            "status": "executed",
            "action": action.name,
            "target": action.target,
            "permit_id": permit_id,
        });

        let evidence_hash = format!("sha256:{}", digest(result_json.to_string().as_bytes()));

        let receipt = ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit_id.to_string(),
            action_id: action.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: evidence_hash.clone(),
            output_data: Some(result_json.clone()),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
        };

        Ok(ExecutionResult {
            success: true,
            output: result_json,
            permit_id: Some(permit_id.to_string()),
            evidence: Some(evidence_hash),
            receipt: Some(receipt),
        })
    }
}

impl Default for DeterministicGate {
    fn default() -> Self {
        Self::with_default_authority()
    }
}

#[async_trait]
impl ToolGate for DeterministicGate {
    async fn dispatch(&self, action: &Action) -> Result<ExecutionResult, DomainError> {
        self.dispatch_for_task("task_default", action, "agent_executor")
            .await
    }

    async fn dispatch_for_task(
        &self,
        task_id: &str,
        action: &Action,
        actor: &str,
    ) -> Result<ExecutionResult, DomainError> {
        // Step 1: Evaluate policy, classify risk, and issue permit via AuthorityEngine
        let permit = self
            .authority
            .authorize_action(task_id, action, actor)
            .await?;

        // Step 2: Dispatch action under sandbox containment
        match action.name.as_str() {
            "read_file" => self.execute_read_file(action, &permit.id).await,
            "list_files" => self.execute_list_files(action, &permit.id).await,
            "patch_preview" => self.execute_patch_preview(action, &permit.id).await,
            _ => self.execute_generic(action, &permit.id).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core_domain::RiskLevel;

    #[tokio::test]
    async fn test_low_risk_read_file_allowed_with_permit_sandbox_and_receipt() {
        let temp_dir = std::env::temp_dir().join(format!("custos_gate_test_{}", new_id("test")));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("sample.txt");
        std::fs::write(&file_path, "Hello Custos Sovereign!").unwrap();

        let gate = DeterministicGate::with_workspace(&temp_dir);
        let action = Action::new(
            "act_read_1".into(),
            "read_file".into(),
            "sample.txt".into(),
            serde_json::json!({"path": "sample.txt"}),
            RiskLevel::Low,
        );

        let res = gate
            .dispatch_for_task("task_test_1", &action, "developer")
            .await
            .expect("Low-risk read_file must be authorized and executed");

        assert!(res.success);
        assert!(res.permit_id.is_some());
        assert!(res.evidence.is_some());
        assert!(res.receipt.is_some());
        let receipt = res.receipt.unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Success);
        assert!(receipt.output_digest.starts_with("sha256:"));
        assert_eq!(res.output["content"], "Hello Custos Sovereign!");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_low_risk_patch_preview_simulated_without_disk_mutation() {
        let temp_dir = std::env::temp_dir().join(format!("custos_patch_test_{}", new_id("test")));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("code.rs");
        let initial_content = "fn main() { println!(\"old\"); }";
        std::fs::write(&file_path, initial_content).unwrap();

        let gate = DeterministicGate::with_workspace(&temp_dir);
        let action = Action::new(
            "act_patch_1".into(),
            "patch_preview".into(),
            "code.rs".into(),
            serde_json::json!({
                "path": "code.rs",
                "patch": "+ fn main() { println!(\"new\"); }"
            }),
            RiskLevel::Low,
        );

        let res = gate
            .dispatch_for_task("task_patch_task", &action, "developer")
            .await
            .expect("Patch preview must be authorized");

        assert!(res.success);
        assert_eq!(res.output["simulated"], true);
        assert!(res.output["preview"]
            .as_str()
            .unwrap()
            .contains("+ fn main()"));
        assert!(res.evidence.is_some());

        // CRITICAL CHECK: Verify zero disk mutations occurred!
        let disk_content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(disk_content, initial_content);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_sandbox_path_traversal_denied() {
        let temp_dir = std::env::temp_dir().join(format!("custos_trav_test_{}", new_id("test")));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let gate = DeterministicGate::with_workspace(&temp_dir);
        let action = Action::new(
            "act_trav_1".into(),
            "read_file".into(),
            "../../etc/passwd".into(),
            serde_json::json!({"path": "../../etc/passwd"}),
            RiskLevel::Low,
        );

        let err = gate
            .dispatch_for_task("task_trav", &action, "developer")
            .await
            .unwrap_err();

        match err {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("Sandbox containment violation"));
            }
            other => panic!("Expected Unauthorized sandbox error, got: {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_uncertain_outcome_missing_file_fails_safely() {
        let temp_dir = std::env::temp_dir().join(format!("custos_miss_test_{}", new_id("test")));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let gate = DeterministicGate::with_workspace(&temp_dir);
        let action = Action::new(
            "act_miss_1".into(),
            "read_file".into(),
            "non_existent.txt".into(),
            serde_json::json!({"path": "non_existent.txt"}),
            RiskLevel::Low,
        );

        // Execution succeeds at the gate level (it has a valid permit and stayed in sandbox),
        // but returns success: false with a Failure receipt!
        let res = gate
            .dispatch_for_task("task_miss", &action, "developer")
            .await
            .expect("Gate dispatch completes with failure outcome");

        assert!(!res.success);
        assert!(res.evidence.is_none());
        assert_eq!(res.output["status"], "failed");
        let receipt = res.receipt.unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Failure);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_high_risk_action_requires_approval() {
        let gate = DeterministicGate::default();
        let action = Action::new(
            "act_del_1".into(),
            "delete_file".into(),
            "src/main.rs".into(),
            serde_json::json!({"path": "src/main.rs"}),
            RiskLevel::High,
        );

        let err = gate
            .dispatch_for_task("task_del", &action, "developer")
            .await
            .unwrap_err();

        match err {
            DomainError::Conflict(msg) => {
                assert!(msg.contains("requires human approval"));
            }
            other => panic!("Expected Conflict error requiring approval, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_critical_risk_action_denied() {
        let gate = DeterministicGate::default();
        let action = Action::new(
            "act_drop_1".into(),
            "drop_database".into(),
            "production.db".into(),
            serde_json::json!({"force": true}),
            RiskLevel::Critical,
        );

        let err = gate
            .dispatch_for_task("task_crit", &action, "developer")
            .await
            .unwrap_err();

        match err {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("denied"));
            }
            other => panic!("Expected Unauthorized error, got: {other:?}"),
        }
    }
}
