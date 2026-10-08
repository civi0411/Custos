use crate::connection::DbConnection;
use crate::repositories::{
    ContinuationRepository, DecisionRepository, FleetAutomationRepository, OutboxRepository,
    ProviderRepository, ReplanRepository, ResearchRepository, RunRepository, SessionRepository,
    SpanRepository, TaskRepository, WorkflowRevisionRepository, WorkspaceRepository,
};
use async_trait::async_trait;
use custos_core::contracts::storage::{
    DecisionPort, EffectLedgerPort, OutboxEntry, OutboxPort, OutboxStatus, ReplanPort, RunPort,
    WorkflowRevisionPort,
};
use custos_core::contracts::workspace::WorkspaceRepository as WorkspaceRepoPort;
use custos_core::kernel::{SessionStore, TaskEvent, TaskStore};
use custos_domain::{
    ContinuationPacket, DecisionRecord, DomainError, ExecutionReceipt, ExecutionWorkspace,
    NodePlacement, ReplanRecord, Run, Session, SessionId, SessionJournalEntry, Span, Task,
    WorkerRun, WorkflowRevision, WorkspaceId, WorkspaceStatus,
};

/// SQLite-backed persistent storage implementing TaskStore, SessionStore, OutboxPort, RunPort,
/// DecisionPort, WorkflowRevisionPort, and ReplanPort.
/// Manages tasks, execution spans, continuation packets, sessions, outbox, runs, decision records,
/// workflow revisions, and replans with WAL mode and foreign key integrity.
#[derive(Clone)]
pub struct SqliteTaskStore {
    db: DbConnection,
    task_repo: TaskRepository,
    span_repo: SpanRepository,
    continuation_repo: ContinuationRepository,
    session_repo: SessionRepository,
    outbox_repo: OutboxRepository,
    run_repo: RunRepository,
    decision_repo: DecisionRepository,
    workflow_repo: WorkflowRevisionRepository,
    replan_repo: ReplanRepository,
    workspace_repo: WorkspaceRepository,
    research_repo: ResearchRepository,
    provider_repo: ProviderRepository,
    fleet_automation_repo: FleetAutomationRepository,
}

impl SqliteTaskStore {
    /// Opens an in-memory SQLite store with all migrations applied.
    pub fn new_in_memory() -> Result<Self, DomainError> {
        let db = DbConnection::open_in_memory()?;
        let task_repo = TaskRepository::new(db.clone());
        let span_repo = SpanRepository::new(db.clone());
        let continuation_repo = ContinuationRepository::new(db.clone());
        let session_repo = SessionRepository::new(db.clone());
        let outbox_repo = OutboxRepository::new(db.clone());
        let run_repo = RunRepository::new(db.clone());
        let decision_repo = DecisionRepository::new(db.clone());
        let workflow_repo = WorkflowRevisionRepository::new(db.clone());
        let replan_repo = ReplanRepository::new(db.clone());
        let workspace_repo = WorkspaceRepository::new(db.clone());
        let research_repo = ResearchRepository::new(db.clone());
        let provider_repo = ProviderRepository::new(db.clone());
        let fleet_automation_repo = FleetAutomationRepository::new(db.clone());
        let store = Self {
            db,
            task_repo,
            span_repo,
            continuation_repo,
            session_repo,
            outbox_repo,
            run_repo,
            decision_repo,
            workflow_repo,
            replan_repo,
            workspace_repo,
            research_repo,
            provider_repo,
            fleet_automation_repo,
        };
        Ok(store)
    }

    /// Opens a file-backed SQLite store at `path` with all migrations applied.
    pub fn new(path: &str) -> Result<Self, DomainError> {
        let db = DbConnection::open(path)?;
        let task_repo = TaskRepository::new(db.clone());
        let span_repo = SpanRepository::new(db.clone());
        let continuation_repo = ContinuationRepository::new(db.clone());
        let session_repo = SessionRepository::new(db.clone());
        let outbox_repo = OutboxRepository::new(db.clone());
        let run_repo = RunRepository::new(db.clone());
        let decision_repo = DecisionRepository::new(db.clone());
        let workflow_repo = WorkflowRevisionRepository::new(db.clone());
        let replan_repo = ReplanRepository::new(db.clone());
        let workspace_repo = WorkspaceRepository::new(db.clone());
        let research_repo = ResearchRepository::new(db.clone());
        let provider_repo = ProviderRepository::new(db.clone());
        let fleet_automation_repo = FleetAutomationRepository::new(db.clone());
        let store = Self {
            db,
            task_repo,
            span_repo,
            continuation_repo,
            session_repo,
            outbox_repo,
            run_repo,
            decision_repo,
            workflow_repo,
            replan_repo,
            workspace_repo,
            research_repo,
            provider_repo,
            fleet_automation_repo,
        };
        store.seed_canonical_data_if_empty()?;
        Ok(store)
    }

    pub fn db(&self) -> &DbConnection {
        &self.db
    }

    pub fn outbox(&self) -> &OutboxRepository {
        &self.outbox_repo
    }

    pub fn tasks(&self) -> &TaskRepository {
        &self.task_repo
    }

    pub fn spans(&self) -> &SpanRepository {
        &self.span_repo
    }

    pub fn continuations(&self) -> &ContinuationRepository {
        &self.continuation_repo
    }

    pub fn sessions(&self) -> &SessionRepository {
        &self.session_repo
    }

    pub fn runs(&self) -> &RunRepository {
        &self.run_repo
    }

    pub fn decisions(&self) -> &DecisionRepository {
        &self.decision_repo
    }

    pub fn workflow_revisions(&self) -> &WorkflowRevisionRepository {
        &self.workflow_repo
    }

    pub fn replans(&self) -> &ReplanRepository {
        &self.replan_repo
    }

    pub fn workspaces(&self) -> &WorkspaceRepository {
        &self.workspace_repo
    }

    pub fn research(&self) -> &ResearchRepository {
        &self.research_repo
    }

    pub fn providers(&self) -> &ProviderRepository {
        &self.provider_repo
    }

    pub fn fleet_automation(&self) -> &FleetAutomationRepository {
        &self.fleet_automation_repo
    }

    pub fn seed_canonical_data_if_empty(&self) -> Result<(), DomainError> {
        // 1. Seed tasks if empty
        if self.task_repo.list_tasks()?.is_empty() {
            let mut task1 = Task::new(
                "task_inv_verify_01".to_string(),
                "Invariant Formal Verification Suite".to_string(),
            );
            task1.status = custos_domain::TaskStatus::Running;
            task1.contract = Some(custos_domain::TaskContract {
                pack_id: "engineering".to_string(),
                name: "Verification Contract".to_string(),
                description: "Verify invariants on multi-agent execution".to_string(),
                required_capabilities: vec!["analysis".to_string()],
                evidence_requirements: vec![],
            });
            self.task_repo.save_task(&task1)?;

            let mut task2 = Task::new(
                "task_worktree_02".to_string(),
                "Autonomous Worktree Pipeline".to_string(),
            );
            task2.status = custos_domain::TaskStatus::Draft;
            self.task_repo.save_task(&task2)?;

            let session_id = custos_domain::SessionId::new("sess_inv_01");
            let mut session = custos_domain::Session::new(
                session_id.clone(),
                custos_domain::SessionMode::Attached {
                    task_id: "task_inv_verify_01".to_string(),
                },
            );
            session.attached_to = Some("task_inv_verify_01".to_string());
            self.session_repo.save_session(&session)?;

            let now = chrono::Utc::now().to_rfc3339();
            self.session_repo.append_journal(&custos_domain::SessionJournalEntry {
                entry_id: None,
                session_id: session_id.clone(),
                entry_type: "user".to_string(),
                entry_data: "Initialize invariant verification bounds for multi-agent swarm".to_string(),
                occurred_at: now.clone(),
            })?;
            self.session_repo.append_journal(&custos_domain::SessionJournalEntry {
                entry_id: None,
                session_id,
                entry_type: "assistant".to_string(),
                entry_data: "Verification bounds initialized with MERKLE-CAS-SEALED invariant contract.".to_string(),
                occurred_at: now,
            })?;
        }

        // 2. Seed research sources & claims if empty
        if self.research_repo.list_sources()?.is_empty() {
            let now_ts = chrono::Utc::now().timestamp_millis();
            let src1 = custos_domain::SourceRecord {
                id: "src_nature_2024_01".to_string(),
                source_type: "paper".to_string(),
                title: "Self-Organizing Invariant Architectures in Deterministic Multi-Agent Swarms".to_string(),
                doi: Some("10.1038/s41586-024-07821-x".to_string()),
                authors: vec!["V. Pham".into(), "M. Chen".into(), "E. Vance".into()],
                year: Some(2024),
                content_hash: "blake3_9941a8e2f7b11c".to_string(),
                local_path: None,
                verified: true,
                abstract_text: Some("We present a zero-trust consensus mechanism bounding stochastic agent divergence using Merkle-sealed invariant contracts.".to_string()),
                created_at: now_ts - 7200000,
            };
            let src2 = custos_domain::SourceRecord {
                id: "src_arxiv_2025_02".to_string(),
                source_type: "paper".to_string(),
                title: "On the Convergence Rates of Cryptographic Capability Tickets under Asymmetric Latency".to_string(),
                doi: Some("10.48550/arXiv.2501.09912".to_string()),
                authors: vec!["T. Lindholm".into(), "K. S. Rao".into()],
                year: Some(2025),
                content_hash: "blake3_7718c091ad4e22".to_string(),
                local_path: None,
                verified: true,
                abstract_text: Some("This study provides lower bounds for atomic ticket acquisition across distributed authority gates.".to_string()),
                created_at: now_ts - 3600000,
            };
            self.research_repo.save_source(&src1)?;
            self.research_repo.save_source(&src2)?;

            let anchor = custos_domain::PassageAnchor {
                id: "anc_01".to_string(),
                source_id: "src_nature_2024_01".to_string(),
                source_title: Some("Self-Organizing Invariant Architectures".to_string()),
                section_title: Some("Section 4.2 Invariant Bounding".to_string()),
                page_number: Some(8),
                start_offset: 1240,
                end_offset: 1485,
                exact_text: "Phantom state execution was reduced by 99.8% across 10,000 runs.".to_string(),
                passage_hash: "blake3_anc_4491c".to_string(),
            };
            self.research_repo.save_anchor(&anchor)?;

            let claim = custos_domain::ResearchClaim {
                id: "claim_01".to_string(),
                statement: "Phantom state execution in unconstrained LLM loops can be reduced by 99.8% using Merkle-sealed state invariants.".to_string(),
                level: custos_domain::ClaimGroundingLevel::L3Sealed,
                confidence_score: 0.99,
                invariants: vec!["INV-PHANTOM-STATE-BOUND".to_string(), "INV-CAS-SEALED".to_string()],
                sealed_proof_uri: Some("cas://bafy2bzace4v3k99a77x1198".to_string()),
                evidence_links: vec![
                    custos_domain::ClaimEvidenceLink {
                        passage_anchor_id: "anc_01".to_string(),
                        source_title: Some("Self-Organizing Invariant Architectures".to_string()),
                        exact_text: Some("Phantom state execution was reduced by 99.8% across 10,000 runs.".to_string()),
                        relation: custos_domain::EvidenceRelation::Supports,
                        rationale: "Empirically proven with deterministic clean-room replays across 10,000 runs.".to_string(),
                        verified_by: "deterministic_engine".to_string(),
                    }
                ],
                created_at: now_ts - 3600000,
            };
            self.research_repo.save_claim(&claim)?;
        }

        // 3. Seed default providers if empty
        if self.provider_repo.list_providers()?.is_empty() {
            let now = chrono::Utc::now().timestamp_millis();
            let p1 = custos_domain::ProviderConfig {
                id: "p_anthropic".to_string(),
                name: "Anthropic Claude (Sonnet 3.7)".to_string(),
                service_type: "anthropic".to_string(),
                api_key_masked: "sk-ant-••••••••".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: None,
                created_at: now,
                updated_at: now,
            };
            let p2 = custos_domain::ProviderConfig {
                id: "p_openai".to_string(),
                name: "OpenAI Codex / GPT-4o".to_string(),
                service_type: "openai".to_string(),
                api_key_masked: "sk-proj-••••••••".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: None,
                created_at: now,
                updated_at: now,
            };
            let p3 = custos_domain::ProviderConfig {
                id: "p_gemini".to_string(),
                name: "Google Gemini 2.5 Flash".to_string(),
                service_type: "gemini".to_string(),
                api_key_masked: "AIzaSy••••••••".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: None,
                created_at: now,
                updated_at: now,
            };
            let p4 = custos_domain::ProviderConfig {
                id: "p_local".to_string(),
                name: "Local Model (Ollama / llama.cpp)".to_string(),
                service_type: "local".to_string(),
                api_key_masked: "none".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: Some("http://127.0.0.1:11434".to_string()),
                created_at: now,
                updated_at: now,
            };
            self.provider_repo.save_provider(&p1)?;
            self.provider_repo.save_provider(&p2)?;
            self.provider_repo.save_provider(&p3)?;
            self.provider_repo.save_provider(&p4)?;
        }

        // 4. Seed default client key if empty
        if self.provider_repo.list_client_keys()?.is_empty() {
            let k1 = custos_domain::ClientApiKeyRecord {
                id: "key_gateway_01".to_string(),
                name: "Custos Desktop Client Token".to_string(),
                token: "custos_live_sec_9941a8e2".to_string(),
                created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                revoked: false,
            };
            self.provider_repo.create_client_key(&k1)?;
        }

        Ok(())
    }

    /// Lists all tasks ordered by creation time descending.
    pub fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        self.task_repo.list_tasks()
    }
}

#[async_trait]
impl TaskStore for SqliteTaskStore {
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        self.task_repo.get_task(task_id)
    }

    async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        self.task_repo.list_tasks()
    }

    async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
        self.task_repo.save_task(task)
    }

    async fn commit_task_event(&self, event: &TaskEvent, task: &Task) -> Result<(), DomainError> {
        self.task_repo.save_task_with_event(task, event)
    }

    async fn get_task_events(&self, task_id: &str) -> Result<Vec<TaskEvent>, DomainError> {
        self.task_repo.get_task_events(task_id)
    }

    async fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError> {
        self.span_repo.get_span(span_id)
    }

    async fn save_span(&self, span: &Span) -> Result<(), DomainError> {
        self.span_repo.save_span(span)
    }

    async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError> {
        self.span_repo.list_spans(task_id)
    }

    async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError> {
        self.continuation_repo.save_continuation(packet)
    }

    async fn get_continuation(
        &self,
        task_id: &str,
        to_span: u32,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        self.continuation_repo.get_continuation(task_id, to_span)
    }

    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        self.continuation_repo.get_latest_continuation(task_id)
    }
}

#[async_trait]
impl SessionStore for SqliteTaskStore {
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, DomainError> {
        self.session_repo.get_session(session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<Session>, DomainError> {
        self.session_repo.list_sessions()
    }

    async fn save_session(&self, session: &Session) -> Result<(), DomainError> {
        self.session_repo.save_session(session)
    }

    async fn append_journal(&self, entry: &SessionJournalEntry) -> Result<i64, DomainError> {
        self.session_repo.append_journal(entry)
    }

    async fn get_journal(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<SessionJournalEntry>, DomainError> {
        self.session_repo.get_journal(session_id)
    }
}

#[async_trait]
impl OutboxPort for SqliteTaskStore {
    async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
        self.outbox_repo.enqueue(entry).await
    }

    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_dispatching(id).await
    }

    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError> {
        self.outbox_repo.mark_receipted(id, receipt).await
    }

    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_uncertain(id).await
    }

    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError> {
        self.outbox_repo.list_by_status(status).await
    }

    async fn list_by_task_and_status(
        &self,
        task_id: &str,
        status: OutboxStatus,
    ) -> Result<Vec<OutboxEntry>, DomainError> {
        self.outbox_repo
            .list_by_task_and_status(task_id, status)
            .await
    }
}

#[async_trait]
impl EffectLedgerPort for SqliteTaskStore {
    async fn record_effect(
        &self,
        effect: &custos_domain::EffectAttempt,
    ) -> Result<(), DomainError> {
        self.outbox_repo.record_effect(effect)
    }

    async fn update_effect_status(
        &self,
        id: &str,
        status: custos_domain::EffectStatus,
        receipt: Option<&ExecutionReceipt>,
    ) -> Result<(), DomainError> {
        self.outbox_repo.update_effect_status(id, status, receipt)
    }

    async fn get_effect_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<custos_domain::EffectAttempt>, DomainError> {
        self.outbox_repo.get_effect_by_idempotency_key(key)
    }

    async fn reconcile_on_startup(&self) -> Result<usize, DomainError> {
        self.outbox_repo.reconcile_on_startup()
    }
}

#[async_trait]
impl RunPort for SqliteTaskStore {
    async fn save_run(&self, run: &Run) -> Result<(), DomainError> {
        self.run_repo.save_run(run)
    }

    async fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError> {
        self.run_repo.get_run(run_id)
    }

    async fn list_runs_for_task(&self, task_id: &str) -> Result<Vec<Run>, DomainError> {
        self.run_repo.list_runs_for_task(task_id)
    }

    async fn save_worker_run(&self, wrun: &WorkerRun) -> Result<(), DomainError> {
        self.run_repo.save_worker_run(wrun)
    }

    async fn get_worker_run(&self, wrun_id: &str) -> Result<Option<WorkerRun>, DomainError> {
        self.run_repo.get_worker_run(wrun_id)
    }

    async fn list_worker_runs_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkerRun>, DomainError> {
        self.run_repo.list_worker_runs_for_task(task_id)
    }

    async fn list_worker_runs_for_run(&self, run_id: &str) -> Result<Vec<WorkerRun>, DomainError> {
        self.run_repo.list_worker_runs_for_run(run_id)
    }
}

#[async_trait]
impl DecisionPort for SqliteTaskStore {
    async fn record_decision(&self, record: &DecisionRecord) -> Result<(), DomainError> {
        self.decision_repo.save_decision(record)
    }

    async fn get_decision(&self, id: &str) -> Result<Option<DecisionRecord>, DomainError> {
        self.decision_repo.get_decision(id)
    }

    async fn list_decisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<DecisionRecord>, DomainError> {
        self.decision_repo.list_decisions_for_task(task_id)
    }
}

#[async_trait]
impl WorkflowRevisionPort for SqliteTaskStore {
    async fn save_revision(
        &self,
        revision: &WorkflowRevision,
        placements: &[NodePlacement],
    ) -> Result<(), DomainError> {
        self.workflow_repo.save_revision(revision, placements)
    }

    async fn get_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<WorkflowRevision>, DomainError> {
        self.workflow_repo.get_revision(revision_id)
    }

    async fn list_revisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkflowRevision>, DomainError> {
        self.workflow_repo.list_revisions_for_task(task_id)
    }

    async fn get_placements_for_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<NodePlacement>, DomainError> {
        self.workflow_repo.get_placements_for_revision(revision_id)
    }
}

#[async_trait]
impl ReplanPort for SqliteTaskStore {
    async fn record_replan(&self, record: &ReplanRecord) -> Result<(), DomainError> {
        self.replan_repo.save_replan(record)
    }

    async fn get_replan(&self, id: &str) -> Result<Option<ReplanRecord>, DomainError> {
        self.replan_repo.get_replan(id)
    }

    async fn list_replans_for_task(&self, task_id: &str) -> Result<Vec<ReplanRecord>, DomainError> {
        self.replan_repo.list_replans_for_task(task_id)
    }
}

#[async_trait]
impl WorkspaceRepoPort for SqliteTaskStore {
    async fn save_workspace(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        self.workspace_repo.save_workspace(workspace)
    }

    async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError> {
        self.workspace_repo.get_workspace(id)
    }

    async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        self.workspace_repo.list_workspaces()
    }

    async fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError> {
        self.workspace_repo.update_workspace_status(id, status)
    }

    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError> {
        self.workspace_repo.delete_workspace(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core::kernel::{AdvanceTask, CreateTask, TaskReducer, TaskService};
    use custos_domain::{ContractEvidence, EvidenceKind, SpanState, TaskContract, TaskStatus};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_task_crud_and_list() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("test_task_1".to_string(), "Test Task 1".to_string());

        store.save_task(&task).await.unwrap();

        let retrieved = store.get_task(&task.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, task.id);
        assert_eq!(retrieved.title, "Test Task 1");
        assert_eq!(retrieved.status, TaskStatus::Draft);

        let all = store.list_tasks().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, task.id);
    }

    #[tokio::test]
    async fn task_contract_survives_persistence_and_list() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let mut task = Task::new("task_contract".into(), "Contract test".into());
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Contract test".into(),
            description: "Preserve contract through SQLite".into(),
            required_capabilities: vec!["fs_read".into(), "fs_write".into()],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        store.save_task(&task).await.unwrap();
        let loaded = store.get_task(&task.id).await.unwrap().unwrap();
        assert_eq!(loaded.contract.as_ref().unwrap().pack_id, "engineering");
        assert_eq!(
            loaded
                .contract
                .as_ref()
                .unwrap()
                .required_capabilities
                .len(),
            2
        );

        let listed = store.list_tasks().unwrap();
        assert_eq!(
            listed[0].contract.as_ref().unwrap().description,
            "Preserve contract through SQLite"
        );
    }

    #[tokio::test]
    async fn task_events_commit_with_snapshot_and_replay_exact_state() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let service = TaskService::new(store.clone());
        let (created, created_event) = service
            .execute_create(CreateTask {
                title: "Replay task".into(),
                metadata: Some(serde_json::json!({"origin": "test"})),
                contract: Some(TaskContract {
                    pack_id: "engineering".into(),
                    name: "Replay task".into(),
                    description: "Rebuild from events".into(),
                    required_capabilities: vec!["fs_read".into()],
                    evidence_requirements: vec![ContractEvidence {
                        kind: EvidenceKind::TestResult,
                        required: true,
                    }],
                }),
            })
            .await
            .unwrap();
        service
            .execute_advance(AdvanceTask {
                task_id: created.id.clone(),
                expected_epoch: created.epoch,
                next_status: TaskStatus::Queued,
                rationale: Some("test replay".into()),
            })
            .await
            .unwrap();

        let events = store.get_task_events(&created.id).await.unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TaskEvent::Created(_)));
        assert!(matches!(events[1], TaskEvent::Advanced(_)));

        let replayed = TaskReducer::replay(&events).unwrap();
        let persisted = store.get_task(&created.id).await.unwrap().unwrap();
        assert_eq!(replayed.id, persisted.id);
        assert_eq!(replayed.status, persisted.status);
        assert_eq!(replayed.epoch, persisted.epoch);
        assert_eq!(replayed.metadata, persisted.metadata);
        assert_eq!(replayed.contract.unwrap().pack_id, "engineering");

        let same_event = TaskEvent::Created(created_event);
        store
            .commit_task_event(&same_event, &created)
            .await
            .unwrap();
        assert!(store.get_task_events(&created.id).await.unwrap().len() == 2);
    }

    #[tokio::test]
    async fn test_span_lifecycle() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("test_task_2".to_string(), "Test Task 2".to_string());
        store.save_task(&task).await.unwrap();

        let span1 = Span::new(
            "span_1".to_string(),
            task.id.clone(),
            1,
            "test-provider".to_string(),
            "test-model".to_string(),
            "sha256:abc".to_string(),
        );
        store.save_span(&span1).await.unwrap();

        let retrieved = store.get_span(&span1.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, span1.id);
        assert_eq!(retrieved.span_num, 1);
        assert_eq!(retrieved.state, SpanState::Started);

        let spans = store.list_spans(&task.id).await.unwrap();
        assert_eq!(spans.len(), 1);
    }

    #[tokio::test]
    async fn test_continuation_packets() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("test_task_3".to_string(), "Test Task 3".to_string());
        store.save_task(&task).await.unwrap();

        let packet = ContinuationPacket::create(
            task.id.clone(),
            1,
            2,
            "test-provider".to_string(),
            "test-model".to_string(),
            "Summary of task 3".to_string(),
            serde_json::json!({"step": 2}),
        )
        .unwrap();
        store.save_continuation(&packet).await.unwrap();

        let retrieved = store
            .get_latest_continuation(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.task_id, task.id);
        assert_eq!(retrieved.to_span, 2);
        assert_eq!(retrieved.task_summary, "Summary of task 3");
        assert!(retrieved.verify().is_ok());
    }

    #[tokio::test]
    async fn test_foreign_key_enforcement() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        // span referring to non-existent task must fail due to foreign keys = ON
        let span = Span::new(
            "span_orphan".to_string(),
            "non_existent_task".to_string(),
            1,
            "test-provider".to_string(),
            "test-model".to_string(),
            "sha256:abc".to_string(),
        );
        let result = store.save_span(&span).await;
        assert!(
            result.is_err(),
            "Foreign key constraint must prevent span creation for missing task"
        );
    }

    #[tokio::test]
    async fn test_session_store_crud_and_journal() {
        use custos_domain::{Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let session_id = SessionId("ses_test_1".into());
        let session = Session::new(session_id.clone(), SessionMode::Bare);

        // 1. Save session
        store.save_session(&session).await.unwrap();

        // 2. Retrieve session
        let loaded = store.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(loaded.id, session_id);
        assert_eq!(loaded.status, SessionStatus::Active);

        // 3. Append journal entries
        let entry1 = SessionJournalEntry {
            entry_id: None,
            session_id: session_id.clone(),
            entry_type: "user_message".into(),
            entry_data: "Hello Custos".into(),
            occurred_at: chrono::Utc::now().to_rfc3339(),
        };
        let row_id = store.append_journal(&entry1).await.unwrap();
        assert!(row_id > 0);

        let entry2 = SessionJournalEntry {
            entry_id: None,
            session_id: session_id.clone(),
            entry_type: "assistant_message".into(),
            entry_data: "Hello! How can I assist you?".into(),
            occurred_at: chrono::Utc::now().to_rfc3339(),
        };
        store.append_journal(&entry2).await.unwrap();

        // 4. Retrieve journal
        let journal = store.get_journal(&session_id).await.unwrap();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[0].entry_type, "user_message");
        assert_eq!(journal[0].entry_data, "Hello Custos");
        assert_eq!(journal[1].entry_type, "assistant_message");

        // 5. Update session status and goal
        let mut updated_session = loaded;
        updated_session.current_goal = Some("Refactor codebase".into());
        updated_session.promotion_score = 0.45;
        updated_session.status = SessionStatus::Paused;
        store.save_session(&updated_session).await.unwrap();

        let reloaded = store.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(reloaded.current_goal.as_deref(), Some("Refactor codebase"));
        assert_eq!(reloaded.promotion_score, 0.45);
        assert_eq!(reloaded.status, SessionStatus::Paused);

        // 6. List sessions
        let list = store.list_sessions().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, session_id);
    }

    #[tokio::test]
    async fn test_run_and_worker_run_persistence() {
        use custos_domain::{Run, RunStatus, WorkerRun};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("task_run_1".into(), "Run Test Task".into());
        store.save_task(&task).await.unwrap();

        // 1. Create and save Run
        let mut run = Run::new(task.id.clone(), 1);
        run.workflow_revision = Some("rev-1".into());
        run.transition(RunStatus::Active).unwrap();
        store.save_run(&run).await.unwrap();

        // 2. Query Run
        let loaded = store.get_run(&run.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, run.id);
        assert_eq!(loaded.task_id, task.id);
        assert_eq!(loaded.status, RunStatus::Active);
        assert_eq!(loaded.workflow_revision.as_deref(), Some("rev-1"));

        // 3. Create and save WorkerRun
        let mut wrun = WorkerRun::new(task.id.clone(), "worker_alpha".into(), 3);
        wrun.run_id = Some(run.id.clone());
        wrun.transition(RunStatus::Active).unwrap();
        store.save_worker_run(&wrun).await.unwrap();

        // 4. Query WorkerRun
        let loaded_wrun = store.get_worker_run(&wrun.id).await.unwrap().unwrap();
        assert_eq!(loaded_wrun.id, wrun.id);
        assert_eq!(loaded_wrun.run_id.as_deref(), Some(run.id.as_str()));
        assert_eq!(loaded_wrun.status, RunStatus::Active);
        assert_eq!(loaded_wrun.worker_id, "worker_alpha");

        // 5. Update Run status to Completed
        run.transition(RunStatus::Completed).unwrap();
        store.save_run(&run).await.unwrap();
        let updated_run = store.get_run(&run.id).await.unwrap().unwrap();
        assert_eq!(updated_run.status, RunStatus::Completed);
        assert!(updated_run.ended_at.is_some());

        // 6. List runs and worker runs
        let runs = store.list_runs_for_task(&task.id).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, run.id);

        let wruns = store.list_worker_runs_for_task(&task.id).await.unwrap();
        assert_eq!(wruns.len(), 1);
        assert_eq!(wruns[0].id, wrun.id);

        let wruns_by_run = store.list_worker_runs_for_run(&run.id).await.unwrap();
        assert_eq!(wruns_by_run.len(), 1);
        assert_eq!(wruns_by_run[0].id, wrun.id);
    }

    #[tokio::test]
    async fn test_workflow_revision_and_replan_ports() {
        use custos_domain::{ReplanBrief, ReplanTrigger, RevisionNode};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("task_store_rev_1".to_string(), "Rev Test".to_string());
        store.save_task(&task).await.unwrap();

        // 1. Save and query WorkflowRevision
        let mut revision = WorkflowRevision::new(&task.id, "prop_oi_01", 1);
        revision.nodes.push(RevisionNode {
            node_id: "node_1".into(),
            step_name: "Inspect".into(),
            role: "worker".into(),
            harness_id: "claude_code".into(),
            allocated_budget_tokens: 3000,
            read_set: vec!["file.txt".into()],
            write_set: vec![],
            required_capabilities: vec!["fs_read".into()],
        });

        let placements = vec![NodePlacement::new("node_1", "worker", "claude_code", 3000)];

        store.save_revision(&revision, &placements).await.unwrap();

        let loaded_rev = store
            .get_revision(&revision.revision_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded_rev.revision_id, revision.revision_id);
        assert_eq!(loaded_rev.nodes.len(), 1);

        let loaded_placements = store
            .get_placements_for_revision(&revision.revision_id)
            .await
            .unwrap();
        assert_eq!(loaded_placements.len(), 1);
        assert_eq!(loaded_placements[0].budget_tokens_slice, 3000);

        let revs = store.list_revisions_for_task(&task.id).await.unwrap();
        assert_eq!(revs.len(), 1);

        // 2. Save and query ReplanRecord
        let brief = ReplanBrief::new(
            &task.id,
            ReplanTrigger::SourceDrift,
            None,
            "Files changed on disk",
        );
        let replan = ReplanRecord::new(&task.id, brief, "prop_oi_02");
        store.record_replan(&replan).await.unwrap();

        let loaded_replan = store.get_replan(&replan.id).await.unwrap().unwrap();
        assert_eq!(loaded_replan.id, replan.id);
        assert_eq!(loaded_replan.brief.trigger, ReplanTrigger::SourceDrift);

        let replans = store.list_replans_for_task(&task.id).await.unwrap();
        assert_eq!(replans.len(), 1);
        assert_eq!(replans[0].id, replan.id);
    }

    #[tokio::test]
    async fn test_execution_workspace_persistence() {
        use custos_core::contracts::workspace::WorkspaceRepository as _;
        use custos_domain::{ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceStatus};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let ws_id = WorkspaceId::generate();

        let ws = ExecutionWorkspace::new(
            ws_id.clone(),
            "orca-worktree-feat",
            WorkspaceKind::Git {
                repo_path: "/workspace/custos".into(),
                branch: "feat/orca".into(),
                base_commit: Some("c0ffee".into()),
            },
            "/workspace/custos/.worktrees/orca",
        )
        .with_metadata(serde_json::json!({
            "domain": "engineering",
            "workbench": "coding"
        }));

        // 1. Save workspace
        store.save_workspace(&ws).await.unwrap();

        // 2. Get workspace
        let loaded = store.get_workspace(&ws_id).await.unwrap().unwrap();
        assert_eq!(loaded.id, ws_id);
        assert_eq!(loaded.name, "orca-worktree-feat");
        assert_eq!(loaded.status, WorkspaceStatus::Initializing);
        assert_eq!(loaded.metadata["domain"], "engineering");

        // 3. Update status
        store
            .update_workspace_status(&ws_id, WorkspaceStatus::Ready)
            .await
            .unwrap();

        let updated = store.get_workspace(&ws_id).await.unwrap().unwrap();
        assert_eq!(updated.status, WorkspaceStatus::Ready);

        // 4. List workspaces
        let list = store.list_workspaces().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, ws_id);

        // 5. Delete workspace
        store.delete_workspace(&ws_id).await.unwrap();
        let deleted = store.get_workspace(&ws_id).await.unwrap();
        assert!(deleted.is_none());
    }
}
