use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{new_id, TaskId};
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

/// Presentation lens active when a canonical conversation turn was recorded.
/// A lens is not an authority scope and changing it does not create a Task or Run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchLens {
    Assistant,
    Coding,
    Research,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnActorKind {
    Human,
    Assistant,
    Tool,
    System,
}

/// Immutable identity and provenance for one entry in the canonical session journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: String,
    pub session_id: SessionId,
    pub actor_id: String,
    pub actor_kind: TurnActorKind,
    pub lens: WorkbenchLens,
    pub task_id: Option<TaskId>,
    pub task_revision: Option<u64>,
    pub model_attempt_id: Option<String>,
    pub resource_refs: Vec<String>,
    pub privacy_class: String,
    pub content: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

impl ConversationTurn {
    pub fn new(
        session_id: SessionId,
        actor_id: impl Into<String>,
        actor_kind: TurnActorKind,
        lens: WorkbenchLens,
        privacy_class: impl Into<String>,
        content: serde_json::Value,
    ) -> Result<Self, DomainError> {
        let actor_id = actor_id.into();
        let privacy_class = privacy_class.into();
        if actor_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Conversation turn actor_id cannot be empty".into(),
            ));
        }
        if privacy_class.trim().is_empty() {
            return Err(DomainError::Validation(
                "Conversation turn privacy_class cannot be empty".into(),
            ));
        }

        Ok(Self {
            id: new_id("turn"),
            session_id,
            actor_id,
            actor_kind,
            lens,
            task_id: None,
            task_revision: None,
            model_attempt_id: None,
            resource_refs: Vec::new(),
            privacy_class,
            content,
            occurred_at: Utc::now(),
        })
    }

    pub fn bind_task(mut self, task_id: TaskId, task_revision: u64) -> Result<Self, DomainError> {
        if task_id.trim().is_empty() || task_revision == 0 {
            return Err(DomainError::Validation(
                "Turn binding requires a task ID and non-zero revision".into(),
            ));
        }
        self.task_id = Some(task_id);
        self.task_revision = Some(task_revision);
        Ok(self)
    }
}

/// Durable attribution of a turn range to one Task revision.
/// Multiple bindings may exist for a session; the legacy `attached_to` field remains compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTaskBinding {
    pub id: String,
    pub session_id: SessionId,
    pub task_id: TaskId,
    pub task_revision: u64,
    pub from_turn_id: String,
    pub through_turn_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SessionTaskBinding {
    pub fn new(
        session_id: SessionId,
        task_id: TaskId,
        task_revision: u64,
        from_turn_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let from_turn_id = from_turn_id.into();
        if task_id.trim().is_empty() || from_turn_id.trim().is_empty() || task_revision == 0 {
            return Err(DomainError::Validation(
                "Session task binding requires task, revision, and first turn".into(),
            ));
        }

        Ok(Self {
            id: new_id("stbind"),
            session_id,
            task_id,
            task_revision,
            from_turn_id,
            through_turn_id: None,
            created_at: Utc::now(),
        })
    }

    pub fn close_at(&mut self, turn_id: impl Into<String>) -> Result<(), DomainError> {
        if self.through_turn_id.is_some() {
            return Err(DomainError::Conflict(format!(
                "Session task binding {} is already closed",
                self.id
            )));
        }
        let turn_id = turn_id.into();
        if turn_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Binding closing turn cannot be empty".into(),
            ));
        }
        self.through_turn_id = Some(turn_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_task_attribution_requires_a_real_revision() {
        let turn = ConversationTurn::new(
            SessionId::new("session_1"),
            "operator",
            TurnActorKind::Human,
            WorkbenchLens::Research,
            "user_private",
            serde_json::json!({"text": "Compare the selected sources"}),
        );
        assert!(turn.is_ok());
        let bound = turn.and_then(|value| value.bind_task("task_1".into(), 0));
        assert!(matches!(bound, Err(DomainError::Validation(_))));
    }

    #[test]
    fn binding_closes_only_once() {
        let mut binding =
            SessionTaskBinding::new(SessionId::new("session_1"), "task_1".into(), 2, "turn_1");
        assert!(binding.is_ok());
        if let Ok(ref mut value) = binding {
            assert!(value.close_at("turn_4").is_ok());
            assert!(matches!(
                value.close_at("turn_5"),
                Err(DomainError::Conflict(_))
            ));
        }
    }
}
