use super::*;
use crate::workflow::contracts::ExecutionContext;

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
        receipt: Option<&custos_contracts::ExecutionReceipt>,
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
        Ok(self
            .effects
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.idempotency_key == key)
            .cloned())
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
        Ok(self
            .outbox
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.id == id)
            .cloned())
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
    async fn mark_receipted(
        &self,
        id: &str,
        receipt: custos_contracts::ExecutionReceipt,
    ) -> Result<(), DomainError> {
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
        Ok(self
            .outbox
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.status == status)
            .cloned()
            .collect())
    }
    async fn list_by_task_and_status(
        &self,
        task_id: &str,
        status: OutboxStatus,
    ) -> Result<Vec<OutboxEntry>, DomainError> {
        Ok(self
            .outbox
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.task_id == task_id && e.status == status)
            .cloned()
            .collect())
    }
}

struct WorkerTestSandbox;
#[async_trait]
impl SandboxPort for WorkerTestSandbox {
    fn driver_id(&self) -> &str {
        "test"
    }
    async fn execute(
        &self,
        action: &ActionIntent,
        permit: &custos_contracts::Permit,
    ) -> Result<custos_contracts::ExecutionReceipt, DomainError> {
        Ok(custos_contracts::ExecutionReceipt {
            receipt_id: "rcpt_test".into(),
            permit_id: permit.id.clone(),
            action_id: action.id.clone(),
            status: custos_contracts::ReceiptStatus::Success,
            output_digest: "sha256:00".into(),
            output_data: Some(serde_json::json!({ "success": true })),
            error_message: None,
            duration_ms: Some(1),
            executed_at: chrono::Utc::now(),
            assurance: custos_contracts::Assurance::CustosMediated,
        })
    }
}

struct WorkerTestKernel;
#[async_trait]
impl KernelPort for WorkerTestKernel {
    async fn create_task(&self, _: String) -> Result<custos_contracts::Task, DomainError> {
        unreachable!()
    }
    async fn create_task_with_contract(
        &self,
        _: String,
        _: custos_contracts::TaskContract,
    ) -> Result<custos_contracts::Task, DomainError> {
        unreachable!()
    }
    async fn get_task(&self, _: &str) -> Result<Option<custos_contracts::Task>, DomainError> {
        unreachable!()
    }
    async fn transition_task(
        &self,
        _: &str,
        _: custos_contracts::TaskStatus,
    ) -> Result<custos_contracts::Task, DomainError> {
        unreachable!()
    }
    async fn request_permit(
        &self,
        task_id: &str,
        intent: &ActionIntent,
        _: &str,
    ) -> Result<custos_contracts::Permit, DomainError> {
        Ok(custos_contracts::Permit::new(
            task_id.into(),
            intent.id.clone(),
            "sandbox".into(),
            None,
            custos_contracts::RiskClass::Low,
            60,
        ))
    }
    async fn consume_permit(
        &self,
        permit_id: &str,
        intent: &ActionIntent,
    ) -> Result<custos_contracts::Permit, DomainError> {
        let mut p = custos_contracts::Permit::new(
            intent
                .task_id
                .clone()
                .unwrap_or_else(|| "task_default".into()),
            intent.id.clone(),
            "sandbox".into(),
            None,
            custos_contracts::RiskClass::Low,
            60,
        );
        p.id = permit_id.to_string();
        Ok(p)
    }
    async fn verify_evidence(
        &self,
        _: &custos_contracts::evidence::EvidenceBundle,
    ) -> Result<custos_contracts::VerificationClaim, DomainError> {
        unreachable!()
    }
    async fn grant_capability(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<custos_contracts::Grant, DomainError> {
        unreachable!()
    }
    async fn find_active_grant(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<custos_contracts::Grant>, DomainError> {
        Ok(None)
    }
    async fn complete_task(
        &self,
        _: &str,
        _: String,
        _: Vec<custos_contracts::VerificationClaim>,
    ) -> Result<custos_contracts::Task, DomainError> {
        unreachable!()
    }
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
            assert_eq!(receipt.status, custos_contracts::ReceiptStatus::Success);
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
        async fn create_task(&self, _: String) -> Result<custos_contracts::Task, DomainError> {
            unreachable!()
        }
        async fn create_task_with_contract(
            &self,
            _: String,
            _: custos_contracts::TaskContract,
        ) -> Result<custos_contracts::Task, DomainError> {
            unreachable!()
        }
        async fn get_task(&self, _: &str) -> Result<Option<custos_contracts::Task>, DomainError> {
            unreachable!()
        }
        async fn transition_task(
            &self,
            _: &str,
            _: custos_contracts::TaskStatus,
        ) -> Result<custos_contracts::Task, DomainError> {
            unreachable!()
        }
        async fn request_permit(
            &self,
            _: &str,
            _: &ActionIntent,
            _: &str,
        ) -> Result<custos_contracts::Permit, DomainError> {
            Err(DomainError::Conflict(
                "Approval required for high-risk write".into(),
            ))
        }
        async fn consume_permit(
            &self,
            _: &str,
            _: &ActionIntent,
        ) -> Result<custos_contracts::Permit, DomainError> {
            unreachable!()
        }
        async fn verify_evidence(
            &self,
            _: &custos_contracts::evidence::EvidenceBundle,
        ) -> Result<custos_contracts::VerificationClaim, DomainError> {
            unreachable!()
        }
        async fn grant_capability(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<custos_contracts::Grant, DomainError> {
            unreachable!()
        }
        async fn find_active_grant(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<custos_contracts::Grant>, DomainError> {
            unreachable!()
        }
        async fn complete_task(
            &self,
            _: &str,
            _: String,
            _: Vec<custos_contracts::VerificationClaim>,
        ) -> Result<custos_contracts::Task, DomainError> {
            unreachable!()
        }
    }

    let store = Arc::new(MockLedgerAndOutbox::new());
    let executor = WorkerExecutor::new()
        .with_kernel(Arc::new(ConflictKernel))
        .with_sandbox(Arc::new(WorkerTestSandbox))
        .with_outbox(store.clone());

    let ctx = ExecutionContext::new(
        "task_conflict_01",
        "run_conflict_01",
        "node_conflict",
        "worker_1",
    );
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
    assert_eq!(
        outbox[0].error_message.as_deref(),
        Some("Approval required for high-risk write")
    );
}
