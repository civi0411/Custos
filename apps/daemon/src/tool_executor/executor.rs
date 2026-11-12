use super::parsing::{is_known_workbench_tool, parse_tool_invocation, parse_tool_with_registry};
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_domain::DomainError;
use std::sync::Arc;

/// Concrete tool executor for GovernedAgentRuntime wiring SovereignDeveloperAdapter, McpFederationHub, and DomainPackRegistry.
pub struct DaemonGovernedToolExecutor {
    developer_adapter: Arc<SovereignDeveloperAdapter>,
    kernel: Arc<dyn custos_core::contracts::kernel::KernelPort>,
    outbox: custos_persistence::repositories::outbox::OutboxRepository,
    mcp_hub: Option<Arc<custos_adapters::mcp::McpFederationHub>>,
    pack_registry: Option<Arc<custos_packs::DomainPackRegistry>>,
    supervision: Option<Arc<dyn custos_core::contracts::supervision::SupervisionTraceSink>>,
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
            pack_registry: None,
            supervision: None,
        }
    }

    pub fn with_mcp_hub(mut self, hub: Arc<custos_adapters::mcp::McpFederationHub>) -> Self {
        self.mcp_hub = Some(hub);
        self
    }

    pub fn with_pack_registry(mut self, registry: Arc<custos_packs::DomainPackRegistry>) -> Self {
        self.pack_registry = Some(registry);
        self
    }

    pub fn with_supervision(
        mut self,
        sink: Arc<dyn custos_core::contracts::supervision::SupervisionTraceSink>,
    ) -> Self {
        self.supervision = Some(sink);
        self
    }

    /// Parses tool name and parameters into canonical action, risk level, and target.
    /// Rejects unknown tools and parameter safety violations (such as directory traversal).
    pub fn parse_tool_invocation(
        name: &str,
        params: &serde_json::Value,
    ) -> Result<(String, custos_domain::RiskLevel, String), DomainError> {
        parse_tool_invocation(name, params)
    }

    /// Parses tool name and parameters with optional domain pack registry awareness.
    /// Rejects uninstalled workbench tools with DomainError::Unauthorized.
    pub fn parse_tool_with_registry(
        name: &str,
        params: &serde_json::Value,
        registry: Option<&custos_packs::DomainPackRegistry>,
    ) -> Result<(String, custos_domain::RiskLevel, String), DomainError> {
        parse_tool_with_registry(name, params, registry)
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
        let (action_name, risk_level, _target) =
            Self::parse_tool_with_registry(name, params, self.pack_registry.as_deref())?;

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
        let started_ms = chrono::Utc::now().timestamp_millis();
        let span_id = custos_domain::new_id("step");
        let res = self.execute_inner(ctx, name, params).await;
        if let Some(ref sink) = self.supervision {
            let ended_ms = chrono::Utc::now().timestamp_millis();
            let status = match &res {
                Ok(custos_runtime::agent::runtime_port::ToolResult::Error(_)) | Err(_) => {
                    custos_domain::supervision::StepSpanStatus::Error
                }
                Ok(_) => custos_domain::supervision::StepSpanStatus::Success,
            };
            sink.record_span(custos_domain::supervision::RunStepSpan {
                id: span_id.clone(),
                run_id: ctx.run_id.clone(),
                worker_run_id: Some(ctx.task_id.clone()),
                parent_id: None,
                kind: custos_domain::supervision::StepSpanKind::Tool,
                name: name.to_string(),
                status,
                started_at_ms: started_ms,
                ended_at_ms: Some(ended_ms),
                duration_ms: Some(ended_ms.saturating_sub(started_ms)),
                metadata: serde_json::json!({ "actor": ctx.actor, "params": params }),
            });
            let err_msg = match &res {
                Ok(custos_runtime::agent::runtime_port::ToolResult::Error(msg)) => Some(msg.clone()),
                Err(e) => Some(e.to_string()),
                _ => None,
            };
            if let Some(msg) = err_msg {
                let (_, sig) =
                    custos_domain::supervision::compute_error_signature("ToolExecutionError", &msg);
                sink.record_error(custos_domain::supervision::StepError {
                    id: custos_domain::new_id("serr"),
                    run_id: ctx.run_id.clone(),
                    worker_run_id: Some(ctx.task_id.clone()),
                    span_id: Some(span_id),
                    error_type: "ToolExecutionError".into(),
                    message: msg,
                    stack: None,
                    agent: ctx.actor.clone(),
                    signature_hash: sig,
                    timestamp_ms: ended_ms,
                });
            }
        }
        res
    }
}

impl DaemonGovernedToolExecutor {
    async fn execute_inner(
        &self,
        ctx: &custos_runtime::agent::runtime_port::InvocationContext,
        name: &str,
        params: &serde_json::Value,
    ) -> Result<custos_runtime::agent::runtime_port::ToolResult, DomainError> {
        let (action_name, risk_level, target) =
            Self::parse_tool_with_registry(name, params, self.pack_registry.as_deref())?;

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

        // 4. Physical / Remote / Pack dispatch
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
        } else if let Some(pack) = if is_known_workbench_tool(name) {
            self.pack_registry
                .as_ref()
                .and_then(|r| r.find_pack_for_tool(name))
        } else {
            None
        } {
            // Domain pack dispatch
            match pack.execute_tool(name, params).await {
                Ok(val) => {
                    let receipt = custos_domain::ExecutionReceipt {
                        receipt_id: custos_domain::new_id("rcpt"),
                        permit_id: permit.id.clone(),
                        action_id: action_intent.id.clone(),
                        status: custos_domain::ReceiptStatus::Success,
                        output_digest: format!(
                            "sha256:{}",
                            custos_domain::digest(val.to_string().as_bytes())
                        ),
                        output_data: Some(val.clone()),
                        error_message: None,
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
                    Ok(custos_runtime::agent::runtime_port::ToolResult::Success(
                        val,
                    ))
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

#[async_trait::async_trait]
impl custos_adapters::mcp::McpToolGate for DaemonGovernedToolExecutor {
    async fn authorize(
        &self,
        namespaced_tool: &str,
        _arguments_digest: &str,
        ctx: &custos_adapters::mcp::ToolCallContext,
    ) -> Result<(), DomainError> {
        if custos_adapters::mcp::security::is_dangerous_tool_pattern(namespaced_tool)
            && ctx.permit_id.as_deref().unwrap_or("").is_empty()
        {
            return Err(DomainError::Unauthorized(format!(
                "Tool '{namespaced_tool}' requires explicit permit"
            )));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl custos_core::contracts::ToolGate for DaemonGovernedToolExecutor {
    async fn dispatch(
        &self,
        action: &custos_domain::Action,
    ) -> Result<custos_core::contracts::ExecutionResult, DomainError> {
        self.dispatch_for_task("task_default", action, "daemon_governed_gate")
            .await
    }

    async fn dispatch_for_task(
        &self,
        task_id: &str,
        action: &custos_domain::Action,
        actor: &str,
    ) -> Result<custos_core::contracts::ExecutionResult, DomainError> {
        use custos_runtime::agent::runtime_port::{
            GovernedToolExecutor, InvocationContext, ToolResult,
        };
        let ctx = InvocationContext {
            task_id: task_id.to_string(),
            run_id: custos_domain::new_id("run"),
            actor: actor.to_string(),
            scope: None,
        };
        self.authorize(&ctx, &action.name, &action.parameters)
            .await?;
        let res = self.execute(&ctx, &action.name, &action.parameters).await?;
        match res {
            ToolResult::Success(val) => Ok(custos_core::contracts::ExecutionResult {
                success: true,
                output: val,
                permit_id: None,
                evidence: None,
                receipt: None,
            }),
            ToolResult::PendingApproval { attempt_id } => {
                Ok(custos_core::contracts::ExecutionResult {
                    success: false,
                    output: serde_json::json!({ "pending_approval": attempt_id }),
                    permit_id: None,
                    evidence: None,
                    receipt: None,
                })
            }
            ToolResult::Error(err) => Ok(custos_core::contracts::ExecutionResult {
                success: false,
                output: serde_json::json!({ "error": err }),
                permit_id: None,
                evidence: None,
                receipt: None,
            }),
        }
    }
}

