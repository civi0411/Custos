//! Custos Session Runtime
//!
//! Fast path interactive session manager supporting Bare and Assisted modes,
//! journaled interaction history, and automatic promotion score calculation.

pub mod journal;
pub mod manager;

pub use custos_core::SessionStore;
pub use journal::SessionJournal;
pub use manager::SessionManager;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_domain::{
        DomainError, Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct MockSessionStore {
        sessions: Mutex<HashMap<SessionId, Session>>,
        journals: Mutex<HashMap<SessionId, Vec<SessionJournalEntry>>>,
    }

    impl MockSessionStore {
        fn new() -> Self {
            Self {
                sessions: Mutex::new(HashMap::new()),
                journals: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl SessionStore for MockSessionStore {
        async fn get_session(
            &self,
            session_id: &SessionId,
        ) -> Result<Option<Session>, DomainError> {
            let lock = self.sessions.lock().unwrap();
            Ok(lock.get(session_id).cloned())
        }

        async fn list_sessions(&self) -> Result<Vec<Session>, DomainError> {
            let lock = self.sessions.lock().unwrap();
            let mut list: Vec<Session> = lock.values().cloned().collect();
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(list)
        }

        async fn save_session(&self, session: &Session) -> Result<(), DomainError> {
            let mut lock = self.sessions.lock().unwrap();
            lock.insert(session.id.clone(), session.clone());
            Ok(())
        }

        async fn delete_session(&self, session_id: &SessionId) -> Result<bool, DomainError> {
            let removed_session = self.sessions.lock().unwrap().remove(session_id).is_some();
            self.journals.lock().unwrap().remove(session_id);
            Ok(removed_session)
        }

        async fn append_journal(&self, entry: &SessionJournalEntry) -> Result<i64, DomainError> {
            let mut lock = self.journals.lock().unwrap();
            let list = lock.entry(entry.session_id.clone()).or_default();
            let id = list.len() as i64 + 1;
            let mut stored = entry.clone();
            stored.entry_id = Some(id);
            list.push(stored);
            Ok(id)
        }

        async fn get_journal(
            &self,
            session_id: &SessionId,
        ) -> Result<Vec<SessionJournalEntry>, DomainError> {
            let lock = self.journals.lock().unwrap();
            Ok(lock.get(session_id).cloned().unwrap_or_default())
        }
    }

    #[tokio::test]
    async fn test_session_manager_with_persistent_store() {
        let store = Arc::new(MockSessionStore::new());
        let manager = SessionManager::with_store(store.clone());

        let session = manager.create_session(SessionMode::Bare).await;
        assert_eq!(session.status, SessionStatus::Active);

        // Verify stored in MockSessionStore immediately
        assert!(store.get_session(&session.id).await.unwrap().is_some());

        // Append user message
        manager
            .append_user_message(&session.id, "Hello from test")
            .await
            .unwrap();

        // Verify journal in store
        let entries = store.get_journal(&session.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type, "user_message");
        assert_eq!(entries[0].entry_data, "Hello from test");

        // Record tool call -> check promotion score persisted in store
        manager
            .record_tool_call(&session.id, "tool_exec", "{}")
            .await
            .unwrap();

        let persisted = store.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(persisted.promotion_score, 0.15);

        // Test list_sessions
        let all = manager.list_sessions().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, session.id);

        // Test creating fresh manager pointing to same store (simulating daemon restart)
        let fresh_manager = SessionManager::with_store(store.clone());
        let restored = fresh_manager.get_session(&session.id).await.unwrap();
        assert_eq!(restored.id, session.id);
        assert_eq!(restored.promotion_score, 0.15);

        let restored_journal = fresh_manager.get_journal(&session.id).await.unwrap();
        assert_eq!(restored_journal.entries.len(), 2);
    }

    #[tokio::test]
    async fn test_session_lifecycle_and_promotion_score() {
        let manager = SessionManager::new();
        let session = manager.create_session(SessionMode::Bare).await;
        assert_eq!(session.status, SessionStatus::Active);

        // Append user and assistant messages
        manager
            .append_user_message(&session.id, "Hello Custos")
            .await
            .unwrap();
        manager
            .append_assistant_message(&session.id, "How can I help?")
            .await
            .unwrap();

        // Record tool calls
        manager
            .record_tool_call(&session.id, "list_files", "{}")
            .await
            .unwrap();
        manager
            .record_tool_call(&session.id, "read_file", r#"{"path":"test.rs"}"#)
            .await
            .unwrap();
        manager
            .record_tool_call(&session.id, "edit_file", r#"{"path":"test.rs"}"#)
            .await
            .unwrap();

        let updated = manager.get_session(&session.id).await.unwrap();
        assert!(updated.promotion_score > 0.0);

        // Test attach
        manager
            .attach_to_task(&session.id, "task_123".to_string())
            .await
            .unwrap();
        let attached = manager.get_session(&session.id).await.unwrap();
        assert_eq!(attached.attached_to, Some("task_123".to_string()));

        // Test transition to paused then active then closed
        let paused = manager
            .transition(&session.id, SessionStatus::Paused)
            .await
            .unwrap();
        assert_eq!(paused.status, SessionStatus::Paused);

        let active = manager
            .transition(&session.id, SessionStatus::Active)
            .await
            .unwrap();
        assert_eq!(active.status, SessionStatus::Active);

        let closed = manager
            .transition(&session.id, SessionStatus::Closed)
            .await
            .unwrap();
        assert_eq!(closed.status, SessionStatus::Closed);
    }

    #[tokio::test]
    async fn journal_updates_release_lock_before_updating_session() {
        let manager = SessionManager::new();
        let session = manager.create_session(SessionMode::Bare).await;

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.append_user_message(&session.id, "hello"),
        )
        .await
        .expect("appending a message must not deadlock")
        .expect("session should exist");

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            manager.record_tool_call(&session.id, "read_file", "{}"),
        )
        .await
        .expect("recording a tool call must not deadlock")
        .expect("session should exist");

        let updated = manager
            .get_session(&session.id)
            .await
            .expect("session exists");
        assert_eq!(updated.promotion_score, 0.15);
    }
}
