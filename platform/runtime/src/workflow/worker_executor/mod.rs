//! Sovereign Worker Step Executor (PR 4 - Gate 5 / Execution Spine)
//!
//! Routes DAG node execution steps to ModelPort (direct LLM turn),
//! AgentRuntimePort (governed harness session), or SandboxPort (tool execution).
//! Unbound worker actions are rejected instead of fabricating execution results.

use async_trait::async_trait;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::storage::{EffectLedgerPort, OutboxEntry, OutboxPort, OutboxStatus};
use custos_domain::{new_id, ActionIntent, DomainError, EffectAttempt, EffectStatus};
use custos_provider::request::ProviderRequest;
use custos_provider::ModelPort;
use std::sync::Arc;

use super::graph_runtime::StepExecutor;

#[cfg(test)]
mod tests;

pub struct WorkerExecutor {
    kernel: Option<Arc<dyn KernelPort>>,
    model: Option<Arc<dyn ModelPort>>,
    harness: Option<Arc<dyn AgentRuntimePort>>,
    sandbox: Option<Arc<dyn SandboxPort>>,
    outbox: Option<Arc<dyn OutboxPort>>,
    effect_ledger: Option<Arc<dyn EffectLedgerPort>>,
}

impl WorkerExecutor {
    pub fn new() -> Self {
        Self {
            kernel: None,
            model: None,
            harness: None,
            sandbox: None,
            outbox: None,
            effect_ledger: None,
        }
    }

    pub fn with_kernel(mut self, kernel: Arc<dyn KernelPort>) -> Self {
        self.kernel = Some(kernel);
        self
    }

    pub fn with_model(mut self, model: Arc<dyn ModelPort>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_harness(mut self, harness: Arc<dyn AgentRuntimePort>) -> Self {
        self.harness = Some(harness);
        self
    }

    pub fn with_sandbox(mut self, sandbox: Arc<dyn SandboxPort>) -> Self {
        self.sandbox = Some(sandbox);
        self
    }

    pub fn with_outbox(mut self, outbox: Arc<dyn OutboxPort>) -> Self {
        self.outbox = Some(outbox);
        self
    }

    pub fn with_effect_ledger(mut self, effect_ledger: Arc<dyn EffectLedgerPort>) -> Self {
        self.effect_ledger = Some(effect_ledger);
        self
    }

    pub fn kernel(&self) -> Option<&Arc<dyn KernelPort>> {
        self.kernel.as_ref()
    }

    pub fn model(&self) -> Option<&Arc<dyn ModelPort>> {
        self.model.as_ref()
    }

    pub fn harness(&self) -> Option<&Arc<dyn AgentRuntimePort>> {
        self.harness.as_ref()
    }

    pub fn sandbox(&self) -> Option<&Arc<dyn SandboxPort>> {
        self.sandbox.as_ref()
    }

    pub fn outbox(&self) -> Option<&Arc<dyn OutboxPort>> {
        self.outbox.as_ref()
    }

    pub fn effect_ledger(&self) -> Option<&Arc<dyn EffectLedgerPort>> {
        self.effect_ledger.as_ref()
    }
}

impl Default for WorkerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepExecutor for WorkerExecutor {
    async fn execute_step(
        &self,
        ctx: &super::contracts::ExecutionContext,
        action_type: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        if ctx.task_id.trim().is_empty() {
            return Err(DomainError::Validation(format!(
                "MissingContext: valid task_id is required to execute node '{}'",
                ctx.node_id
            )));
        }

        match action_type {
            "model_turn" | "model" | "llm" => {
                let model = self.model.as_ref().ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot execute step '{}': action_type '{}' requires ModelPort, but none was configured",
                        ctx.node_id, action_type
                    ))
                })?;

                let prompt = inputs
                    .get("prompt")
                    .or_else(|| inputs.get("instruction"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Execute model turn");

                let req = ProviderRequest::new(
                    new_id("req"),
                    &ctx.task_id,
                    1,
                    prompt.to_string(),
                    model.provider_id(),
                );

                let resp = model.generate(&req).await?;
                serde_json::to_value(&resp)
                    .map_err(|e| DomainError::Validation(format!("Serialization error: {}", e)))
            }

            "tool_call" | "sandbox" => {
                let kernel = self.kernel.as_ref().ok_or_else(|| DomainError::Unauthorized(format!(
                    "Cannot execute tool call on node '{}': KernelPort required to evaluate policy and mint ExecutionPermit",
                    ctx.node_id
                )))?;
                let sandbox = self.sandbox.as_ref().ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot execute step '{}': action_type '{}' requires SandboxPort, but none was configured",
                        ctx.node_id, action_type
                    ))
                })?;

                let action_name = inputs
                    .get("tool")
                    .or_else(|| inputs.get("action_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("developer__bash");

                let params = inputs
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));

                let risk_level = kernel.classify_effect(action_name, &params);
                let target = params
                    .get("path")
                    .or_else(|| params.get("file"))
                    .or_else(|| params.get("target_file"))
                    .or_else(|| params.get("target"))
                    .or_else(|| params.get("command"))
                    .or_else(|| params.get("cmd"))
                    .or_else(|| params.get("program"))
                    .and_then(|value| value.as_str())
                    .unwrap_or(action_name)
                    .to_string();

                let mut action = ActionIntent::new(
                    new_id("act"),
                    action_name.to_string(),
                    target,
                    params,
                    risk_level,
                )
                .with_task_id(ctx.task_id.clone());
                action.assurance = custos_domain::Assurance::CustosMediated;

                let proposed = match kernel
                    .request_permit(&ctx.task_id, &action, &ctx.worker_id)
                    .await
                {
                    Ok(p) => p,
                    Err(DomainError::Conflict(reason)) => {
                        let attempt_id = new_id("att");
                        if let Some(ref outbox) = self.outbox {
                            let entry = OutboxEntry {
                                id: attempt_id.clone(),
                                task_id: ctx.task_id.clone(),
                                action_id: action.id.clone(),
                                permit_id: "".to_string(),
                                argument_digest: action.argument_digest(),
                                idempotency_key: Some(format!("{}:{}", ctx.task_id, action.id)),
                                status: OutboxStatus::Pending,
                                created_at: chrono::Utc::now(),
                                receipt: None,
                                recipient: None,
                                payload_json: Some(action.parameters.to_string()),
                                error_message: Some(reason.clone()),
                            };
                            outbox.enqueue(entry).await?;
                        }
                        return Ok(serde_json::json!({
                            "status": "pending_approval",
                            "attempt_id": attempt_id,
                            "reason": reason,
                        }));
                    }
                    Err(e) => return Err(e),
                };

                let permit = kernel.consume_permit(&proposed.id, &action).await?;

                let idempotency_key = action
                    .idempotency_key
                    .clone()
                    .unwrap_or_else(|| action.id.clone());

                let effect_id = if let Some(ref ledger) = self.effect_ledger {
                    let mut effect = EffectAttempt::new(
                        ctx.run_id.clone(),
                        permit.id.clone(),
                        idempotency_key.clone(),
                    );
                    effect.status = EffectStatus::InFlight;
                    ledger.record_effect(&effect).await?;
                    Some(effect.id)
                } else {
                    None
                };

                let outbox_id = new_id("ob");
                if let Some(ref outbox) = self.outbox {
                    let entry = OutboxEntry::new(
                        outbox_id.clone(),
                        ctx.task_id.clone(),
                        action.id.clone(),
                        permit.id.clone(),
                        action.argument_digest(),
                        Some(idempotency_key),
                        OutboxStatus::Pending,
                        chrono::Utc::now(),
                    );
                    outbox.enqueue(entry).await?;
                    outbox.mark_approved(&outbox_id).await?;
                    outbox.mark_dispatching(&outbox_id).await?;
                }

                let receipt = match sandbox.execute(&action, &permit).await {
                    Ok(r) => {
                        if let Some(ref outbox) = self.outbox {
                            outbox.mark_receipted(&outbox_id, r.clone()).await?;
                        }
                        if let Some(ref ledger) = self.effect_ledger {
                            if let Some(ref eff_id) = effect_id {
                                let effect_status = EffectStatus::from(r.status);
                                ledger
                                    .update_effect_status(eff_id, effect_status, Some(&r))
                                    .await?;
                            }
                        }
                        r
                    }
                    Err(e) => {
                        if let Some(ref outbox) = self.outbox {
                            let _ = outbox.mark_uncertain(&outbox_id).await;
                        }
                        if let Some(ref ledger) = self.effect_ledger {
                            if let Some(ref eff_id) = effect_id {
                                let _ = ledger
                                    .update_effect_status(eff_id, EffectStatus::Uncertain, None)
                                    .await;
                            }
                        }
                        return Err(e);
                    }
                };
                serde_json::to_value(&receipt)
                    .map_err(|e| DomainError::Validation(format!("Serialization error: {}", e)))
            }

            "harness_run" | "agent_turn" => {
                let harness = self.harness.as_ref().ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot execute step '{}': action_type '{}' requires AgentRuntimePort, but none was configured",
                        ctx.node_id, action_type
                    ))
                })?;

                let wrun =
                    custos_domain::WorkerRun::new(ctx.task_id.clone(), ctx.worker_id.clone(), 3)
                        .with_run_id(ctx.run_id.clone());

                let context_pack = custos_domain::ContextPack::new(
                    new_id("ctx"),
                    Vec::new(),
                    0,
                    "sha256:empty".into(),
                );

                let intents = harness.execute_turn(&wrun, &context_pack).await?;
                serde_json::to_value(&intents)
                    .map_err(|e| DomainError::Validation(format!("Serialization error: {}", e)))
            }

            other => Err(DomainError::Validation(format!(
                "Unsupported action type '{}' for node '{}'",
                other, ctx.node_id
            ))),
        }
    }
}
