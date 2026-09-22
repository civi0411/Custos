use crate::connection::DbConnection;
use crate::repositories::{ContinuationRepository, SpanRepository, TaskRepository};
use async_trait::async_trait;
use custos_core_domain::{ContinuationPacket, DomainError, Span, Task};
use custos_task_kernel::TaskStore;

/// SQLite-backed persistent storage implementing TaskStore.
/// Manages tasks, execution spans, and continuation packets with WAL mode and foreign key integrity.
#[derive(Clone)]
pub struct SqliteTaskStore {
    db: DbConnection,
    task_repo: TaskRepository,
    span_repo: SpanRepository,
    continuation_repo: ContinuationRepository,
}

impl SqliteTaskStore {
    /// Opens an in-memory SQLite store with all migrations applied.
    pub fn new_in_memory() -> Result<Self, DomainError> {
        let db = DbConnection::open_in_memory()?;
        let task_repo = TaskRepository::new(db.clone());
        let span_repo = SpanRepository::new(db.clone());
        let continuation_repo = ContinuationRepository::new(db.clone());
        Ok(Self {
            db,
            task_repo,
            span_repo,
            continuation_repo,
        })
    }

    /// Opens a file-backed SQLite store at `path` with all migrations applied.
    pub fn new(path: &str) -> Result<Self, DomainError> {
        let db = DbConnection::open(path)?;
        let task_repo = TaskRepository::new(db.clone());
        let span_repo = SpanRepository::new(db.clone());
        let continuation_repo = ContinuationRepository::new(db.clone());
        Ok(Self {
            db,
            task_repo,
            span_repo,
            continuation_repo,
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

    async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
        self.task_repo.save_task(task)
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

    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        self.continuation_repo.get_latest_continuation(task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core_domain::{SpanState, TaskStatus};

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
}
