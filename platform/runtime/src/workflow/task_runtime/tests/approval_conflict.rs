use super::*;
use custos_domain::StartRunCommand;

#[tokio::test]
async fn test_native_turn_approval_conflict_suspends_run_and_blocks_task() {
    use custos_core::contracts::harness::{AgentRuntimePort, HarnessProfile, ToolMediationLevel};
    use custos_core::contracts::kernel::KernelPort;
    use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
    use custos_domain::{
        Action, ActionLifecycleState, Assurance, ContextPack, RiskLevel, TaskStatus, WorkerRun,
    };
    use tokio::sync::Mutex;

    struct ConflictHarness;
    #[async_trait::async_trait]
    impl AgentRuntimePort for ConflictHarness {
        fn harness_id(&self) -> &str {
            "conflict-harness"
        }
        fn profile(&self) -> HarnessProfile {
            HarnessProfile::new("conflict-harness", ToolMediationLevel::CustosMediated)
        }
        async fn execute_turn(
            &self,
            worker_run: &WorkerRun,
            _ctx: &ContextPack,
        ) -> Result<Vec<custos_domain::ActionIntent>, DomainError> {
            let mut intent = Action::new(
                "act_delete".into(),
                "delete_database".into(),
                "prod.db".into(),
                serde_json::json!({}),
                RiskLevel::High,
            );
            intent.task_id = Some(worker_run.task_id.clone());
            intent.assurance = Assurance::CustosMediated;
            intent.lifecycle_state = ActionLifecycleState::Intent;
            Ok(vec![intent])
        }
    }

    struct TaskStatusKernel {
        task_status: Arc<Mutex<TaskStatus>>,
    }
    #[async_trait::async_trait]
    impl KernelPort for TaskStatusKernel {
        async fn create_task(&self, _: String) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn create_task_with_contract(
            &self,
            _: String,
            _: custos_domain::TaskContract,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
        async fn get_task(
            &self,
            task_id: &str,
        ) -> Result<Option<custos_domain::Task>, DomainError> {
            let status = *self.task_status.lock().await;
            let mut t = custos_domain::Task::new(task_id.into(), "Conflict".into());
            t.status = status;
            Ok(Some(t))
        }
        async fn transition_task(
            &self,
            _: &str,
            next: TaskStatus,
        ) -> Result<custos_domain::Task, DomainError> {
            let mut status = self.task_status.lock().await;
            *status = next;
            let mut t = custos_domain::Task::new("task_conflict_02".into(), "Conflict".into());
            t.status = next;
            Ok(t)
        }
        async fn request_permit(
            &self,
            _: &str,
            _: &custos_domain::ActionIntent,
            _: &str,
        ) -> Result<custos_domain::Permit, DomainError> {
            Err(DomainError::Conflict(
                "Approval required for high risk delete".into(),
            ))
        }
        async fn consume_permit(
            &self,
            _: &str,
            _: &custos_domain::ActionIntent,
        ) -> Result<custos_domain::Permit, DomainError> {
            unreachable!()
        }
        async fn verify_evidence(
            &self,
            _: &custos_core::evidence::EvidenceBundle,
        ) -> Result<custos_domain::VerificationClaim, DomainError> {
            unreachable!()
        }
        async fn grant_capability(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<custos_domain::Grant, DomainError> {
            unreachable!()
        }
        async fn find_active_grant(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<custos_domain::Grant>, DomainError> {
            unreachable!()
        }
        async fn complete_task(
            &self,
            _: &str,
            _: String,
            _: Vec<custos_domain::VerificationClaim>,
        ) -> Result<custos_domain::Task, DomainError> {
            unreachable!()
        }
    }

    struct InMemoryOutbox {
        entries: Arc<Mutex<Vec<OutboxEntry>>>,
    }
    #[async_trait::async_trait]
    impl OutboxPort for InMemoryOutbox {
        async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
            self.entries.lock().await.push(entry);
            Ok(())
        }
        async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError> {
            Ok(self
                .entries
                .lock()
                .await
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }
        async fn list_by_status(
            &self,
            status: OutboxStatus,
        ) -> Result<Vec<OutboxEntry>, DomainError> {
            Ok(self
                .entries
                .lock()
                .await
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
                .entries
                .lock()
                .await
                .iter()
                .filter(|e| e.task_id == task_id && e.status == status)
                .cloned()
                .collect())
        }
        async fn mark_approved(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_dispatching(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_receipted(
            &self,
            _: &str,
            _: custos_domain::Receipt,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_uncertain(&self, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn mark_failed(&self, _: &str, _: &str) -> Result<(), DomainError> {
            Ok(())
        }
    }

    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_domain::Task::new("task_conflict_02".into(), "Conflict Test".into());
    custos_core::TaskStore::save_task(&*store, &task)
        .await
        .unwrap();

    let task_status = Arc::new(Mutex::new(TaskStatus::Running));
    let kernel = Arc::new(TaskStatusKernel {
        task_status: task_status.clone(),
    });
    let outbox_entries = Arc::new(Mutex::new(Vec::new()));
    let outbox = Arc::new(InMemoryOutbox {
        entries: outbox_entries.clone(),
    });

    let runtime = TaskRuntime::new()
        .with_harness(Arc::new(ConflictHarness))
        .with_kernel(kernel)
        .with_outbox(outbox)
        .with_run_store(store);

    let cmd = StartRunCommand::new(&task.id, "actor_user")
        .with_session_id("session_conflict_02")
        .with_preferred_mode("native")
        .with_harness_id("conflict-harness")
        .with_workspace_root("/tmp/custos-conflict-test");

    let handle = runtime.start_run(cmd).await.unwrap();
    // Crucial regression assert: MUST NOT be Completed!
    assert_eq!(
        handle.status,
        RunStatus::Suspended,
        "Pending approval must suspend run, not complete it"
    );
    assert_eq!(
        *task_status.lock().await,
        TaskStatus::Blocked,
        "Task must transition to Blocked"
    );

    {
        let entries = outbox_entries.lock().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, OutboxStatus::Pending);
    }

    // Test 1: live_run replay after daemon restart must preserve suspended status, not report "interrupted"
    let replay = runtime
        .live_run("session_conflict_02", &handle.run_id, 0)
        .await
        .unwrap();
    assert_eq!(
        replay.snapshot.status, "suspended",
        "Replay must preserve suspended status"
    );
    assert_eq!(
        replay.snapshot.error,
        Some("Approval required for high risk delete".into()),
        "Replay must preserve pending approval reason"
    );

    // Test 2: resume before approval must fail with conflict
    let early_resume = runtime.resume(&handle.run_id).await;
    assert!(
        early_resume.is_err(),
        "Resume must fail while approval is still pending"
    );
    assert!(matches!(
        early_resume.unwrap_err(),
        DomainError::Conflict(_)
    ));

    // Test 3: mark approval granted with valid execution receipt -> resume transitions to Active and task to Running
    {
        let mut entries = outbox_entries.lock().await;
        entries[0].status = OutboxStatus::Approved;
        let entry_id = entries[0].id.clone();
        entries[0].receipt = Some(custos_domain::ExecutionReceipt {
            receipt_id: format!("rcpt_{entry_id}"),
            permit_id: format!("pmt_{entry_id}"),
            action_id: "test_action".into(),
            status: custos_domain::ReceiptStatus::Success,
            output_digest: "sha256:executed".into(),
            output_data: Some(serde_json::json!({"action": "executed"})),
            error_message: None,
            duration_ms: Some(10),
            executed_at: chrono::Utc::now(),
            assurance: custos_domain::Assurance::CustosMediated,
        });
    }
    let resumed = runtime.resume(&handle.run_id).await.unwrap();
    assert_eq!(
        resumed.status,
        RunStatus::Active,
        "Run must become Active upon approval resume"
    );
    assert_eq!(
        *task_status.lock().await,
        TaskStatus::Running,
        "Task must transition to Running upon approval resume"
    );

    // Test 4: background continuation executor completes the approved execution
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(
        *task_status.lock().await,
        TaskStatus::Succeeded,
        "Task must succeed after resumed approval execution"
    );
}

