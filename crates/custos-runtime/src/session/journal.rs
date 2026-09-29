use custos_core_domain::{SessionId, SessionJournalEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionJournal {
    pub session_id: SessionId,
    pub entries: Vec<SessionJournalEntry>,
}

impl SessionJournal {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            entries: Vec::new(),
        }
    }

    pub fn append(&mut self, entry_type: impl Into<String>, entry_data: impl Into<String>) {
        let now = chrono::Utc::now().to_rfc3339();
        self.entries.push(SessionJournalEntry {
            entry_id: None,
            session_id: self.session_id.clone(),
            entry_type: entry_type.into(),
            entry_data: entry_data.into(),
            occurred_at: now,
        });
    }

    pub fn tool_call_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.entry_type == "tool_call")
            .count()
    }
}
