use super::traits::{ExecutionResult, ToolGate};
use crate::authority::AuthorityEngine;
use crate::contracts::sandbox::SandboxPort;
use crate::sandbox::PathSandbox;
use async_trait::async_trait;
use custos_domain::{
    digest, new_id, Action, DomainError, ExecutionReceipt, ReceiptStatus, VerificationClaim,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub struct DeterministicGate {
    pub authority: Arc<AuthorityEngine>,
    pub sandbox: PathSandbox,
    pub driver: Option<Arc<dyn SandboxPort>>,
    memory_files: Arc<RwLock<HashMap<PathBuf, Vec<u8>>>>,
}

impl DeterministicGate {
    pub fn new(authority: Arc<AuthorityEngine>, workspace_root: PathBuf) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self {
            authority,
            sandbox,
            driver: None,
            memory_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_default_authority() -> Self {
        let current_dir = PathBuf::from("/workspace");
        let sandbox = PathSandbox::new(current_dir).expect("Valid current directory");
        Self {
            authority: Arc::new(AuthorityEngine::default()),
            sandbox,
            driver: None,
            memory_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_workspace(workspace_root: impl Into<PathBuf>) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self {
            authority: Arc::new(AuthorityEngine::default()),
            sandbox,
            driver: None,
            memory_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_authority_and_workspace(
        authority: Arc<AuthorityEngine>,
        workspace_root: impl Into<PathBuf>,
    ) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self {
            authority,
            sandbox,
            driver: None,
            memory_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_driver(
        authority: Arc<AuthorityEngine>,
        workspace_root: impl Into<PathBuf>,
        driver: Arc<dyn SandboxPort>,
    ) -> Self {
        let sandbox = PathSandbox::new(workspace_root).expect("Valid workspace root");
        Self {
            authority,
            sandbox,
            driver: Some(driver),
            memory_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_memory_file(&self, path: impl Into<PathBuf>, content: impl AsRef<[u8]>) {
        let mut files = self.memory_files.write().unwrap();
        files.insert(path.into(), content.as_ref().to_vec());
    }

    pub fn get_memory_file(&self, path: &Path) -> Option<String> {
        let files = self.memory_files.read().unwrap();
        files
            .get(path)
            .map(|b| String::from_utf8_lossy(b).to_string())
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

        let files = self.memory_files.read().unwrap();
        let bytes = match files.get(&resolved_path) {
            Some(b) => b.clone(),
            None => {
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
                    assurance: custos_domain::Assurance::CustosMediated,
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
            assurance: custos_domain::Assurance::CustosMediated,
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

        let files = self.memory_files.read().unwrap();
        let mut entries = Vec::new();
        for path in files.keys() {
            if let Ok(rel) = path.strip_prefix(&resolved_path) {
                if let Some(first_comp) = rel.components().next() {
                    let name = first_comp.as_os_str().to_string_lossy().to_string();
                    if !entries.contains(&name) {
                        entries.push(name);
                    }
                }
            }
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
            assurance: custos_domain::Assurance::CustosMediated,
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

        let files = self.memory_files.read().unwrap();
        let original_content = if let Some(bytes) = files.get(&resolved_path) {
            String::from_utf8_lossy(bytes).to_string()
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
            assurance: custos_domain::Assurance::CustosMediated,
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
        _permit_id: &str,
    ) -> Result<ExecutionResult, DomainError> {
        Err(DomainError::Unauthorized(format!(
            "Unsupported capability '{}': no physical dispatcher is registered",
            action.name
        )))
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
        let mut bound_action = action.clone();
        if bound_action.task_id.is_none() {
            bound_action.task_id = Some(task_id.to_string());
        }

        // Step 1: Evaluate policy, classify risk, and issue permit via AuthorityEngine
        let permit = self
            .authority
            .authorize_action(task_id, &bound_action, actor)
            .await?;

        // Step 2: Atomic consume/burn permit before physical execution (Invariant INV-04)
        let expected_digest = bound_action.argument_digest();
        self.authority
            .permits
            .consume_permit(&permit.id, &expected_digest)?;

        // Step 3: Dispatch action under sandbox containment
        if let Some(ref driver) = self.driver {
            match bound_action.name.as_str() {
                "read_file" | "list_files" | "patch_preview" => {
                    let receipt = driver.execute(&bound_action, &permit).await?;
                    let success = receipt.status == ReceiptStatus::Success;
                    let evidence = if success {
                        Some(receipt.output_digest.clone())
                    } else {
                        None
                    };
                    let output = receipt
                        .output_data
                        .clone()
                        .unwrap_or(serde_json::Value::Null);
                    Ok(ExecutionResult {
                        success,
                        output,
                        permit_id: Some(permit.id),
                        evidence,
                        receipt: Some(receipt),
                    })
                }
                _ => self.execute_generic(&bound_action, &permit.id).await,
            }
        } else {
            match bound_action.name.as_str() {
                "read_file" => self.execute_read_file(&bound_action, &permit.id).await,
                "list_files" => self.execute_list_files(&bound_action, &permit.id).await,
                "patch_preview" => self.execute_patch_preview(&bound_action, &permit.id).await,
                _ => self.execute_generic(&bound_action, &permit.id).await,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::RiskLevel;

    #[tokio::test]
    async fn test_low_risk_read_file_allowed_with_permit_sandbox_and_receipt() {
        let temp_dir = PathBuf::from("/workspace/test_env");
        let gate = DeterministicGate::with_workspace(&temp_dir);
        let file_path = temp_dir.join("sample.txt");
        gate.add_memory_file(&file_path, "Hello Custos Sovereign!");

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
    }

    #[tokio::test]
    async fn test_unknown_low_risk_action_does_not_fabricate_success() {
        let gate = DeterministicGate::default();
        let action = Action::new(
            "act_unknown_1".into(),
            "unknown_tool".into(),
            "target".into(),
            serde_json::json!({"target": "target"}),
            RiskLevel::Low,
        );

        let result = gate
            .dispatch_for_task("task_unknown", &action, "developer")
            .await;
        assert!(matches!(
            result,
            Err(DomainError::Unauthorized(message)) if message.contains("Unsupported capability")
        ));
    }

    #[tokio::test]
    async fn test_low_risk_patch_preview_simulated_without_disk_mutation() {
        let temp_dir = PathBuf::from("/workspace/test_env");
        let gate = DeterministicGate::with_workspace(&temp_dir);
        let file_path = temp_dir.join("code.rs");
        let initial_content = "fn main() { println!(\"old\"); }";
        gate.add_memory_file(&file_path, initial_content);

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
        let disk_content = gate.get_memory_file(&file_path).unwrap();
        assert_eq!(disk_content, initial_content);
    }

    #[tokio::test]
    async fn test_sandbox_path_traversal_denied() {
        let temp_dir = PathBuf::from("/workspace/test_env");
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
    }

    #[tokio::test]
    async fn test_uncertain_outcome_missing_file_fails_safely() {
        let temp_dir = PathBuf::from("/workspace/test_env");
        let gate = DeterministicGate::with_workspace(&temp_dir);
        let action = Action::new(
            "act_miss_1".into(),
            "read_file".into(),
            "non_existent.txt".into(),
            serde_json::json!({"path": "non_existent.txt"}),
            RiskLevel::Low,
        );

        let res = gate
            .dispatch_for_task("task_miss", &action, "developer")
            .await
            .expect("Gate dispatch completes with failure outcome");

        assert!(!res.success);
        assert!(res.evidence.is_none());
        assert_eq!(res.output["status"], "failed");
        let receipt = res.receipt.unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Failure);
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
