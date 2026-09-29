use crate::journal::SessionJournal;
use custos_core_domain::{DomainError, Session, SessionId, SessionMode, SessionStatus, TaskId};
use custos_kernel::{SessionStateMachine, SessionStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SessionManager {
    store: Option<Arc<dyn SessionStore>>,
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    journals: Arc<RwLock<HashMap<SessionId, SessionJournal>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Creates an in-memory session manager without durable backing (for testing).
    pub fn new() -> Self {
        Self {
            store: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            journals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a session manager backed by persistent storage (e.g. SQLite).
    pub fn with_store(store: Arc<dyn SessionStore>) -> Self {
        Self {
            store: Some(store),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            journals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(&self, mode: SessionMode) -> Session {
        let id = SessionId::generate();
        let session = Session::new(id.clone(), mode);
        let journal = SessionJournal::new(id.clone());

        if let Some(ref store) = self.store {
            if let Err(e) = store.save_session(&session).await {
                tracing::error!(session_id = %id, error = %e, "Failed to persist new session to store");
            }
        }

        let mut sess_lock = self.sessions.write().await;
        let mut journ_lock = self.journals.write().await;

        sess_lock.insert(id.clone(), session.clone());
        journ_lock.insert(id, journal);

        session
    }

    pub async fn get_session(&self, id: &SessionId) -> Option<Session> {
        {
            let lock = self.sessions.read().await;
            if let Some(session) = lock.get(id) {
                return Some(session.clone());
            }
        }

        if let Some(ref store) = self.store {
            if let Ok(Some(session)) = store.get_session(id).await {
                let mut sess_lock = self.sessions.write().await;
                sess_lock.insert(id.clone(), session.clone());
                return Some(session);
            }
        }

        None
    }

    pub async fn get_journal(&self, id: &SessionId) -> Option<SessionJournal> {
        {
            let lock = self.journals.read().await;
            if let Some(journal) = lock.get(id) {
                return Some(journal.clone());
            }
        }

        if let Some(ref store) = self.store {
            if let Ok(entries) = store.get_journal(id).await {
                let mut journal = SessionJournal::new(id.clone());
                journal.entries = entries;
                let mut journ_lock = self.journals.write().await;
                journ_lock.insert(id.clone(), journal.clone());
                return Some(journal);
            }
        }

        None
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>, DomainError> {
        if let Some(ref store) = self.store {
            store.list_sessions().await
        } else {
            let lock = self.sessions.read().await;
            let mut list: Vec<Session> = lock.values().cloned().collect();
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(list)
        }
    }

    pub async fn append_user_message(
        &self,
        id: &SessionId,
        message: &str,
    ) -> Result<(), DomainError> {
        self.ensure_session_loaded(id).await?;

        let (entry, tool_call_count) = {
            let mut journals = self.journals.write().await;
            let journal = journals.get_mut(id).ok_or_else(|| DomainError::NotFound {
                kind: "Session".into(),
                id: id.to_string(),
            })?;
            journal.append("user_message", message);
            let entry = journal
                .entries
                .last()
                .cloned()
                .ok_or_else(|| DomainError::InvariantViolation("empty journal entry".into()))?;
            (entry, journal.tool_call_count())
        };

        if let Some(ref store) = self.store {
            store.append_journal(&entry).await?;
        }

        self.update_promotion_score(id, tool_call_count).await?;
        Ok(())
    }

    pub async fn append_assistant_message(
        &self,
        id: &SessionId,
        message: &str,
    ) -> Result<(), DomainError> {
        self.ensure_session_loaded(id).await?;

        let entry = {
            let mut journ_lock = self.journals.write().await;
            let journal = journ_lock
                .get_mut(id)
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Session".into(),
                    id: id.to_string(),
                })?;
            journal.append("assistant_message", message);
            journal
                .entries
                .last()
                .cloned()
                .ok_or_else(|| DomainError::InvariantViolation("empty journal entry".into()))?
        };

        if let Some(ref store) = self.store {
            store.append_journal(&entry).await?;
        }

        Ok(())
    }

    pub async fn record_tool_call(
        &self,
        id: &SessionId,
        tool_name: &str,
        payload: &str,
    ) -> Result<(), DomainError> {
        self.ensure_session_loaded(id).await?;

        let (entry, tool_call_count) = {
            let mut journals = self.journals.write().await;
            let journal = journals.get_mut(id).ok_or_else(|| DomainError::NotFound {
                kind: "Session".into(),
                id: id.to_string(),
            })?;
            journal.append("tool_call", format!("{tool_name}:{payload}"));
            let entry = journal
                .entries
                .last()
                .cloned()
                .ok_or_else(|| DomainError::InvariantViolation("empty journal entry".into()))?;
            (entry, journal.tool_call_count())
        };

        if let Some(ref store) = self.store {
            store.append_journal(&entry).await?;
        }

        self.update_promotion_score(id, tool_call_count).await?;
        Ok(())
    }

    pub async fn transition(
        &self,
        id: &SessionId,
        next: SessionStatus,
    ) -> Result<Session, DomainError> {
        self.ensure_session_loaded(id).await?;

        let session = {
            let mut sess_lock = self.sessions.write().await;
            let sess = sess_lock.get_mut(id).ok_or_else(|| DomainError::NotFound {
                kind: "Session".into(),
                id: id.to_string(),
            })?;
            let updated = SessionStateMachine::transition(&sess.status, next)?;
            sess.status = updated;
            sess.updated_at = chrono::Utc::now().to_rfc3339();
            sess.clone()
        };

        if let Some(ref store) = self.store {
            store.save_session(&session).await?;
        }

        Ok(session)
    }

    pub async fn attach_to_task(&self, id: &SessionId, task_id: TaskId) -> Result<(), DomainError> {
        self.ensure_session_loaded(id).await?;

        let session = {
            let mut sess_lock = self.sessions.write().await;
            let sess = sess_lock.get_mut(id).ok_or_else(|| DomainError::NotFound {
                kind: "Session".into(),
                id: id.to_string(),
            })?;
            sess.mode = SessionMode::Attached {
                task_id: task_id.clone(),
            };
            sess.attached_to = Some(task_id);
            sess.updated_at = chrono::Utc::now().to_rfc3339();
            sess.clone()
        };

        if let Some(ref store) = self.store {
            store.save_session(&session).await?;
        }

        Ok(())
    }

    pub async fn detach_from_task(&self, id: &SessionId) -> Result<(), DomainError> {
        self.ensure_session_loaded(id).await?;

        let session = {
            let mut sess_lock = self.sessions.write().await;
            let sess = sess_lock.get_mut(id).ok_or_else(|| DomainError::NotFound {
                kind: "Session".into(),
                id: id.to_string(),
            })?;
            sess.mode = SessionMode::Assisted;
            sess.attached_to = None;
            sess.updated_at = chrono::Utc::now().to_rfc3339();
            sess.clone()
        };

        if let Some(ref store) = self.store {
            store.save_session(&session).await?;
        }

        Ok(())
    }

    async fn update_promotion_score(
        &self,
        id: &SessionId,
        tool_call_count: usize,
    ) -> Result<(), DomainError> {
        let session = {
            let mut sessions = self.sessions.write().await;
            if let Some(sess) = sessions.get_mut(id) {
                // Heuristic promotion score: each tool call adds 0.15, max 1.0
                sess.promotion_score = (tool_call_count as f32 * 0.15).min(1.0);
                sess.updated_at = chrono::Utc::now().to_rfc3339();
                Some(sess.clone())
            } else {
                None
            }
        };

        if let (Some(sess), Some(ref store)) = (session, &self.store) {
            store.save_session(&sess).await?;
        }

        Ok(())
    }

    async fn ensure_session_loaded(&self, id: &SessionId) -> Result<(), DomainError> {
        {
            let sessions = self.sessions.read().await;
            let journals = self.journals.read().await;
            if sessions.contains_key(id) && journals.contains_key(id) {
                return Ok(());
            }
        }

        if let Some(ref store) = self.store {
            if let Some(sess) = store.get_session(id).await? {
                let entries = store.get_journal(id).await.unwrap_or_default();
                let mut journal = SessionJournal::new(id.clone());
                journal.entries = entries;

                let mut sessions = self.sessions.write().await;
                let mut journals = self.journals.write().await;
                sessions.insert(id.clone(), sess);
                journals.insert(id.clone(), journal);
            }
        }

        Ok(())
    }
}
