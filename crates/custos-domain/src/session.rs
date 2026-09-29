use crate::ids::TaskId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(format!("ses_{}", uuid::Uuid::new_v4().simple()))
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Bare,
    Assisted,
    Attached { task_id: TaskId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Promoted { task_id: TaskId },
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub mode: SessionMode,
    pub status: SessionStatus,
    pub current_goal: Option<String>,
    pub promotion_score: f32, // 0.0 - 1.0
    pub attached_to: Option<TaskId>,
    pub promoted_to: Option<TaskId>,
    pub created_at: String,
    pub updated_at: String,
}

impl Session {
    pub fn new(id: SessionId, mode: SessionMode) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            mode,
            status: SessionStatus::Active,
            current_goal: None,
            promotion_score: 0.0,
            attached_to: None,
            promoted_to: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionJournalEntry {
    pub entry_id: Option<i64>,
    pub session_id: SessionId,
    pub entry_type: String,
    pub entry_data: String,
    pub occurred_at: String,
}
