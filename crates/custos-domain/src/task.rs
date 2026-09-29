//! Task State Machine & Lifecycle
//!
//! Implements strict, auditable lifecycle transitions for tasks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    FileAnchor,
    SymbolAnchor,
    TestResult,
    Diff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractEvidence {
    pub kind: EvidenceKind,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskContract {
    pub pack_id: String,
    pub name: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub evidence_requirements: Vec<ContractEvidence>,
}

/// Version 1 Canonical Contract alias for SSOT.
pub type TaskContractV1 = TaskContract;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Queued,
    Running,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Draft => write!(f, "draft"),
            TaskStatus::Queued => write!(f, "queued"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Succeeded => write!(f, "succeeded"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(&self, next: TaskStatus) -> bool {
        match self {
            TaskStatus::Draft => matches!(next, TaskStatus::Queued | TaskStatus::Cancelled),
            TaskStatus::Queued => matches!(next, TaskStatus::Running | TaskStatus::Cancelled),
            TaskStatus::Running => matches!(
                next,
                TaskStatus::Blocked
                    | TaskStatus::Succeeded
                    | TaskStatus::Failed
                    | TaskStatus::Cancelled
            ),
            TaskStatus::Blocked => matches!(next, TaskStatus::Running | TaskStatus::Cancelled),
            TaskStatus::Succeeded | TaskStatus::Failed | TaskStatus::Cancelled => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRevision {
    pub revision: u64,
    pub goal: String,
    pub scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub base_source_version: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub epoch: u64,
    pub state_version: u64,
    pub active_revision: Option<TaskRevision>,
    pub revision_history: Vec<TaskRevision>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub contract: Option<TaskContract>,
    pub metadata: serde_json::Value,
}

impl Task {
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            status: TaskStatus::Draft,
            epoch: 0,
            state_version: 0,
            active_revision: None,
            revision_history: Vec::new(),
            created_at: now,
            updated_at: now,
            contract: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn transition(&mut self, next: TaskStatus) -> Result<(), DomainError> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidStateTransition {
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }
        self.status = next;
        self.epoch += 1;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn create_revision(
        &mut self,
        goal: String,
        scope: Vec<String>,
        acceptance_criteria: Vec<String>,
        base_source_version: Option<String>,
    ) -> &TaskRevision {
        let rev_num = self.active_revision.as_ref().map_or(1, |r| r.revision + 1);
        let rev = TaskRevision {
            revision: rev_num,
            goal,
            scope,
            acceptance_criteria,
            base_source_version,
            created_at: Utc::now(),
        };
        if let Some(prev) = self.active_revision.take() {
            self.revision_history.push(prev);
        }
        self.active_revision = Some(rev);
        self.state_version += 1;
        self.updated_at = Utc::now();
        self.active_revision.as_ref().unwrap()
    }

    pub fn is_stale_against(&self, current_source_version: &str) -> bool {
        if let Some(rev) = &self.active_revision {
            if let Some(base) = &rev.base_source_version {
                return base != current_source_version;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_task_transitions() {
        let mut t = Task::new("task_123".into(), "Test Task".into());
        assert_eq!(t.status, TaskStatus::Draft);
        assert!(t.transition(TaskStatus::Queued).is_ok());
        assert_eq!(t.epoch, 1);
        assert!(t.transition(TaskStatus::Running).is_ok());
        assert_eq!(t.epoch, 2);
        assert!(t.transition(TaskStatus::Succeeded).is_ok());
        assert_eq!(t.epoch, 3);
        assert!(t.status.is_terminal());

        // Cannot transition out of terminal state
        assert!(t.transition(TaskStatus::Running).is_err());
    }

    #[test]
    fn test_task_revision_and_stale_detection() {
        let mut t = Task::new("task_rev".into(), "Revision Task".into());
        assert_eq!(t.state_version, 0);
        assert!(t.active_revision.is_none());

        // Create first revision
        let rev1 = t.create_revision(
            "Fix bug in parser".into(),
            vec!["crates/parser".into()],
            vec!["cargo test passes".into()],
            Some("commit_sha_1".into()),
        );
        assert_eq!(rev1.revision, 1);
        assert_eq!(t.state_version, 1);
        assert!(!t.is_stale_against("commit_sha_1"));
        assert!(t.is_stale_against("commit_sha_2"));

        // Create second revision
        let rev2 = t.create_revision(
            "Fix bug in parser and add benchmark".into(),
            vec!["crates/parser".into(), "benches".into()],
            vec!["cargo test passes".into(), "bench passes".into()],
            Some("commit_sha_2".into()),
        );
        assert_eq!(rev2.revision, 2);
        assert_eq!(t.state_version, 2);
        assert_eq!(t.revision_history.len(), 1);
        assert_eq!(t.revision_history[0].revision, 1);
        assert!(!t.is_stale_against("commit_sha_2"));
        assert!(t.is_stale_against("commit_sha_3"));
    }
}
