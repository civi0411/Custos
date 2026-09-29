use async_trait::async_trait;
use custos_core_domain::{ContinuationPacket, DomainError};
use custos_kernel::TaskStore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub cursor: usize,
    pub data: Value,
    pub results: Vec<Value>,
    pub pending_effect: Option<Value>,
    pub completed: bool,
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self {
            cursor: 0,
            data: Value::Null,
            results: Vec::new(),
            pending_effect: None,
            completed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationOutcome {
    NotApplicable,
    Applied(Value),
    Yield(Value),
}

#[async_trait]
pub trait WorkflowOperation: Send + Sync {
    fn name(&self) -> &'static str;
    async fn evaluate(&self, state: &WorkflowState) -> Result<OperationOutcome, DomainError>;
}

#[async_trait]
pub trait EffectExecutor: Send + Sync {
    async fn execute(&self, effect: &Value, idempotency_key: &str) -> Result<Value, DomainError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowStep {
    Advanced {
        operation: String,
    },
    EffectRequested {
        operation: String,
        idempotency_key: String,
    },
    EffectCompleted {
        idempotency_key: String,
    },
    Completed,
}

pub struct WorkflowMachine {
    task_id: String,
    provider: String,
    model: String,
    store: Arc<dyn TaskStore>,
    operations: Vec<Arc<dyn WorkflowOperation>>,
    effects: Arc<dyn EffectExecutor>,
}

impl WorkflowMachine {
    pub fn new(
        task_id: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
        store: Arc<dyn TaskStore>,
        operations: Vec<Arc<dyn WorkflowOperation>>,
        effects: Arc<dyn EffectExecutor>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            provider: provider.into(),
            model: model.into(),
            store,
            operations,
            effects,
        }
    }

    pub async fn step(&self) -> Result<WorkflowStep, DomainError> {
        let mut state = self.load_state().await?;
        if state.completed {
            return Ok(WorkflowStep::Completed);
        }

        if let Some(effect) = state.pending_effect.take() {
            let key = self.effect_key(state.cursor, &effect)?;
            let receipt = self.effects.execute(&effect, &key).await?;
            state.results.push(receipt);
            state.cursor += 1;
            self.save_state(&state).await?;
            tracing::info!(task_id = %self.task_id, effect_key = %key, "workflow effect completed");
            return Ok(WorkflowStep::EffectCompleted {
                idempotency_key: key,
            });
        }

        if state.cursor >= self.operations.len() {
            state.completed = true;
            self.save_state(&state).await?;
            return Ok(WorkflowStep::Completed);
        }

        let operation = &self.operations[state.cursor];
        let name = operation.name().to_string();
        match operation.evaluate(&state).await? {
            OperationOutcome::NotApplicable => {
                state.cursor += 1;
                self.save_state(&state).await?;
                Ok(WorkflowStep::Advanced { operation: name })
            }
            OperationOutcome::Applied(result) => {
                state.results.push(result);
                state.cursor += 1;
                self.save_state(&state).await?;
                Ok(WorkflowStep::Advanced { operation: name })
            }
            OperationOutcome::Yield(effect) => {
                let key = self.effect_key(state.cursor, &effect)?;
                state.pending_effect = Some(effect);
                self.save_state(&state).await?;
                Ok(WorkflowStep::EffectRequested {
                    operation: name,
                    idempotency_key: key,
                })
            }
        }
    }

    async fn load_state(&self) -> Result<WorkflowState, DomainError> {
        let Some(latest) = self.store.get_latest_continuation(&self.task_id).await? else {
            return Ok(WorkflowState::default());
        };
        let mut selected = None;
        for sequence in (1..=latest.to_span).rev() {
            let Some(packet) = self.store.get_continuation(&self.task_id, sequence).await? else {
                continue;
            };
            packet.verify()?;
            if let Some(checkpoint) = packet.current_state.get("custos_workflow_v1").cloned() {
                selected = Some((packet, checkpoint));
                break;
            }
        }
        let Some((packet, checkpoint)) = selected else {
            return Ok(WorkflowState::default());
        };
        let state: WorkflowState = serde_json::from_value(checkpoint).map_err(|error| {
            DomainError::Validation(format!("Invalid workflow checkpoint: {error}"))
        })?;
        if state.cursor > self.operations.len() {
            return Err(DomainError::InvariantViolation(
                "workflow checkpoint cursor exceeds operation plan".into(),
            ));
        }
        if state.pending_effect.is_some() && state.completed {
            return Err(DomainError::InvariantViolation(
                "completed workflow cannot contain a pending effect".into(),
            ));
        }
        // Keep the selected packet alive until all validation is complete so the
        // checkpoint source is explicit during future schema evolution.
        let _checkpoint_packet = packet;
        Ok(state)
    }

    async fn save_state(&self, state: &WorkflowState) -> Result<(), DomainError> {
        let previous = self.store.get_latest_continuation(&self.task_id).await?;
        let from_span = match previous.as_ref() {
            Some(packet) => packet.to_span,
            None => 0,
        };
        let to_span = from_span.checked_add(1).ok_or_else(|| {
            DomainError::InvariantViolation("workflow checkpoint sequence exhausted".into())
        })?;
        let packet = ContinuationPacket::create(
            self.task_id.clone(),
            from_span,
            to_span,
            self.provider.clone(),
            self.model.clone(),
            format!("workflow checkpoint at operation {}", state.cursor),
            serde_json::json!({
                "custos_workflow_v1": serde_json::to_value(state)
                    .map_err(|error| DomainError::Validation(error.to_string()))?
            }),
        )?;
        let current = self.store.get_latest_continuation(&self.task_id).await?;
        if current.as_ref().map(|packet| packet.to_span)
            != previous.as_ref().map(|packet| packet.to_span)
        {
            return Err(DomainError::Conflict(
                "workflow checkpoint changed concurrently; retry the step".into(),
            ));
        }
        self.store.save_continuation(&packet).await
    }

    fn effect_key(&self, cursor: usize, effect: &Value) -> Result<String, DomainError> {
        let canonical = custos_core_domain::canonical_json(effect)
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        Ok(custos_core_domain::digest(
            format!("{}:{cursor}:{canonical}", self.task_id).as_bytes(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core_domain::{ContinuationPacket, Span, Task};
    use custos_kernel::TaskEvent;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryStore {
        continuations: Mutex<Vec<ContinuationPacket>>,
        fail_next_save: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl TaskStore for MemoryStore {
        async fn get_task(&self, _task_id: &str) -> Result<Option<Task>, DomainError> {
            Ok(None)
        }
        async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
            Ok(Vec::new())
        }
        async fn save_task(&self, _task: &Task) -> Result<(), DomainError> {
            Ok(())
        }
        async fn commit_task_event(
            &self,
            _event: &TaskEvent,
            _task: &Task,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_task_events(&self, _task_id: &str) -> Result<Vec<TaskEvent>, DomainError> {
            Ok(Vec::new())
        }
        async fn get_span(&self, _span_id: &str) -> Result<Option<Span>, DomainError> {
            Ok(None)
        }
        async fn save_span(&self, _span: &Span) -> Result<(), DomainError> {
            Ok(())
        }
        async fn list_spans(&self, _task_id: &str) -> Result<Vec<Span>, DomainError> {
            Ok(Vec::new())
        }
        async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError> {
            if self.fail_next_save.swap(false, Ordering::SeqCst) {
                return Err(DomainError::Validation(
                    "injected checkpoint write failure".into(),
                ));
            }
            self.continuations
                .lock()
                .map_err(|_| DomainError::InvariantViolation("test store lock poisoned".into()))?
                .push(packet.clone());
            Ok(())
        }
        async fn get_continuation(
            &self,
            task_id: &str,
            to_span: u32,
        ) -> Result<Option<ContinuationPacket>, DomainError> {
            let packets = self
                .continuations
                .lock()
                .map_err(|_| DomainError::InvariantViolation("test store lock poisoned".into()))?;
            Ok(packets
                .iter()
                .find(|packet| packet.task_id == task_id && packet.to_span == to_span)
                .cloned())
        }
        async fn get_latest_continuation(
            &self,
            task_id: &str,
        ) -> Result<Option<ContinuationPacket>, DomainError> {
            let packets = self
                .continuations
                .lock()
                .map_err(|_| DomainError::InvariantViolation("test store lock poisoned".into()))?;
            Ok(packets
                .iter()
                .filter(|packet| packet.task_id == task_id)
                .max_by_key(|packet| packet.to_span)
                .cloned())
        }
    }

    struct YieldEffect;

    #[async_trait]
    impl WorkflowOperation for YieldEffect {
        fn name(&self) -> &'static str {
            "yield_effect"
        }
        async fn evaluate(&self, _state: &WorkflowState) -> Result<OperationOutcome, DomainError> {
            Ok(OperationOutcome::Yield(
                serde_json::json!({"kind": "test_effect"}),
            ))
        }
    }

    #[derive(Default)]
    struct IdempotentExecutor {
        actions: AtomicUsize,
        receipts: Mutex<HashMap<String, Value>>,
    }

    #[async_trait]
    impl EffectExecutor for IdempotentExecutor {
        async fn execute(
            &self,
            _effect: &Value,
            idempotency_key: &str,
        ) -> Result<Value, DomainError> {
            let mut receipts = self.receipts.lock().map_err(|_| {
                DomainError::InvariantViolation("test executor lock poisoned".into())
            })?;
            if let Some(receipt) = receipts.get(idempotency_key) {
                return Ok(receipt.clone());
            }
            self.actions.fetch_add(1, Ordering::SeqCst);
            let receipt = serde_json::json!({"accepted": true});
            receipts.insert(idempotency_key.to_string(), receipt.clone());
            Ok(receipt)
        }
    }

    fn machine(store: Arc<dyn TaskStore>, effects: Arc<IdempotentExecutor>) -> WorkflowMachine {
        WorkflowMachine::new(
            "task-resume",
            "fake",
            "test",
            store,
            vec![Arc::new(YieldEffect)],
            effects,
        )
    }

    #[tokio::test]
    async fn yielded_effect_resumes_from_persisted_checkpoint() {
        let store: Arc<dyn TaskStore> = Arc::new(MemoryStore::default());
        let effects = Arc::new(IdempotentExecutor::default());

        let first_process = machine(store.clone(), effects.clone());
        let requested = first_process.step().await.unwrap();
        assert!(matches!(requested, WorkflowStep::EffectRequested { .. }));

        let restarted_process = machine(store, effects.clone());
        let completed = restarted_process.step().await.unwrap();
        assert!(matches!(completed, WorkflowStep::EffectCompleted { .. }));
        assert!(matches!(
            restarted_process.step().await.unwrap(),
            WorkflowStep::Completed
        ));
        assert_eq!(effects.actions.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn effect_retry_reuses_idempotency_key_after_receipt_write_failure() {
        let store_impl = Arc::new(MemoryStore::default());
        let store: Arc<dyn TaskStore> = store_impl.clone();
        let effects = Arc::new(IdempotentExecutor::default());
        let first_process = machine(store.clone(), effects.clone());

        first_process.step().await.unwrap();
        store_impl.fail_next_save.store(true, Ordering::SeqCst);
        assert!(first_process.step().await.is_err());

        let restarted_process = machine(store, effects.clone());
        assert!(matches!(
            restarted_process.step().await.unwrap(),
            WorkflowStep::EffectCompleted { .. }
        ));
        assert!(matches!(
            restarted_process.step().await.unwrap(),
            WorkflowStep::Completed
        ));
        assert_eq!(effects.actions.load(Ordering::SeqCst), 1);
    }
}
