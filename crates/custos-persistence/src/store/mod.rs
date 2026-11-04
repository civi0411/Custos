use crate::connection::DbConnection;
use crate::repositories::{
    AssistantRepository, CommandLedgerRepository, ContinuationRepository, DecisionRepository,
    ExperimentAttemptRepository, FleetAutomationRepository, OutboxRepository, ProviderRepository,
    QuantRepository, ReplanRepository, ResearchRepository, RunRepository, SessionRepository,
    SpanRepository, TaskRepository, WorkflowRevisionRepository, WorkspaceRepository,
};
use async_trait::async_trait;
use custos_core::contracts::storage::{
    CommandAdmission, CommandLedgerEntry, CommandLedgerPort, DecisionPort, EffectLedgerPort, NotebookStoragePort,
    OutboxEntry, OutboxPort, OutboxStatus, QuantPort, ReplanPort, RunPort, WorkflowRevisionPort,
};
use custos_core::contracts::workspace::WorkspaceRepository as WorkspaceRepoPort;
use custos_core::kernel::{SessionStore, TaskEvent, TaskStore};
use custos_domain::{
    ContinuationPacket, DecisionRecord, DispatchClaim, DomainError, ExecutionReceipt,
    ExecutionWorkspace, LaunchAttempt, NodePlacement, NotebookCell, NotebookKernelState, ReplanRecord, Run, Session,
    SessionId, SessionJournalEntry, Span, Task, WorkerRun, WorkflowRevision, WorkspaceId,
    WorkspaceStatus,
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
    assistant_repo: AssistantRepository,
    command_ledger_repo: CommandLedgerRepository,
    experiment_attempt_repo: ExperimentAttemptRepository,
    quant_repo: QuantRepository,
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
        let assistant_repo = AssistantRepository::new(db.clone());
        let command_ledger_repo = CommandLedgerRepository::new(db.clone());
        let experiment_attempt_repo = ExperimentAttemptRepository::new(db.writer_arc());
        let quant_repo = QuantRepository::new(db.clone());
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
            assistant_repo,
            command_ledger_repo,
            experiment_attempt_repo,
            quant_repo,
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
        let assistant_repo = AssistantRepository::new(db.clone());
        let command_ledger_repo = CommandLedgerRepository::new(db.clone());
        let experiment_attempt_repo = ExperimentAttemptRepository::new(db.writer_arc());
        let quant_repo = QuantRepository::new(db.clone());
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
            assistant_repo,
            command_ledger_repo,
            experiment_attempt_repo,
            quant_repo,
        };
        Ok(store)
    }

    pub fn db(&self) -> &DbConnection {
        &self.db
    }

    pub fn experiment_attempts(&self) -> &ExperimentAttemptRepository {
        &self.experiment_attempt_repo
    }

    pub fn outbox(&self) -> &OutboxRepository {
        &self.outbox_repo
    }

    pub fn assistant(&self) -> &AssistantRepository {
        &self.assistant_repo
    }

    pub fn quant(&self) -> &QuantRepository {
        &self.quant_repo
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

    pub fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        self.run_repo.reconcile_runs_on_startup()
    }

    /// Reconciles all in-flight entities across outbox, runs, and attempts on daemon startup/recovery.
    pub fn reconcile_all_on_startup(&self) -> Result<custos_domain::ReconciliationSummary, DomainError> {
        let outbox_reconciled = self.outbox_repo.reconcile_on_startup()?;
        let runs_reconciled = self.run_repo.reconcile_runs_on_startup()?;
        let attempts_reconciled = self.experiment_attempt_repo.reconcile_attempts_on_startup()?;
        Ok(custos_domain::ReconciliationSummary {
            outbox_reconciled,
            runs_reconciled,
            attempts_reconciled,
        })
    }

    pub fn list_uncertain_runs(&self) -> Result<Vec<Run>, DomainError> {
        self.run_repo.list_uncertain_runs()
    }

    pub fn fence_claim(&self, claim_id: &str) -> Result<(), DomainError> {
        self.run_repo.fence_claim(claim_id)
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

    pub fn command_ledger(&self) -> &CommandLedgerRepository {
        &self.command_ledger_repo
    }

    pub fn get_settings(&self) -> Result<std::collections::HashMap<String, String>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM daemon_settings")
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut map = std::collections::HashMap::new();
        for r in rows {
            let (key, value) = r.map_err(|e| DomainError::Validation(e.to_string()))?;
            map.insert(key, value);
        }
        Ok(map)
    }

    pub fn set_settings(
        &self,
        settings: &std::collections::HashMap<String, String>,
    ) -> Result<(), DomainError> {
        if settings.keys().any(|key| key.trim().is_empty() || key.len() > 128)
            || settings.values().any(|value| value.len() > 65_536)
        {
            return Err(DomainError::Validation(
                "Settings keys must be non-empty and <=128 bytes; values must be <=65536 bytes".into(),
            ));
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        for (key, value) in settings {
            tx.execute(
                "INSERT INTO daemon_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
                rusqlite::params![key, value],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO daemon_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            rusqlite::params![key, value],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    #[cfg(test)]
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
            self.session_repo
                .append_journal(&custos_domain::SessionJournalEntry {
                    entry_id: None,
                    session_id: session_id.clone(),
                    entry_type: "user".to_string(),
                    entry_data: "Initialize invariant verification bounds for multi-agent swarm"
                        .to_string(),
                    occurred_at: now.clone(),
                })?;
            self.session_repo
                .append_journal(&custos_domain::SessionJournalEntry {
                    entry_id: None,
                    session_id,
                    entry_type: "assistant".to_string(),
                    entry_data:
                        "Verification bounds initialized with MERKLE-CAS-SEALED invariant contract."
                            .to_string(),
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
                exact_text: "Phantom state execution was reduced by 99.8% across 10,000 runs."
                    .to_string(),
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
                default_model: Some("claude-3-7-sonnet".to_string()),
                context_window: Some(200_000),
                fast_mode: Some(true),
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
                default_model: Some("gpt-4o".to_string()),
                context_window: Some(128_000),
                fast_mode: Some(true),
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
                default_model: Some("gemini-2.5-flash".to_string()),
                context_window: Some(1_048_576),
                fast_mode: Some(true),
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
                default_model: Some("qwen2.5-coder:32b".to_string()),
                context_window: Some(32_768),
                fast_mode: Some(false),
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

    async fn delete_session(&self, session_id: &SessionId) -> Result<bool, DomainError> {
        self.session_repo.delete_session(session_id)
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

    async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError> {
        self.outbox_repo.get(id).await
    }

    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_dispatching(id).await
    }

    async fn mark_approved(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_approved(id).await
    }

    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError> {
        self.outbox_repo.mark_receipted(id, receipt).await
    }

    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_uncertain(id).await
    }

    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_failed(id, error).await
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
    async fn save_run(&self, run: &mut Run) -> Result<(), DomainError> {
        self.run_repo.save_run(run)
    }

    async fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError> {
        self.run_repo.get_run(run_id)
    }

    async fn list_runs_for_task(&self, task_id: &str) -> Result<Vec<Run>, DomainError> {
        self.run_repo.list_runs_for_task(task_id)
    }

    async fn save_worker_run(&self, wrun: &mut WorkerRun) -> Result<(), DomainError> {
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

    async fn claim_ready(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        self.run_repo.claim_ready(claim)
    }

    async fn update_claim(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        self.run_repo.update_claim(claim)
    }

    async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError> {
        self.run_repo.get_claim(claim_id)
    }

    async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Result<Option<DispatchClaim>, DomainError> {
        self.run_repo.get_active_claim(task_id, node_id)
    }

    async fn save_launch_attempt(&self, attempt: &LaunchAttempt) -> Result<(), DomainError> {
        self.run_repo.save_launch_attempt(attempt)
    }

    async fn get_launch_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<Option<LaunchAttempt>, DomainError> {
        self.run_repo.get_launch_attempt(attempt_id)
    }

    async fn list_active_claims(&self) -> Result<Vec<DispatchClaim>, DomainError> {
        self.run_repo.list_active_claims()
    }

    async fn fence_claim(&self, claim_id: &str) -> Result<(), DomainError> {
        self.run_repo.fence_claim(claim_id)
    }

    async fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        self.run_repo.reconcile_runs_on_startup()
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
impl CommandLedgerPort for SqliteTaskStore {
    async fn get_command(&self, command_id: &str) -> Result<Option<CommandLedgerEntry>, DomainError> {
        self.command_ledger_repo.get_command(command_id)
    }

    async fn record_command(&self, entry: &CommandLedgerEntry) -> Result<(), DomainError> {
        self.command_ledger_repo.record_command(entry)
    }

    async fn admit(
        &self,
        command_id: &str,
        method: &str,
        payload_digest: &str,
        scope: Option<&str>,
        now_ms: i64,
        ttl_ms: i64,
    ) -> Result<CommandAdmission, DomainError> {
        self.command_ledger_repo
            .admit(command_id, method, payload_digest, scope, now_ms, ttl_ms)
    }

    async fn complete(&self, command_id: &str, result_json: &str) -> Result<(), DomainError> {
        self.command_ledger_repo.complete(command_id, result_json)
    }

    async fn abandon(&self, command_id: &str) -> Result<(), DomainError> {
        self.command_ledger_repo.abandon(command_id)
    }
}

#[async_trait]
impl NotebookStoragePort for SqliteTaskStore {
    async fn list_notebook_cells(
        &self,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        self.research_repo.list_notebook_cells(session_id)
    }

    async fn list_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        self.research_repo
            .list_notebook_cells_scoped(session_id, notebook_id)
    }

    async fn replace_notebook_cells(
        &self,
        session_id: &str,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.research_repo.replace_notebook_cells(session_id, cells)
    }

    async fn replace_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.research_repo
            .replace_notebook_cells_scoped(session_id, notebook_id, cells)
    }

    async fn save_notebook_cell(&self, cell: &NotebookCell) -> Result<(), DomainError> {
        self.research_repo.save_notebook_cell(cell)
    }

    async fn get_notebook_kernel_state(
        &self,
        session_id: &str,
    ) -> Result<NotebookKernelState, DomainError> {
        self.research_repo.get_notebook_kernel_state(session_id)
    }

    async fn get_notebook_kernel_state_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, DomainError> {
        self.research_repo
            .get_notebook_kernel_state_scoped(session_id, notebook_id)
    }

    async fn save_notebook_kernel_state(
        &self,
        state: &NotebookKernelState,
    ) -> Result<(), DomainError> {
        self.research_repo.save_notebook_kernel_state(state)
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

#[async_trait]
impl QuantPort for SqliteTaskStore {
    async fn save_tearsheet(&self, tearsheet: &custos_domain::quant::BacktestTearsheet, task_id: &str) -> Result<(), DomainError> {
        self.quant_repo.save_tearsheet(tearsheet, task_id).await
    }

    async fn get_latest_tearsheet(&self, strategy_id: &str) -> Result<Option<custos_domain::quant::BacktestTearsheet>, DomainError> {
        self.quant_repo.get_latest_tearsheet(strategy_id).await
    }

    async fn save_order_ticket(&self, ticket: &custos_domain::quant::OrderTicket, task_id: &str) -> Result<(), DomainError> {
        self.quant_repo.save_order_ticket(ticket, task_id).await
    }

    async fn list_order_tickets(&self, strategy_id: Option<&str>, task_id: Option<&str>) -> Result<Vec<custos_domain::quant::OrderTicket>, DomainError> {
        self.quant_repo.list_order_tickets(strategy_id, task_id).await
    }

    async fn get_order_ticket(&self, order_id: &str) -> Result<Option<custos_domain::quant::OrderTicket>, DomainError> {
        self.quant_repo.get_order_ticket(order_id).await
    }

    async fn update_order_ticket(&self, ticket: &custos_domain::quant::OrderTicket) -> Result<(), DomainError> {
        self.quant_repo.update_order_ticket(ticket).await
    }

    async fn save_market_bars(&self, symbol: &str, timeframe: &str, bars: &[custos_domain::quant::OhlcvBar]) -> Result<(), DomainError> {
        self.quant_repo.save_market_bars(symbol, timeframe, bars).await
    }

    async fn get_market_bars(&self, symbol: &str, timeframe: &str, limit: usize) -> Result<Vec<custos_domain::quant::OhlcvBar>, DomainError> {
        self.quant_repo.get_market_bars(symbol, timeframe, limit).await
    }
}


#[cfg(test)]
mod tests;
