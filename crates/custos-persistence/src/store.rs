use crate::connection::DbConnection;
use crate::repositories::{
    ContinuationRepository, SessionRepository, SpanRepository, TaskRepository,
};
use async_trait::async_trait;
use custos_domain::{
    ContinuationPacket, DomainError, Session, SessionId, SessionJournalEntry, Span, Task,
};
use custos_core::kernel::{SessionStore, TaskEvent, TaskStore};

/// SQLite-backed persistent storage implementing TaskStore and SessionStore.
/// Manages tasks, execution spans, continuation packets, sessions, and session journals
/// with WAL mode and foreign key integrity.
#[derive(Clone)]
pub struct SqliteTaskStore {
    db: DbConnection,
    task_repo: TaskRepository,
    span_repo: SpanRepository,
    continuation_repo: ContinuationRepository,
    session_repo: SessionRepository,
}

impl SqliteTaskStore {
    /// Opens an in-memory SQLite store with all migrations applied.
    pub fn new_in_memory() -> Result<Self, DomainError> {
        let db = DbConnection::open_in_memory()?;
        let task_repo = TaskRepository::new(db.clone());
        let span_repo = SpanRepository::new(db.clone());
        let continuation_repo = ContinuationRepository::new(db.clone());
        let session_repo = SessionRepository::new(db.clone());
        Ok(Self {
            db,
            task_repo,
            span_repo,
            continuation_repo,
            session_repo,
        })
    }

    /// Opens a file-backed SQLite store at `path` with all migrations applied.
    pub fn new(path: &str) -> Result<Self, DomainError> {
        let db = DbConnection::open(path)?;
        let task_repo = TaskRepository::new(db.clone());
        let span_repo = SpanRepository::new(db.clone());
        let continuation_repo = ContinuationRepository::new(db.clone());
        let session_repo = SessionRepository::new(db.clone());
        Ok(Self {
            db,
            task_repo,
            span_repo,
            continuation_repo,
            session_repo,
        })
    }

    pub fn db(&self) -> &DbConnection {
        &self.db
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

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::{ContractEvidence, EvidenceKind, SpanState, TaskContract, TaskStatus};
    use custos_core::kernel::{AdvanceTask, CreateTask, TaskReducer, TaskService};
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
}
