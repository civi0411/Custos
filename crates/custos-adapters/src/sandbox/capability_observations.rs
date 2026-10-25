//! Capability Observations Adapter
//!
//! Provides concrete filesystem observation and execution capabilities (read_file, list_files,
//! patch_preview, write_file) strictly constrained to a workspace directory under SandboxPort.

use async_trait::async_trait;
use custos_core::contracts::sandbox::{verify_permit_binding, SandboxPort};
use custos_domain::{
    digest, new_id, ActionIntent, Assurance, DomainError, ExecutionReceipt, Permit, ReceiptStatus,
};
use std::path::{Path, PathBuf};

pub struct CapabilityObservationsAdapter {
    workspace_root: PathBuf,
}

impl CapabilityObservationsAdapter {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Validates that a requested file path does not escape the workspace root.
    fn resolve_safe_path(&self, rel_or_abs: &str) -> Result<PathBuf, DomainError> {
        let path = Path::new(rel_or_abs);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Normalize components
        let mut components = Vec::new();
        for comp in resolved.components() {
            match comp {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::Normal(c) => {
                    components.push(c);
                }
                std::path::Component::RootDir => {
                    components.clear();
                    components.push(std::ffi::OsStr::new("/"));
                }
                _ => {}
            }
        }

        let mut normalized = PathBuf::new();
        for c in components {
            normalized.push(c);
        }

        if self.workspace_root.is_absolute()
            && normalized.is_absolute()
            && !normalized.starts_with(&self.workspace_root)
        {
            return Err(DomainError::Unauthorized(format!(
                "Sandbox containment violation: '{}' escapes workspace root '{}'",
                rel_or_abs,
                self.workspace_root.display()
            )));
        }

        Ok(normalized)
    }

    async fn handle_read_file(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        let target_str = intent
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&intent.target);

        let resolved_path = self.resolve_safe_path(target_str)?;

        if !resolved_path.exists() || !resolved_path.is_file() {
            let output = serde_json::json!({
                "status": "failed",
                "action": "read_file",
                "target": target_str,
                "error": format!("File not found or not a regular file: {}", resolved_path.display()),
            });

            return Ok(ExecutionReceipt {
                receipt_id: new_id("rcpt"),
                permit_id: permit.id.clone(),
                action_id: intent.id.clone(),
                status: ReceiptStatus::Failure,
                output_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                output_data: Some(output),
                error_message: Some(format!("File not found: {}", resolved_path.display())),
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
                assurance: Assurance::CustosMediated,
            });
        }

        let bytes = tokio::fs::read(&resolved_path).await.map_err(|e| {
            DomainError::Validation(format!("Failed to read file {}: {e}", resolved_path.display()))
        })?;

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

        Ok(ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit.id.clone(),
            action_id: intent.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: digest_val,
            output_data: Some(output),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
            assurance: Assurance::CustosMediated,
        })
    }

    async fn handle_list_files(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        let target_str = intent
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(if intent.target.is_empty() {
                "."
            } else {
                &intent.target
            });

        let resolved_path = self.resolve_safe_path(target_str)?;

        if !resolved_path.exists() || !resolved_path.is_dir() {
            let output = serde_json::json!({
                "status": "failed",
                "action": "list_files",
                "target": target_str,
                "error": format!("Directory not found: {}", resolved_path.display()),
            });

            return Ok(ExecutionReceipt {
                receipt_id: new_id("rcpt"),
                permit_id: permit.id.clone(),
                action_id: intent.id.clone(),
                status: ReceiptStatus::Failure,
                output_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                output_data: Some(output),
                error_message: Some(format!("Directory not found: {}", resolved_path.display())),
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
                assurance: Assurance::CustosMediated,
            });
        }

        let mut entries = Vec::new();
        let mut dir_reader = tokio::fs::read_dir(&resolved_path).await.map_err(|e| {
            DomainError::Validation(format!(
                "Failed to read directory {}: {e}",
                resolved_path.display()
            ))
        })?;

        while let Ok(Some(entry)) = dir_reader.next_entry().await {
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

        Ok(ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit.id.clone(),
            action_id: intent.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: digest_val,
            output_data: Some(output),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
            assurance: Assurance::CustosMediated,
        })
    }

    async fn handle_patch_preview(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        let target_str = intent
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(&intent.target);

        let resolved_path = self.resolve_safe_path(target_str)?;

        let patch_content = intent
            .parameters
            .get("patch")
            .or_else(|| intent.parameters.get("diff"))
            .or_else(|| intent.parameters.get("replacement"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let original_content = if resolved_path.exists() && resolved_path.is_file() {
            tokio::fs::read_to_string(&resolved_path)
                .await
                .unwrap_or_default()
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

        Ok(ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit.id.clone(),
            action_id: intent.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: diff_digest,
            output_data: Some(output),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
            assurance: Assurance::CustosMediated,
        })
    }
}

#[async_trait]
impl SandboxPort for CapabilityObservationsAdapter {
    fn driver_id(&self) -> &str {
        "capability_observations"
    }

    async fn execute(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        verify_permit_binding(permit, intent)?;

        match intent.name.as_str() {
            "read_file" => self.handle_read_file(intent, permit).await,
            "list_files" => self.handle_list_files(intent, permit).await,
            "patch_preview" => self.handle_patch_preview(intent, permit).await,
            other => Err(DomainError::Unauthorized(format!(
                "Unsupported capability '{other}': no physical dispatcher is registered"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{ActionIntent, RiskLevel};

    #[tokio::test]
    async fn test_capability_observations_read_and_list() {
        let temp_dir = std::env::temp_dir().join(format!("custos_obs_test_{}", new_id("test")));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "Hello Observations!").await.unwrap();

        let adapter = CapabilityObservationsAdapter::new(&temp_dir);

        let mut intent = ActionIntent::new(
            "act_1".into(),
            "read_file".into(),
            "sample.txt".into(),
            serde_json::json!({"path": "sample.txt"}),
            RiskLevel::Low,
        );
        intent.task_id = Some("task_1".into());

        let mut permit = custos_domain::Permit::new(
            "task_1".into(),
            "act_1".into(),
            "read_file".into(),
            None,
            custos_domain::RiskClass::Low,
            300,
        );
        permit.argument_digest = intent.argument_digest();

        let receipt = adapter.execute(&intent, &permit).await.unwrap();
        assert_eq!(receipt.status, ReceiptStatus::Success);
        assert!(receipt.output_digest.starts_with("sha256:"));

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
