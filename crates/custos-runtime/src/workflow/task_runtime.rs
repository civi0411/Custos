use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::KernelPort;
use custos_core::contracts::sandbox::SandboxPort;
use custos_core::contracts::storage::{
    EffectLedgerPort, OutboxEntry, OutboxPort, OutboxStatus, RunPort,
};
use custos_core::contracts::workflow::WorkflowPort;
use custos_domain::{
    new_id, Assurance, CancelReceipt, ContextPack, ContinuationPacket, DomainError, Run, RunHandle,
    RunStatus, StartRunCommand, TaskStatus, WorkerRun,
};
use custos_provider::request::ProviderRequest;
use custos_provider::ModelPort;

pub struct TaskRuntime {
    kernel: Option<Arc<dyn KernelPort>>,
    model: Option<Arc<dyn ModelPort>>,
    harness: Option<Arc<dyn AgentRuntimePort>>,
    sandbox: Option<Arc<dyn SandboxPort>>,
    outbox: Option<Arc<dyn OutboxPort>>,
    effect_ledger: Option<Arc<dyn EffectLedgerPort>>,
    run_store: Option<Arc<dyn RunPort>>,
    active_runs: Arc<RwLock<HashMap<String, Run>>>,
    active_workers: Arc<RwLock<HashMap<String, WorkerRun>>>,
}

impl TaskRuntime {
    pub fn new() -> Self {
        Self {
            kernel: None,
            model: None,
            harness: None,
            sandbox: None,
            outbox: None,
            effect_ledger: None,
            run_store: None,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_ports(kernel: Arc<dyn KernelPort>, model: Arc<dyn ModelPort>) -> Self {
        Self {
            kernel: Some(kernel),
            model: Some(model),
            harness: None,
            sandbox: None,
            outbox: None,
            effect_ledger: None,
            run_store: None,
            active_runs: Arc::new(RwLock::new(HashMap::new())),
            active_workers: Arc::new(RwLock::new(HashMap::new())),
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

    pub fn with_run_store(mut self, run_store: Arc<dyn RunPort>) -> Self {
        self.run_store = Some(run_store);
        self
    }

    pub fn kernel(&self) -> Option<&Arc<dyn KernelPort>> {
        self.kernel.as_ref()
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

    pub fn run_store(&self) -> Option<&Arc<dyn RunPort>> {
        self.run_store.as_ref()
    }
}

impl Default for TaskRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowPort for TaskRuntime {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        // 1. Verify and drive Task lifecycle if KernelPort is available
        if let Some(ref kernel) = self.kernel {
            let task = kernel
                .get_task(&cmd.task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: cmd.task_id.clone(),
                })?;

            if task.status == TaskStatus::Draft {
                kernel.transition_task(&task.id, TaskStatus::Queued).await?;
                kernel.transition_task(&task.id, TaskStatus::Running).await?;
            } else if task.status == TaskStatus::Queued {
                kernel.transition_task(&task.id, TaskStatus::Running).await?;
            } else if task.status.is_terminal() {
                return Err(DomainError::InvalidStateTransition {
                    from: task.status.to_string(),
                    to: "running".into(),
                });
            }
        }

        // 2. Instantiate and activate Run
        let mut run = Run::new(cmd.task_id.clone(), 1);
        run.workflow_revision = cmd.workflow_revision;
        run.transition(RunStatus::Active)?;

        if let Some(ref store) = self.run_store {
            store.save_run(&run).await?;
        }

        // 3. Instantiate bounded WorkerRun
        let mut wrun = WorkerRun::new(cmd.task_id.clone(), cmd.actor.clone(), 3);
        wrun.run_id = Some(run.id.clone());
        wrun.transition(RunStatus::Active)?;

        if let Some(ref store) = self.run_store {
            store.save_worker_run(&wrun).await?;
        }

        // 4. Execute governed agent turn if Harness is provided
        if let Some(ref harness) = self.harness {
            let context_pack = ContextPack::new(
                new_id("ctx"),
                Vec::new(),
                0,
                "sha256:empty".into(),
            );

            let intents = harness.execute_turn(&wrun, &context_pack).await?;

            // Process generated action intents through Gate 4, Kernel, Outbox, and Sandbox
            for mut intent in intents {
                // Gate 4: Validate intent assurance against harness profile
                harness.profile().validate_intent_assurance(&intent)?;

                // If intent is custos-mediated and we have kernel and sandbox:
                if intent.assurance == Assurance::CustosMediated {
                    if let Some(ref kernel) = self.kernel {
                        // Ensure intent is explicitly bound to task
                        intent.task_id = Some(cmd.task_id.clone());

                        // Request permit (Gate 1 Budget, Gate 2 Policy, Gate 3 Idempotency)
                        let permit = kernel.request_permit(&cmd.task_id, &intent, &cmd.actor).await?;
                        let burned_permit = kernel.consume_permit(&permit.id, &intent).await?;

                        let idempotency_key = intent
                            .idempotency_key
                            .clone()
                            .unwrap_or_else(|| intent.id.clone());

                        // Record effect attempt in effect ledger if available
                        let effect_id = if let Some(ref ledger) = self.effect_ledger {
                            let mut effect = custos_domain::EffectAttempt::new(
                                wrun.id.clone(),
                                burned_permit.id.clone(),
                                idempotency_key.clone(),
                            );
                            effect.status = custos_domain::EffectStatus::InFlight;
                            ledger.record_effect(&effect).await?;
                            Some(effect.id)
                        } else {
                            None
                        };

                        // T3: Transactional Outbox write BEFORE execution
                        let outbox_id = new_id("outbox");
                        if let Some(ref outbox) = self.outbox {
                            let outbox_entry = OutboxEntry {
                                id: outbox_id.clone(),
                                task_id: cmd.task_id.clone(),
                                action_id: intent.id.clone(),
                                permit_id: burned_permit.id.clone(),
                                argument_digest: intent.argument_digest(),
                                idempotency_key: Some(idempotency_key),
                                status: OutboxStatus::Pending,
                                created_at: Utc::now(),
                                receipt: None,
                            };
                            outbox.enqueue(outbox_entry).await?;
                            outbox.mark_dispatching(&outbox_id).await?;
                        }

                        // T4: Execute in Sandbox
                        if let Some(ref sandbox) = self.sandbox {
                            let receipt = sandbox.execute(&intent, &burned_permit).await?;

                            // Update Outbox and EffectLedger with verified receipt
                            if let Some(ref outbox) = self.outbox {
                                outbox.mark_receipted(&outbox_id, receipt.clone()).await?;
                            }
                            if let Some(ref ledger) = self.effect_ledger {
                                if let Some(eff_id) = effect_id {
                                    ledger
                                        .update_effect_status(
                                            &eff_id,
                                            custos_domain::EffectStatus::Succeeded,
                                            Some(&receipt),
                                        )
                                        .await?;
                                }
                            }
                        }
                    }
                } else if let Some(ref ledger) = self.effect_ledger {
                    // For observe-only or provider-governed effects, record observation in ledger
                    let idempotency_key = intent
                        .idempotency_key
                        .clone()
                        .unwrap_or_else(|| intent.id.clone());
                    let mut effect = custos_domain::EffectAttempt::new(
                        wrun.id.clone(),
                        "native-unmediated",
                        idempotency_key,
                    );
                    effect.status = custos_domain::EffectStatus::Succeeded;
                    effect.ended_at = Some(Utc::now());
                    let _ = ledger.record_effect(&effect).await;
                }
            }
        } else if let Some(ref model) = self.model {
            // Fallback to legacy/simple ModelPort turn generation
            let req = ProviderRequest::new(
                new_id("req"),
                &cmd.task_id,
                1,
                format!("Initial run turn for task {}", cmd.task_id),
                model.provider_id(),
            );
            let _ = model.generate(&req).await?;
        }

        let handle = RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
        };

        // 5. Register active executions
        self.active_runs.write().await.insert(run.id.clone(), run);
        self.active_workers.write().await.insert(wrun.id.clone(), wrun);

        Ok(handle)
    }

    async fn request_cancel(&self, run_id: &str, reason: &str) -> Result<CancelReceipt, DomainError> {
        let run = {
            let mut runs = self.active_runs.write().await;
            if let Some(r) = runs.get_mut(run_id) {
                r.transition(RunStatus::Cancelled)?;
                r.clone()
            } else if let Some(ref store) = self.run_store {
                let mut r = store.get_run(run_id).await?.ok_or_else(|| DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                })?;
                r.transition(RunStatus::Cancelled)?;
                r
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        if let Some(ref store) = self.run_store {
            let _ = store.save_run(&run).await;
        }

        if let Some(ref harness) = self.harness {
            let _ = harness.cancel_run(run_id).await;
        }

        if let Some(ref kernel) = self.kernel {
            let _ = kernel.transition_task(&run.task_id, TaskStatus::Cancelled).await;
        }

        let uncertain_count = if let Some(ref outbox) = self.outbox {
            let uncertain = outbox.list_by_status(OutboxStatus::Uncertain).await.unwrap_or_default();
            let dispatching = outbox.list_by_status(OutboxStatus::Dispatching).await.unwrap_or_default();
            (uncertain.len() + dispatching.len()) as u32
        } else {
            0
        };

        Ok(CancelReceipt {
            run_id: run_id.to_string(),
            cancelled_at: Utc::now(),
            reason: reason.to_string(),
            uncertain_effects_count: uncertain_count,
        })
    }

    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError> {
        let run = {
            let mut runs = self.active_runs.write().await;
            if let Some(r) = runs.get_mut(run_id) {
                if r.status == RunStatus::Suspended {
                    r.transition(RunStatus::Active)?;
                }
                r.clone()
            } else if let Some(ref store) = self.run_store {
                let mut r = store.get_run(run_id).await?.ok_or_else(|| DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                })?;
                if r.status == RunStatus::Suspended {
                    r.transition(RunStatus::Active)?;
                }
                r
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        if let Some(ref store) = self.run_store {
            let _ = store.save_run(&run).await;
        }

        Ok(RunHandle {
            run_id: run.id.clone(),
            task_id: run.task_id.clone(),
            status: run.status,
            started_at: run.started_at,
        })
    }

    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError> {
        let run = {
            let runs = self.active_runs.read().await;
            if let Some(r) = runs.get(run_id) {
                r.clone()
            } else if let Some(ref store) = self.run_store {
                store.get_run(run_id).await?.ok_or_else(|| DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                })?
            } else {
                return Err(DomainError::NotFound {
                    kind: "Run".into(),
                    id: run_id.to_string(),
                });
            }
        };

        ContinuationPacket::create(
            run.task_id.clone(),
            run.current_span_num,
            run.current_span_num + 1,
            "custos-runtime".into(),
            "default".into(),
            format!("Checkpoint for run {run_id}"),
            serde_json::json!({
                "run_id": run.id,
                "status": run.status.to_string(),
                "attempt": run.attempt,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_runtime_lifecycle() {
        let runtime = TaskRuntime::new();
        let cmd = StartRunCommand::new("task_test_123", "test_actor");
        let handle = runtime.start_run(cmd).await.unwrap();
        assert_eq!(handle.task_id, "task_test_123");
        assert_eq!(handle.status, RunStatus::Active);

        // Checkpoint
        let cp = runtime.checkpoint(&handle.run_id).await.unwrap();
        assert_eq!(cp.task_id, "task_test_123");

        // Cancel
        let receipt = runtime.request_cancel(&handle.run_id, "user request").await.unwrap();
        assert_eq!(receipt.run_id, handle.run_id);
        assert_eq!(receipt.reason, "user request");
    }
}
