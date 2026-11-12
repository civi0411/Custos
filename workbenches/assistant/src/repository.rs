//! Assistant Draft Repository (SQLite Implementation)
//!
//! Handles durable persistence of assistant communication drafts,
//! meeting proposals, and locked payloads for the Assistant Workbench.

use crate::domain::{AssistantDraft, AssistantDraftPort};
use async_trait::async_trait;
use custos_domain::DomainError;
use rusqlite::{params, OptionalExtension};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DbConnection(pub Arc<Mutex<rusqlite::Connection>>);

impl DbConnection {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self(conn)
    }

    pub fn open(path: &str) -> Result<Self, DomainError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn open_in_memory() -> Result<Self, DomainError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, DomainError> {
        self.0
            .lock()
            .map_err(|e| DomainError::Validation(format!("Database mutex poisoned: {e}")))
    }
}

#[derive(Clone)]
pub struct AssistantRepository {
    pub(crate) db: DbConnection,
}

impl AssistantRepository {
    pub fn new(db: DbConnection) -> Self {
        let repo = Self { db };
        if let Err(e) = repo.ensure_tables() {
            tracing::error!("Failed to initialize assistant schema: {e}");
        }
        repo
    }

    pub fn from_raw(writer: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self::new(DbConnection::new(writer))
    }

    pub fn ensure_tables(&self) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute_batch(include_str!("../migrations/assistant_schema.sql"))
            .map_err(|e| DomainError::Validation(format!("Failed to initialize assistant schema: {e}")))?;
        Ok(())
    }

    pub fn save_draft_sync(&self, draft: &AssistantDraft) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        tx.execute(
            "INSERT INTO assistant_drafts (id, task_id, session_id, title, kind, content, recipient, status, payload_lock, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                kind = excluded.kind,
                content = excluded.content,
                recipient = excluded.recipient,
                status = excluded.status,
                payload_lock = excluded.payload_lock,
                updated_at = excluded.updated_at",
            params![
                draft.id,
                draft.task_id,
                draft.session_id,
                draft.title,
                draft.kind,
                draft.content,
                draft.recipient,
                draft.status,
                draft.payload_lock,
                draft.created_at,
                draft.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(format!("Failed to save draft: {}", e)))?;
        let _ = tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("draft_event"),
                "assistant.draft.saved",
                serde_json::json!({
                    "draft_id": draft.id,
                    "session_id": draft.session_id,
                    "status": draft.status
                }).to_string(),
                chrono::Utc::now().timestamp()
            ],
        );
        tx.commit()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        Ok(())
    }

    pub fn get_draft_sync(&self, id: &str) -> Result<Option<AssistantDraft>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, session_id, title, kind, content, recipient, status, payload_lock, created_at, updated_at
                 FROM assistant_drafts WHERE id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let res = stmt
            .query_row(params![id], |row| {
                Ok(AssistantDraft {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    session_id: row.get(2)?,
                    title: row.get(3)?,
                    kind: row.get(4)?,
                    content: row.get(5)?,
                    recipient: row.get(6)?,
                    status: row.get(7)?,
                    payload_lock: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(res)
    }

    pub fn list_drafts_sync(
        &self,
        session_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<AssistantDraft>, DomainError> {
        let conn = self.db.lock()?;
        let mut query = "SELECT id, task_id, session_id, title, kind, content, recipient, status, payload_lock, created_at, updated_at FROM assistant_drafts WHERE 1=1".to_string();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(sid) = session_id {
            query.push_str(" AND session_id = ?");
            param_values.push(sid.to_string());
        }
        if let Some(tid) = task_id {
            query.push_str(" AND task_id = ?");
            param_values.push(tid.to_string());
        }
        query.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rusqlite_params = rusqlite::params_from_iter(param_values.iter());

        let rows = stmt
            .query_map(rusqlite_params, |row| {
                Ok(AssistantDraft {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    session_id: row.get(2)?,
                    title: row.get(3)?,
                    kind: row.get(4)?,
                    content: row.get(5)?,
                    recipient: row.get(6)?,
                    status: row.get(7)?,
                    payload_lock: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut drafts = Vec::new();
        for r in rows {
            drafts.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(drafts)
    }

    pub fn delete_draft_sync(&self, id: &str) -> Result<bool, DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let affected = tx
            .execute("DELETE FROM assistant_drafts WHERE id = ?1", params![id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if affected > 0 {
            let _ = tx.execute(
                "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    custos_domain::new_id("draft_event"),
                    "assistant.draft.deleted",
                    serde_json::json!({"draft_id": id}).to_string(),
                    chrono::Utc::now().timestamp()
                ],
            );
        }
        tx.commit()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        Ok(affected > 0)
    }
}

#[async_trait]
impl AssistantDraftPort for AssistantRepository {
    async fn save_draft(&self, draft: &AssistantDraft) -> Result<(), DomainError> {
        self.save_draft_sync(draft)
    }

    async fn get_draft(&self, id: &str) -> Result<Option<AssistantDraft>, DomainError> {
        self.get_draft_sync(id)
    }

    async fn list_drafts(
        &self,
        session_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<AssistantDraft>, DomainError> {
        self.list_drafts_sync(session_id, task_id)
    }

    async fn delete_draft(&self, id: &str) -> Result<bool, DomainError> {
        self.delete_draft_sync(id)
    }
}

