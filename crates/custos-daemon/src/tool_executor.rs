//! Governed Tool Executor Implementation for Custos Daemon
//!
//! Enforces policy gating, argument traversal protection, action permits,
//! durable outbox logging, and sandbox/MCP dispatching.

use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_domain::DomainError;
use std::sync::Arc;

/// Concrete tool executor for GovernedAgentRuntime wiring SovereignDeveloperAdapter and McpFederationHub.
pub struct DaemonGovernedToolExecutor {
    developer_adapter: Arc<SovereignDeveloperAdapter>,
    kernel: Arc<dyn custos_core::contracts::kernel::KernelPort>,
    outbox: custos_persistence::repositories::outbox::OutboxRepository,
    mcp_hub: Option<Arc<custos_adapters::mcp::McpFederationHub>>,
}

impl DaemonGovernedToolExecutor {
    pub fn new(
        developer_adapter: Arc<SovereignDeveloperAdapter>,
        kernel: Arc<dyn custos_core::contracts::kernel::KernelPort>,
        outbox: custos_persistence::repositories::outbox::OutboxRepository,
    ) -> Self {
        Self {
            developer_adapter,
            kernel,
            outbox,
            mcp_hub: None,
        }
    }

    pub fn with_mcp_hub(mut self, hub: Arc<custos_adapters::mcp::McpFederationHub>) -> Self {
        self.mcp_hub = Some(hub);
        self
    }

    /// Parses tool name and parameters into canonical action, risk level, and target.
    /// Rejects unknown tools and parameter safety violations (such as directory traversal).
    pub fn parse_tool_invocation(
        name: &str,
        params: &serde_json::Value,
    ) -> Result<(String, custos_domain::RiskLevel, String), DomainError> {
        // 1. Parameter safety check: directory traversal
        for key in &["path", "file", "target_file", "target", "dir"] {
            if let Some(val) = params.get(key).and_then(|v| v.as_str()) {
                if val.contains("..") {
                    return Err(DomainError::Validation(format!(
                        "Path traversal is forbidden in parameter '{key}': '{val}'"
                    )));
                }
            }
        }

        // 2. Resolve action name, risk level, and target
        if name.contains("__") {
            let risk = if custos_adapters::mcp::security::is_dangerous_tool_pattern(name) {
                custos_domain::RiskLevel::High
            } else {
                custos_domain::RiskLevel::Medium
            };
            let tgt = params
                .get("target")
                .or_else(|| params.get("path"))
                .or_else(|| params.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or(name);
            Ok((name.to_string(), risk, tgt.to_string()))
        } else {
            let act = match name {
                "read_file" | "read" => "read_file",
                "write_file" | "write" => "write_file",
                "list_dir" | "list" | "list_files" => "list_dir",
                "bash" | "shell" | "exec" => "bash",
                "patch_preview" => "patch_preview",
                unknown => {
                    return Err(DomainError::Validation(format!(
                        "Unknown or unmapped tool: '{unknown}'"
                    )))
                }
            };
            let risk = match act {
                "write_file" => custos_domain::RiskLevel::Medium,
                "bash" => {
                    let cmd = params
                        .get("command")
                        .or_else(|| params.get("cmd"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if cmd.contains("mkfs") || cmd.contains("dd if=") || cmd.contains("format_disk")
                    {
                        custos_domain::RiskLevel::Critical
                    } else {
                        custos_domain::RiskLevel::High
                    }
                }
                _ => custos_domain::RiskLevel::Low,
            };
            let tgt = params
                .get("path")
                .or_else(|| params.get("file"))
                .or_else(|| params.get("target_file"))
                .or_else(|| params.get("command"))
                .or_else(|| params.get("cmd"))
                .or_else(|| params.get("target"))
                .and_then(|v| v.as_str())
                .unwrap_or("workspace");
            Ok((act.to_string(), risk, tgt.to_string()))
        }
    }
}

#[async_trait::async_trait]
impl custos_runtime::agent::runtime_port::GovernedToolExecutor for DaemonGovernedToolExecutor {
    async fn authorize(
        &self,
        ctx: &custos_runtime::agent::runtime_port::InvocationContext,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<(), DomainError> {
        let (action_name, risk_level, _target) = Self::parse_tool_invocation(name, params)?;

        if risk_level == custos_domain::RiskLevel::Critical {
            return Err(DomainError::Unauthorized(format!(
                "Tool '{name}' is forbidden by kernel policy (critical risk)"
            )));
        }

        if name.starts_with("dangerous_")
            || (name.contains("__")
                && custos_adapters::mcp::security::is_dangerous_tool_pattern(name))
        {
            let grant = self
                .kernel
                .find_active_grant(&ctx.task_id, &action_name)
                .await?;
            if grant.is_none() {
                return Err(DomainError::Unauthorized(format!(
                    "Tool '{name}' requires explicit capability grant"
                )));
            }
        }
        Ok(())
    }

    async fn execute(
        &self,
        ctx: &custos_runtime::agent::runtime_port::InvocationContext,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<custos_runtime::agent::runtime_port::ToolResult, DomainError> {
        let (action_name, risk_level, target) = Self::parse_tool_invocation(name, params)?;

        let action_id = custos_domain::new_id("act");
        let mut action_intent = custos_domain::ActionIntent::new(
            action_id.clone(),
            action_name.to_string(),
            target,
            params.clone(),
            risk_level,
        )
        .with_task_id(ctx.task_id.clone());
        action_intent.assurance = custos_domain::Assurance::CustosMediated;

        // 1. Authorize action and obtain permit via Kernel
        let permit = match self
            .kernel
            .request_permit(&ctx.task_id, &action_intent, &ctx.actor)
            .await
        {
            Ok(p) => p,
            Err(DomainError::Conflict(reason)) => {
                let attempt_id = custos_domain::new_id("att");
                let entry = custos_core::contracts::storage::OutboxEntry {
                    id: attempt_id.clone(),
                    task_id: ctx.task_id.clone(),
                    action_id: action_id.clone(),
                    permit_id: "".to_string(),
                    argument_digest: action_intent.argument_digest(),
                    idempotency_key: Some(format!("{}:{}", ctx.task_id, action_id)),
                    status: custos_core::contracts::storage::OutboxStatus::Pending,
                    created_at: chrono::Utc::now(),
                    receipt: None,
                    recipient: None,
                    payload_json: Some(params.to_string()),
                    error_message: Some(reason),
                };
                custos_core::contracts::storage::OutboxPort::enqueue(&self.outbox, entry).await?;
                return Ok(
                    custos_runtime::agent::runtime_port::ToolResult::PendingApproval { attempt_id },
                );
            }
            Err(e) => return Err(e),
        };

        // 2. Consume permit atomically (INV-04)
        let permit = self
            .kernel
            .consume_permit(&permit.id, &action_intent)
            .await?;

        // 3. Durable ledger admission: MUST succeed before dispatching side-effect
        let attempt_id = custos_domain::new_id("att");
        let idempotency_key = action_intent
            .idempotency_key
            .clone()
            .unwrap_or_else(|| format!("{}:{}", ctx.task_id, action_id));
        let attempt = custos_domain::EffectAttempt {
            id: attempt_id.clone(),
            node_attempt_id: ctx.run_id.clone(),
            permit_id: permit.id.clone(),
            idempotency_key: idempotency_key.clone(),
            status: custos_domain::EffectStatus::InFlight,
            started_at: chrono::Utc::now(),
            ended_at: None,
            receipt: None,
        };
        self.outbox.record_effect(&attempt)?;

        let outbox_entry = custos_core::contracts::storage::OutboxEntry {
            id: attempt_id.clone(),
            task_id: ctx.task_id.clone(),
            action_id: action_id.clone(),
            permit_id: permit.id.clone(),
            argument_digest: action_intent.argument_digest(),
            idempotency_key: Some(idempotency_key),
            status: custos_core::contracts::storage::OutboxStatus::Dispatching,
            created_at: chrono::Utc::now(),
            receipt: None,
            recipient: None,
            payload_json: Some(params.to_string()),
            error_message: None,
        };
        custos_core::contracts::storage::OutboxPort::enqueue(&self.outbox, outbox_entry).await?;

        // 4. Physical / Remote dispatch
        if name.contains("__") {
            let hub = self.mcp_hub.as_ref().ok_or_else(|| {
                DomainError::Validation(
                    "MCP federation hub is not configured on this tool executor".into(),
                )
            })?;
            let tool_ctx = custos_adapters::mcp::ToolCallContext::new(&permit.id, &ctx.run_id);
            match hub
                .dispatch_call(name, Some(params.clone()), &tool_ctx)
                .await
            {
                Ok(res) => {
                    let is_error = res.is_error.unwrap_or(false);
                    let val = serde_json::to_value(&res)
                        .unwrap_or_else(|_| serde_json::json!({"success": !is_error}));
                    let receipt = custos_domain::ExecutionReceipt {
                        receipt_id: custos_domain::new_id("rcpt"),
                        permit_id: permit.id.clone(),
                        action_id: action_intent.id.clone(),
                        status: if is_error {
                            custos_domain::ReceiptStatus::Failure
                        } else {
                            custos_domain::ReceiptStatus::Success
                        },
                        output_digest: format!(
                            "sha256:{}",
                            custos_domain::digest(val.to_string().as_bytes())
                        ),
                        output_data: Some(val.clone()),
                        error_message: if is_error {
                            Some(format!("{:?}", res.content))
                        } else {
                            None
                        },
                        duration_ms: None,
                        executed_at: chrono::Utc::now(),
                        assurance: custos_domain::Assurance::CustosMediated,
                    };
                    let status = custos_domain::EffectStatus::from(receipt.status);
                    self.outbox
                        .update_effect_status(&attempt_id, status, Some(&receipt))?;
                    if let Err(settle_err) =
                        custos_core::contracts::storage::OutboxPort::mark_receipted(
                            &self.outbox,
                            &attempt_id,
                            receipt.clone(),
                        )
                        .await
                    {
                        let _ = self.outbox.update_effect_status(
                            &attempt_id,
                            custos_domain::EffectStatus::Uncertain,
                            Some(&receipt),
                        );
                        let _ = custos_core::contracts::storage::OutboxPort::mark_uncertain(
                            &self.outbox,
                            &attempt_id,
                        )
                        .await;
                        return Err(DomainError::Validation(format!(
                            "Outbox settlement failed: {settle_err}"
                        )));
                    }
                    if is_error {
                        Ok(custos_runtime::agent::runtime_port::ToolResult::Error(
                            format!("{:?}", res.content),
                        ))
                    } else {
                        Ok(custos_runtime::agent::runtime_port::ToolResult::Success(
                            val,
                        ))
                    }
                }
                Err(err) => {
                    let _ = self.outbox.update_effect_status(
                        &attempt_id,
                        custos_domain::EffectStatus::Uncertain,
                        None,
                    );
                    let _ = custos_core::contracts::storage::OutboxPort::mark_uncertain(
                        &self.outbox,
                        &attempt_id,
                    )
                    .await;
                    Err(err)
                }
            }
        } else {
            let receipt_res = custos_core::contracts::sandbox::SandboxPort::execute(
                self.developer_adapter.as_ref(),
                &action_intent,
                &permit,
            )
            .await;

            match receipt_res {
                Ok(receipt) => {
                    let status = custos_domain::EffectStatus::from(receipt.status);
                    self.outbox
                        .update_effect_status(&attempt_id, status, Some(&receipt))?;
                    if let Err(settle_err) =
                        custos_core::contracts::storage::OutboxPort::mark_receipted(
                            &self.outbox,
                            &attempt_id,
                            receipt.clone(),
                        )
                        .await
                    {
                        let _ = self.outbox.update_effect_status(
                            &attempt_id,
                            custos_domain::EffectStatus::Uncertain,
                            Some(&receipt),
                        );
                        let _ = custos_core::contracts::storage::OutboxPort::mark_uncertain(
                            &self.outbox,
                            &attempt_id,
                        )
                        .await;
                        return Err(DomainError::Validation(format!(
                            "Outbox settlement failed: {settle_err}"
                        )));
                    }
                    if receipt.status == custos_domain::ReceiptStatus::Success {
                        Ok(custos_runtime::agent::runtime_port::ToolResult::Success(
                            receipt
                                .output_data
                                .unwrap_or(serde_json::json!({ "success": true })),
                        ))
                    } else {
                        Ok(custos_runtime::agent::runtime_port::ToolResult::Error(
                            receipt
                                .error_message
                                .unwrap_or_else(|| "Execution failed".into()),
                        ))
                    }
                }
                Err(err) => {
                    let _ = self.outbox.update_effect_status(
                        &attempt_id,
                        custos_domain::EffectStatus::Uncertain,
                        None,
                    );
                    let _ = custos_core::contracts::storage::OutboxPort::mark_uncertain(
                        &self.outbox,
                        &attempt_id,
                    )
                    .await;
                    Err(err)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_invocation_valid_standard_tools() {
        let params = serde_json::json!({ "path": "crates/custos-core/src/lib.rs" });
        let (act, risk, target) =
            DaemonGovernedToolExecutor::parse_tool_invocation("read_file", &params).unwrap();
        assert_eq!(act, "read_file");
        assert_eq!(risk, custos_domain::RiskLevel::Low);
        assert_eq!(target, "crates/custos-core/src/lib.rs");

        let write_params = serde_json::json!({ "path": "output.txt", "content": "hello" });
        let (act, risk, _) =
            DaemonGovernedToolExecutor::parse_tool_invocation("write_file", &write_params).unwrap();
        assert_eq!(act, "write_file");
        assert_eq!(risk, custos_domain::RiskLevel::Medium);
    }

    #[test]
    fn test_parse_tool_invocation_rejects_path_traversal() {
        let params = serde_json::json!({ "path": "../../etc/passwd" });
        let res = DaemonGovernedToolExecutor::parse_tool_invocation("read_file", &params);
        assert!(res.is_err());
        assert!(
            matches!(res.unwrap_err(), DomainError::Validation(msg) if msg.contains("Path traversal"))
        );
    }

    #[test]
    fn test_parse_tool_invocation_rejects_unknown_tools() {
        let params = serde_json::json!({});
        let res = DaemonGovernedToolExecutor::parse_tool_invocation("unregistered_tool", &params);
        assert!(res.is_err());
        assert!(
            matches!(res.unwrap_err(), DomainError::Validation(msg) if msg.contains("Unknown or unmapped"))
        );
    }

    #[test]
    fn test_parse_tool_invocation_classifies_critical_bash_commands() {
        let safe_cmd = serde_json::json!({ "command": "cargo check" });
        let (_, risk, _) =
            DaemonGovernedToolExecutor::parse_tool_invocation("bash", &safe_cmd).unwrap();
        assert_eq!(risk, custos_domain::RiskLevel::High);

        let destructive_cmd = serde_json::json!({ "command": "mkfs.ext4 /dev/sda1" });
        let (_, risk, _) =
            DaemonGovernedToolExecutor::parse_tool_invocation("bash", &destructive_cmd).unwrap();
        assert_eq!(risk, custos_domain::RiskLevel::Critical);
    }

    #[test]
    fn test_parse_tool_invocation_mcp_tools() {
        let mcp_params = serde_json::json!({ "target": "https://example.com" });
        let (act, risk, target) =
            DaemonGovernedToolExecutor::parse_tool_invocation("web__fetch", &mcp_params).unwrap();
        assert_eq!(act, "web__fetch");
        assert_eq!(risk, custos_domain::RiskLevel::Medium);
        assert_eq!(target, "https://example.com");
    }
}
