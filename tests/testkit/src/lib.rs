//! Custos Testkit — mocks for every port + a vertical-slice `Harness`.
//!
//! Rule (TEAM_WORK_ALLOCATION §1.5): each member ships the mock of THEIR port so the others
//! can build in parallel.
//!   Vĩ     -> `MockModel`, `InMemoryMemory`
//!   Trường -> in-memory SQLite (`SqliteTaskStore::new_in_memory`), `InMemoryOutbox`
//!   Vinh   -> `MockSandbox`
//! Nothing here touches the network, spawns a process, or needs a database file.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_core::contracts::sandbox::verify_permit_binding;
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::contracts::{
    MemoryPort, OutboxEntry, OutboxPort, OutboxStatus, SandboxCommand, SandboxPort, TrustedKernel,
};
use custos_domain::{
    digest, new_id, ActionIntent, CancelReceipt, ContinuationPacket, DomainError, ExecutionReceipt,
    FactProposal, MemoryEntry, MemoryError, MemoryScope, Permit, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, ReceiptStatus, RunHandle, StartRunCommand, WorkerRunId,
};
use custos_persistence::SqliteTaskStore;
use custos_provider::request::{ModelResponse, ProviderRequest};
use custos_provider::ModelProvider;

// ───────────────────────────── ModelPort mock (Vĩ) ─────────────────────────────

/// Replays queued replies in FIFO order; falls back to a fixed string when empty.
#[derive(Default)]
pub struct MockModel {
    replies: Mutex<VecDeque<String>>,
    pub seen_prompts: Mutex<Vec<String>>,
}

impl MockModel {
    pub fn push_reply(&self, reply: impl Into<String>) {
        self.replies.lock().unwrap().push_back(reply.into());
    }
}

#[async_trait]
impl ModelProvider for MockModel {
    fn provider_id(&self) -> &str {
        "mock"
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        self.seen_prompts.lock().unwrap().push(req.prompt.clone());
        let content = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| "mock-default".into());
        Ok(ModelResponse {
            tokens_used: content.len() / 4 + 1,
            content,
            model_id: "mock".into(),
            tool_calls: vec![],
        })
    }
}

// ──────────────────────────── SandboxPort mock (Vinh) ───────────────────────────

/// Never spawns anything. Enforces the same permit guard a real driver must.
#[derive(Default)]
pub struct MockSandbox {
    pub executed: Mutex<Vec<SandboxCommand>>,
}

#[async_trait]
impl SandboxPort for MockSandbox {
    fn driver_id(&self) -> &str {
        "mock"
    }

    async fn execute(
        &self,
        intent: &ActionIntent,
        permit: &Permit,
    ) -> Result<ExecutionReceipt, DomainError> {
        verify_permit_binding(permit, intent)?;
        let cmd = SandboxCommand::from_intent(intent)?;
        self.executed.lock().unwrap().push(cmd);
        Ok(ExecutionReceipt {
            receipt_id: new_id("rcpt"),
            permit_id: permit.id.clone(),
            action_id: intent.id.clone(),
            status: ReceiptStatus::Success,
            output_digest: format!("sha256:{}", digest(b"ok")),
            output_data: Some(serde_json::json!({ "exit_code": 0 })),
            error_message: None,
            duration_ms: Some(1),
            executed_at: Utc::now(),
            assurance: custos_domain::Assurance::CustosMediated,
        })
    }
}

// ─────────────────────────── MemoryPort in-memory (Vĩ) ──────────────────────────

#[derive(Default)]
pub struct InMemoryMemory {
    facts: Mutex<Vec<PersonalFact>>,
    pub proposals: Mutex<Vec<(WorkerRunId, FactProposal)>>,
}

impl InMemoryMemory {
    /// Test-only seeding. Production code can never do this through the port.
    pub fn seed_fact(&self, fact: PersonalFact) {
        self.facts.lock().unwrap().push(fact);
    }
}

#[async_trait]
impl MemoryPort for InMemoryMemory {
    async fn recall_context(
        &self,
        query: &RecallQuery,
        _scope: &MemoryScope,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let needle = query.text.to_lowercase();
        Ok(self
            .facts
            .lock()
            .unwrap()
            .iter()
            .filter(|f| {
                format!("{} {} {}", f.subject, f.predicate, f.object)
                    .to_lowercase()
                    .contains(&needle)
            })
            .take(query.limit)
            .map(|f| MemoryEntry {
                id: new_id("mem"),
                content: format!("{} {} {}", f.subject, f.predicate, f.object),
                source: "in-memory".into(),
                score: f.confidence,
                recorded_at: f.valid_from,
            })
            .collect())
    }

    async fn recall_temporal_fact(
        &self,
        subject: &str,
        predicate: &str,
        at_time: DateTime<Utc>,
    ) -> Result<Option<PersonalFact>, MemoryError> {
        Ok(self
            .facts
            .lock()
            .unwrap()
            .iter()
            .find(|f| {
                f.subject == subject
                    && f.predicate == predicate
                    && f.valid_from <= at_time
                    && f.valid_to.is_none_or(|end| at_time < end)
            })
            .cloned())
    }

    async fn propose_fact(
        &self,
        proposal: FactProposal,
        worker_run_id: WorkerRunId,
    ) -> Result<ProposalReceipt, MemoryError> {
        // Proposals never become facts here: acceptance is a Kernel decision.
        self.proposals
            .lock()
            .unwrap()
            .push((worker_run_id, proposal));
        Ok(ProposalReceipt {
            proposal_id: new_id("prop"),
            status: ProposalStatus::Pending,
        })
    }
}

// ───────────────────────── OutboxPort in-memory (Trường) ────────────────────────

#[derive(Default)]
pub struct InMemoryOutbox {
    entries: Mutex<Vec<OutboxEntry>>,
}

impl InMemoryOutbox {
    fn with_entry<F: FnOnce(&mut OutboxEntry)>(&self, id: &str, f: F) -> Result<(), DomainError> {
        let mut guard = self.entries.lock().unwrap();
        let entry = guard
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "OutboxEntry".into(),
                id: id.into(),
            })?;
        f(entry);
        Ok(())
    }
}

#[async_trait]
impl OutboxPort for InMemoryOutbox {
    async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }

    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
        self.with_entry(id, |e| e.status = OutboxStatus::Dispatching)
    }

    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError> {
        self.with_entry(id, |e| {
            e.status = OutboxStatus::Receipted;
            e.receipt = Some(receipt);
        })
    }

    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError> {
        self.with_entry(id, |e| e.status = OutboxStatus::Uncertain)
    }

    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError> {
        Ok(self
            .entries
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
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.task_id == task_id && e.status == status)
            .cloned()
            .collect())
    }
}

// ──────────────────────────── WorkflowPort mock (Vinh) ───────────────────────────

#[derive(Default)]
pub struct MockWorkflow {
    pub started_runs: Mutex<Vec<StartRunCommand>>,
    pub cancelled_runs: Mutex<Vec<(String, String)>>,
}

#[async_trait]
impl WorkflowPort for MockWorkflow {
    async fn start_run(&self, cmd: StartRunCommand) -> Result<RunHandle, DomainError> {
        let run_id = new_id("run");
        self.started_runs.lock().unwrap().push(cmd.clone());
        Ok(RunHandle::new(
            run_id,
            cmd.task_id,
            custos_domain::RunStatus::Active,
            Utc::now(),
        ))
    }

    async fn request_cancel(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<CancelReceipt, DomainError> {
        self.cancelled_runs
            .lock()
            .unwrap()
            .push((run_id.to_string(), reason.to_string()));
        Ok(CancelReceipt {
            run_id: run_id.to_string(),
            cancelled_at: Utc::now(),
            reason: reason.to_string(),
            uncertain_effects_count: 0,
        })
    }

    async fn resume(&self, run_id: &str) -> Result<RunHandle, DomainError> {
        Ok(RunHandle::new(
            run_id.to_string(),
            "resumed",
            custos_domain::RunStatus::Active,
            Utc::now(),
        ))
    }

    async fn checkpoint(&self, run_id: &str) -> Result<ContinuationPacket, DomainError> {
        ContinuationPacket::create(
            "task_mock".into(),
            0,
            1,
            "mock".into(),
            "mock".into(),
            format!("Checkpoint for {run_id}"),
            serde_json::json!({}),
        )
    }
}

// ─────────────────────────────── Vertical harness ───────────────────────────────

/// Everything wired in RAM. This is a miniature `custos-daemon` composition root:
/// swap any field for the real adapter and the same tests must still pass.
pub struct Harness {
    pub kernel: TrustedKernel,
    pub store: Arc<SqliteTaskStore>,
    pub model: Arc<MockModel>,
    pub sandbox: Arc<MockSandbox>,
    pub memory: Arc<InMemoryMemory>,
    pub outbox: Arc<InMemoryOutbox>,
    pub workflow: Arc<MockWorkflow>,
    pub workspace: PathBuf,
}

impl Harness {
    pub fn new() -> Self {
        let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite"));
        let workspace = std::env::temp_dir().join(new_id("custos_ws"));
        std::fs::create_dir_all(&workspace).expect("temp workspace");
        Self {
            kernel: TrustedKernel::new(store.clone()),
            store,
            model: Arc::new(MockModel::default()),
            sandbox: Arc::new(MockSandbox::default()),
            memory: Arc::new(InMemoryMemory::default()),
            outbox: Arc::new(InMemoryOutbox::default()),
            workflow: Arc::new(MockWorkflow::default()),
            workspace,
        }
    }

    pub fn write_file(&self, rel: &str, content: &str) {
        let path = self.workspace.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.workspace);
    }
}
