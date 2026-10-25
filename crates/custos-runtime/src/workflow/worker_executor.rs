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
                                ledger.update_effect_status(eff_id, effect_status, Some(&r)).await?;
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
                                let _ = ledger.update_effect_status(eff_id, EffectStatus::Uncertain, None).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::contracts::ExecutionContext;
    use custos_domain::RiskLevel;
    use custos_provider::request::ModelResponse;

    struct TestMockModel;

    #[async_trait]
    impl ModelPort for TestMockModel {
        fn provider_id(&self) -> &str {
            "test_mock"
        }

        async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
            Ok(ModelResponse::text(
                format!("Mock generated response for {}", req.task_id),
                "test_mock",
                30,
            ))
        }
    }

    #[tokio::test]
    async fn unbound_worker_nodes_cannot_fabricate_results() {
        let executor = WorkerExecutor::new();
        let inputs = serde_json::json!({
            "role": "engineer",
            "harness_id": "claude_code"
        });
        let ctx = ExecutionContext::new("task_01", "run_01", "node_1", "worker_01");

        let error = executor
            .execute_step(&ctx, "execute_worker", &inputs)
            .await
            .unwrap_err();
        assert!(
            matches!(error, DomainError::Validation(message) if message.contains("Unsupported action type"))
        );
    }

    #[tokio::test]
    async fn test_worker_executor_model_routing() {
        let provider = Arc::new(TestMockModel);
        let executor = WorkerExecutor::new().with_model(provider);
        let ctx = ExecutionContext::new("task_model", "run_model", "node_model", "worker_model");

        let inputs = serde_json::json!({
            "prompt": "Analyze repository architecture"
        });

        let val = executor
            .execute_step(&ctx, "model_turn", &inputs)
            .await
            .expect("Model turn execution should succeed");

        assert!(val.get("content").is_some());
    }

    #[tokio::test]
    async fn test_worker_executor_missing_port_error() {
        let executor = WorkerExecutor::new();
        let inputs = serde_json::json!({});
        let ctx = ExecutionContext::new("task_err", "run_err", "node_err", "worker_err");

        let err = executor
            .execute_step(&ctx, "model_turn", &inputs)
            .await
            .expect_err("Should error when model port is missing");

        match err {
            DomainError::Validation(msg) => {
                assert!(msg.contains("requires ModelPort"));
            }
            other => panic!("Expected DomainError::Validation, got {:?}", other),
        }
    }

    struct DummySandbox;
    #[async_trait]
    impl SandboxPort for DummySandbox {
        fn driver_id(&self) -> &str {
            "dummy"
        }
        async fn execute(
            &self,
            _intent: &ActionIntent,
            _permit: &custos_domain::Permit,
        ) -> Result<custos_domain::ExecutionReceipt, DomainError> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn test_worker_executor_tool_call_without_kernel_unauthorized() {
        let executor = WorkerExecutor::new().with_sandbox(Arc::new(DummySandbox));
        let inputs = serde_json::json!({ "tool": "developer__bash" });
        let ctx = ExecutionContext::new("task_tool", "run_tool", "node_tool", "worker_tool");

        let err = executor
            .execute_step(&ctx, "tool_call", &inputs)
            .await
            .expect_err("Tool call without KernelPort must be rejected as Unauthorized");

        assert!(matches!(err, DomainError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn dag_step_uses_task_id_not_node_id() {
        struct CapturingKernel {
            captured_task_id: std::sync::Mutex<Option<String>>,
        }

        #[async_trait]
        impl KernelPort for CapturingKernel {
            async fn create_task(
                &self,
                _title: String,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
            async fn create_task_with_contract(
                &self,
                _title: String,
                _contract: custos_domain::TaskContract,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
            async fn get_task(
                &self,
                _task_id: &str,
            ) -> Result<Option<custos_domain::Task>, DomainError> {
                unreachable!()
            }
            async fn transition_task(
                &self,
                _task_id: &str,
                _next: custos_domain::TaskStatus,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
            async fn request_permit(
                &self,
                task_id: &str,
                _intent: &ActionIntent,
                _actor: &str,
            ) -> Result<custos_domain::Permit, DomainError> {
                *self.captured_task_id.lock().unwrap() = Some(task_id.to_string());
                Ok(custos_domain::Permit::new(
                    task_id.into(),
                    "act_1".into(),
                    "sandbox".into(),
                    None,
                    custos_domain::RiskClass::Low,
                    60,
                ))
            }
            async fn consume_permit(
                &self,
                _permit_id: &str,
                intent: &ActionIntent,
            ) -> Result<custos_domain::Permit, DomainError> {
                Ok(custos_domain::Permit::new(
                    intent
                        .task_id
                        .clone()
                        .unwrap_or_else(|| "task_default".into()),
                    intent.id.clone(),
                    "sandbox".into(),
                    None,
                    custos_domain::RiskClass::Low,
                    60,
                ))
            }
            async fn verify_evidence(
                &self,
                _bundle: &custos_core::evidence::EvidenceBundle,
            ) -> Result<custos_domain::VerificationClaim, DomainError> {
                unreachable!()
            }
            async fn grant_capability(
                &self,
                _task_id: &str,
                _grantee: &str,
                _capability: &str,
            ) -> Result<custos_domain::Grant, DomainError> {
                unreachable!()
            }
            async fn find_active_grant(
                &self,
                _task_id: &str,
                _capability: &str,
            ) -> Result<Option<custos_domain::Grant>, DomainError> {
                Ok(None)
            }
            async fn complete_task(
                &self,
                _task_id: &str,
                _summary: String,
                _claims: Vec<custos_domain::VerificationClaim>,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
        }

        struct EchoSandbox;
        #[async_trait]
        impl SandboxPort for EchoSandbox {
            fn driver_id(&self) -> &str {
                "echo"
            }
            async fn execute(
                &self,
                action: &ActionIntent,
                permit: &custos_domain::Permit,
            ) -> Result<custos_domain::ExecutionReceipt, DomainError> {
                Ok(custos_domain::ExecutionReceipt {
                    receipt_id: "rcpt_1".into(),
                    permit_id: permit.id.clone(),
                    action_id: action.id.clone(),
                    status: custos_domain::ReceiptStatus::Success,
                    output_digest: "blake3:00".into(),
                    output_data: Some(
                        serde_json::json!({ "task_id": action.task_id, "target": action.target }),
                    ),
                    error_message: None,
                    duration_ms: Some(1),
                    executed_at: chrono::Utc::now(),
                    assurance: custos_domain::Assurance::CustosMediated,
                })
            }
        }

        let kernel = Arc::new(CapturingKernel {
            captured_task_id: std::sync::Mutex::new(None),
        });
        let executor = WorkerExecutor::new()
            .with_kernel(kernel.clone())
            .with_sandbox(Arc::new(EchoSandbox));

        let ctx = ExecutionContext::new("real_task_123", "run_456", "dag_node_789", "worker_1");
        let inputs =
            serde_json::json!({ "tool": "read_file", "parameters": { "path": "test.txt" } });

        let res = executor
            .execute_step(&ctx, "tool_call", &inputs)
            .await
            .unwrap();
        assert_eq!(
            kernel.captured_task_id.lock().unwrap().as_deref(),
            Some("real_task_123")
        );
        assert_ne!(
            kernel.captured_task_id.lock().unwrap().as_deref(),
            Some("dag_node_789")
        );
        assert_eq!(res["output_data"]["task_id"], "real_task_123");
        assert_eq!(res["output_data"]["target"], "test.txt");
    }

    #[tokio::test]
    async fn shell_tool_does_not_inherit_read_risk_level() {
        struct MockKernel;
        #[async_trait]
        impl KernelPort for MockKernel {
            async fn create_task(
                &self,
                _title: String,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
            async fn create_task_with_contract(
                &self,
                _title: String,
                _contract: custos_domain::TaskContract,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
            async fn get_task(
                &self,
                _task_id: &str,
            ) -> Result<Option<custos_domain::Task>, DomainError> {
                unreachable!()
            }
            async fn transition_task(
                &self,
                _task_id: &str,
                _next: custos_domain::TaskStatus,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
            async fn request_permit(
                &self,
                _task_id: &str,
                _intent: &ActionIntent,
                _actor: &str,
            ) -> Result<custos_domain::Permit, DomainError> {
                unreachable!()
            }
            async fn consume_permit(
                &self,
                _permit_id: &str,
                _intent: &ActionIntent,
            ) -> Result<custos_domain::Permit, DomainError> {
                unreachable!()
            }
            async fn verify_evidence(
                &self,
                _bundle: &custos_core::evidence::EvidenceBundle,
            ) -> Result<custos_domain::VerificationClaim, DomainError> {
                unreachable!()
            }
            async fn grant_capability(
                &self,
                _task_id: &str,
                _grantee: &str,
                _capability: &str,
            ) -> Result<custos_domain::Grant, DomainError> {
                unreachable!()
            }
            async fn find_active_grant(
                &self,
                _task_id: &str,
                _capability: &str,
            ) -> Result<Option<custos_domain::Grant>, DomainError> {
                unreachable!()
            }
            async fn complete_task(
                &self,
                _task_id: &str,
                _summary: String,
                _claims: Vec<custos_domain::VerificationClaim>,
            ) -> Result<custos_domain::Task, DomainError> {
                unreachable!()
            }
        }
        let kernel = MockKernel;
        let read_risk = kernel.classify_effect("read_file", &serde_json::json!({}));
        let shell_risk =
            kernel.classify_effect("bash", &serde_json::json!({ "command": "echo 1" }));
        assert_eq!(read_risk, RiskLevel::Low);
        assert_eq!(shell_risk, RiskLevel::High);
        assert_ne!(read_risk, shell_risk);
    }

    struct MockLedgerAndOutbox {
        effects: std::sync::Mutex<Vec<EffectAttempt>>,
        outbox: std::sync::Mutex<Vec<OutboxEntry>>,
    }

    impl MockLedgerAndOutbox {
        fn new() -> Self {
            Self {
                effects: std::sync::Mutex::new(Vec::new()),
                outbox: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl EffectLedgerPort for MockLedgerAndOutbox {
        async fn record_effect(&self, effect: &EffectAttempt) -> Result<(), DomainError> {
            self.effects.lock().unwrap().push(effect.clone());
            Ok(())
        }
        async fn update_effect_status(
            &self,
            id: &str,
            status: EffectStatus,
            receipt: Option<&custos_domain::ExecutionReceipt>,
        ) -> Result<(), DomainError> {
            let mut list = self.effects.lock().unwrap();
            if let Some(eff) = list.iter_mut().find(|e| e.id == id) {
                eff.status = status;
                eff.receipt = receipt.cloned();
            }
            Ok(())
        }
        async fn get_effect_by_idempotency_key(
            &self,
            key: &str,
        ) -> Result<Option<EffectAttempt>, DomainError> {
            Ok(self.effects.lock().unwrap().iter().find(|e| e.idempotency_key == key).cloned())
        }
        async fn reconcile_on_startup(&self) -> Result<usize, DomainError> {
            Ok(0)
        }
    }

    #[async_trait]
    impl OutboxPort for MockLedgerAndOutbox {
        async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
            self.outbox.lock().unwrap().push(entry);
            Ok(())
        }
        async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError> {
            Ok(self.outbox.lock().unwrap().iter().find(|e| e.id == id).cloned())
        }
        async fn mark_approved(&self, id: &str) -> Result<(), DomainError> {
            let mut list = self.outbox.lock().unwrap();
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.status = OutboxStatus::Approved;
            }
            Ok(())
        }
        async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
            let mut list = self.outbox.lock().unwrap();
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.status = OutboxStatus::Dispatching;
            }
            Ok(())
        }
        async fn mark_receipted(&self, id: &str, receipt: custos_domain::ExecutionReceipt) -> Result<(), DomainError> {
            let mut list = self.outbox.lock().unwrap();
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.status = OutboxStatus::Receipted;
                e.receipt = Some(receipt);
            }
            Ok(())
        }
        async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError> {
            let mut list = self.outbox.lock().unwrap();
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.status = OutboxStatus::Uncertain;
            }
            Ok(())
        }
        async fn mark_failed(&self, id: &str, _error: &str) -> Result<(), DomainError> {
            let mut list = self.outbox.lock().unwrap();
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.status = OutboxStatus::Failed;
            }
            Ok(())
        }
        async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError> {
            Ok(self.outbox.lock().unwrap().iter().filter(|e| e.status == status).cloned().collect())
        }
        async fn list_by_task_and_status(
            &self,
            task_id: &str,
            status: OutboxStatus,
        ) -> Result<Vec<OutboxEntry>, DomainError> {
            Ok(self.outbox.lock().unwrap().iter().filter(|e| e.task_id == task_id && e.status == status).cloned().collect())
        }
    }

    struct WorkerTestSandbox;
    #[async_trait]
    impl SandboxPort for WorkerTestSandbox {
        fn driver_id(&self) -> &str { "test" }
        async fn execute(&self, action: &ActionIntent, permit: &custos_domain::Permit) -> Result<custos_domain::ExecutionReceipt, DomainError> {
            Ok(custos_domain::ExecutionReceipt {
                receipt_id: "rcpt_test".into(),
                permit_id: permit.id.clone(),
                action_id: action.id.clone(),
                status: custos_domain::ReceiptStatus::Success,
                output_digest: "sha256:00".into(),
                output_data: Some(serde_json::json!({ "success": true })),
                error_message: None,
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
                assurance: custos_domain::Assurance::CustosMediated,
            })
        }
    }

    struct WorkerTestKernel;
    #[async_trait]
    impl KernelPort for WorkerTestKernel {
        async fn create_task(&self, _: String) -> Result<custos_domain::Task, DomainError> { unreachable!() }
        async fn create_task_with_contract(&self, _: String, _: custos_domain::TaskContract) -> Result<custos_domain::Task, DomainError> { unreachable!() }
        async fn get_task(&self, _: &str) -> Result<Option<custos_domain::Task>, DomainError> { unreachable!() }
        async fn transition_task(&self, _: &str, _: custos_domain::TaskStatus) -> Result<custos_domain::Task, DomainError> { unreachable!() }
        async fn request_permit(&self, task_id: &str, intent: &ActionIntent, _: &str) -> Result<custos_domain::Permit, DomainError> {
            Ok(custos_domain::Permit::new(
                task_id.into(),
                intent.id.clone(),
                "sandbox".into(),
                None,
                custos_domain::RiskClass::Low,
                60,
            ))
        }
        async fn consume_permit(&self, permit_id: &str, intent: &ActionIntent) -> Result<custos_domain::Permit, DomainError> {
            let mut p = custos_domain::Permit::new(
                intent.task_id.clone().unwrap_or_else(|| "task_default".into()),
                intent.id.clone(),
                "sandbox".into(),
                None,
                custos_domain::RiskClass::Low,
                60,
            );
            p.id = permit_id.to_string();
            Ok(p)
        }
        async fn verify_evidence(&self, _: &custos_core::evidence::EvidenceBundle) -> Result<custos_domain::VerificationClaim, DomainError> { unreachable!() }
        async fn grant_capability(&self, _: &str, _: &str, _: &str) -> Result<custos_domain::Grant, DomainError> { unreachable!() }
        async fn find_active_grant(&self, _: &str, _: &str) -> Result<Option<custos_domain::Grant>, DomainError> { Ok(None) }
        async fn complete_task(&self, _: &str, _: String, _: Vec<custos_domain::VerificationClaim>) -> Result<custos_domain::Task, DomainError> { unreachable!() }
    }

    #[tokio::test]
    async fn test_worker_executor_durable_effect_ledger_and_outbox_integration() {
        let store = Arc::new(MockLedgerAndOutbox::new());
        let executor = WorkerExecutor::new()
            .with_kernel(Arc::new(WorkerTestKernel))
            .with_sandbox(Arc::new(WorkerTestSandbox))
            .with_effect_ledger(store.clone())
            .with_outbox(store.clone());

        let ctx = ExecutionContext::new("task_durable_01", "run_durable_01", "node_1", "worker_1");
        let inputs = serde_json::json!({
            "tool": "read_file",
            "parameters": { "path": "durable_test.txt" }
        });

        let obs = executor
            .execute_step_observation(&ctx, "tool_call", &inputs)
            .await
            .unwrap();

        match obs {
            crate::workflow::StepObservation::ToolExecuted { receipt, .. } => {
                assert_eq!(receipt.status, custos_domain::ReceiptStatus::Success);
            }
            other => panic!("Expected ToolExecuted observation, got {:?}", other),
        }

        // Verify EffectAttempt was recorded in ledger as Succeeded
        let effects = store.effects.lock().unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].status, EffectStatus::Succeeded);
        assert!(effects[0].receipt.is_some());

        // Verify OutboxEntry was processed to Receipted
        let outbox = store.outbox.lock().unwrap();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].status, OutboxStatus::Receipted);
    }

    #[tokio::test]
    async fn test_worker_executor_approval_conflict_yields_pending_observation() {
        struct ConflictKernel;
        #[async_trait]
        impl KernelPort for ConflictKernel {
            async fn create_task(&self, _: String) -> Result<custos_domain::Task, DomainError> { unreachable!() }
            async fn create_task_with_contract(&self, _: String, _: custos_domain::TaskContract) -> Result<custos_domain::Task, DomainError> { unreachable!() }
            async fn get_task(&self, _: &str) -> Result<Option<custos_domain::Task>, DomainError> { unreachable!() }
            async fn transition_task(&self, _: &str, _: custos_domain::TaskStatus) -> Result<custos_domain::Task, DomainError> { unreachable!() }
            async fn request_permit(&self, _: &str, _: &ActionIntent, _: &str) -> Result<custos_domain::Permit, DomainError> {
                Err(DomainError::Conflict("Approval required for high-risk write".into()))
            }
            async fn consume_permit(&self, _: &str, _: &ActionIntent) -> Result<custos_domain::Permit, DomainError> { unreachable!() }
            async fn verify_evidence(&self, _: &custos_core::evidence::EvidenceBundle) -> Result<custos_domain::VerificationClaim, DomainError> { unreachable!() }
            async fn grant_capability(&self, _: &str, _: &str, _: &str) -> Result<custos_domain::Grant, DomainError> { unreachable!() }
            async fn find_active_grant(&self, _: &str, _: &str) -> Result<Option<custos_domain::Grant>, DomainError> { unreachable!() }
            async fn complete_task(&self, _: &str, _: String, _: Vec<custos_domain::VerificationClaim>) -> Result<custos_domain::Task, DomainError> { unreachable!() }
        }

        let store = Arc::new(MockLedgerAndOutbox::new());
        let executor = WorkerExecutor::new()
            .with_kernel(Arc::new(ConflictKernel))
            .with_sandbox(Arc::new(WorkerTestSandbox))
            .with_outbox(store.clone());

        let ctx = ExecutionContext::new("task_conflict_01", "run_conflict_01", "node_conflict", "worker_1");
        let inputs = serde_json::json!({
            "tool": "delete_all",
            "parameters": { "path": "/" }
        });

        let obs = executor
            .execute_step_observation(&ctx, "tool_call", &inputs)
            .await
            .unwrap();

        match obs {
            crate::workflow::StepObservation::ToolPending { attempt_id } => {
                assert!(!attempt_id.is_empty());
            }
            other => panic!("Expected ToolPending observation, got {:?}", other),
        }

        // Verify outbox entry created in Pending state
        let outbox = store.outbox.lock().unwrap();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].status, OutboxStatus::Pending);
        assert_eq!(outbox[0].error_message.as_deref(), Some("Approval required for high-risk write"));
    }
}
