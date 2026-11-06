use crate::connection::DbConnection;
use crate::repositories::{
    AssistantRepository, CommandLedgerRepository, ContinuationRepository, DecisionRepository,
    ExperimentAttemptRepository, FleetAutomationRepository, OutboxRepository, PackageRepository,
    ProviderRepository, QuantRepository, ReplanRepository, ResearchRepository, RunRepository,
    SessionRepository, SpanRepository, TaskRepository, WorkflowRevisionRepository,
    WorkspaceRepository,
};
pub use custos_core::contracts::storage::{
    CommandAdmission, CommandLedgerEntry, CommandLedgerPort, DecisionPort, EffectLedgerPort,
    NotebookStoragePort, OutboxEntry, OutboxPort, OutboxStatus, QuantPort, ReplanPort, RunPort,
    WorkflowRevisionPort,
};
pub use custos_core::contracts::workspace::WorkspaceRepository as WorkspaceRepoPort;
pub use custos_core::kernel::{SessionStore, TaskEvent, TaskStore};
pub use custos_domain::{
    ContinuationPacket, DecisionRecord, DispatchClaim, DomainError, ExecutionReceipt,
    ExecutionWorkspace, LaunchAttempt, NodePlacement, NotebookCell, NotebookKernelState,
    ReplanRecord, Run, Session, SessionId, SessionJournalEntry, Span, Task, WorkerRun,
    WorkflowRevision, WorkspaceId, WorkspaceStatus,
};

#[cfg(test)]
mod seed;
mod traits;

/// SQLite-backed persistent storage implementing TaskStore, SessionStore, OutboxPort, RunPort,
/// DecisionPort, WorkflowRevisionPort, and ReplanPort.
/// Manages tasks, execution spans, continuation packets, sessions, outbox, runs, decision records,
/// workflow revisions, and replans with WAL mode and foreign key integrity.
/// Alias representing the canonical SQLite persistent store for Custos
pub type CustosStore = SqliteTaskStore;

#[derive(Clone)]
pub struct SqliteTaskStore {
    pub(super) db: DbConnection,
    pub(super) task_repo: TaskRepository,
    pub(super) span_repo: SpanRepository,
    pub(super) continuation_repo: ContinuationRepository,
    pub(super) session_repo: SessionRepository,
    pub(super) outbox_repo: OutboxRepository,
    pub(super) run_repo: RunRepository,
    pub(super) decision_repo: DecisionRepository,
    pub(super) workflow_repo: WorkflowRevisionRepository,
    pub(super) replan_repo: ReplanRepository,
    pub(super) workspace_repo: WorkspaceRepository,
    pub(super) research_repo: ResearchRepository,
    pub(super) provider_repo: ProviderRepository,
    pub(super) fleet_automation_repo: FleetAutomationRepository,
    pub(super) assistant_repo: AssistantRepository,
    pub(super) command_ledger_repo: CommandLedgerRepository,
    pub(super) experiment_attempt_repo: ExperimentAttemptRepository,
    pub(super) quant_repo: QuantRepository,
    pub(super) package_repo: PackageRepository,
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
        let package_repo = PackageRepository::new(db.clone());
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
            package_repo,
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
        let package_repo = PackageRepository::new(db.clone());
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
            package_repo,
        };
        Ok(store)
    }

    pub fn db(&self) -> &DbConnection {
        &self.db
    }

    pub fn packages(&self) -> &PackageRepository {
        &self.package_repo
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
    pub fn reconcile_all_on_startup(
        &self,
    ) -> Result<custos_domain::ReconciliationSummary, DomainError> {
        let outbox_reconciled = self.outbox_repo.reconcile_on_startup()?;
        let runs_reconciled = self.run_repo.reconcile_runs_on_startup()?;
        let attempts_reconciled = self
            .experiment_attempt_repo
            .reconcile_attempts_on_startup()?;
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

    /// Health check pinging SQLite to verify read/write availability and WAL consistency
    pub fn health_check(&self) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        let res: i64 = conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .map_err(|e| DomainError::Validation(format!("SQLite health check failed: {e}")))?;
        if res == 1 {
            Ok(())
        } else {
            Err(DomainError::Validation(
                "SQLite health check returned unexpected value".into(),
            ))
        }
    }

    pub fn get_settings(&self) -> Result<std::collections::HashMap<String, String>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM daemon_settings")
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
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
        if settings
            .keys()
            .any(|key| key.trim().is_empty() || key.len() > 128)
            || settings.values().any(|value| value.len() > 65_536)
        {
            return Err(DomainError::Validation(
                "Settings keys must be non-empty and <=128 bytes; values must be <=65536 bytes"
                    .into(),
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

    /// Lists all tasks ordered by creation time descending.
    pub fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        self.task_repo.list_tasks()
    }
}

#[cfg(test)]
mod tests;
