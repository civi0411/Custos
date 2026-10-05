//! Task Domain Events
//!
//! Immutable records of state transitions and lifecycle changes.

use chrono::{DateTime, Utc};
use custos_domain::{Task, TaskContract, TaskStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCreated {
    pub task_id: String,
    pub title: String,
    #[serde(default)]
    pub contract: Option<TaskContract>,
    #[serde(default = "empty_metadata")]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

fn empty_metadata() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskAdvanced {
    pub task_id: String,
    pub from_status: TaskStatus,
    pub to_status: TaskStatus,
    pub new_epoch: u64,
    pub advanced_at: DateTime<Utc>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBlocked {
    pub task_id: String,
    pub from_status: TaskStatus,
    pub new_epoch: u64,
    pub reason: String,
    pub blocked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCompleted {
    pub task_id: String,
    pub final_epoch: u64,
    pub summary: String,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCancelled {
    pub task_id: String,
    pub final_epoch: u64,
    pub reason: String,
    pub cancelled_at: DateTime<Utc>,
}

/// One-time seed preserving a task snapshot created before event journaling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSnapshotImported {
    pub task: Task,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum TaskEvent {
    Created(TaskCreated),
    Advanced(TaskAdvanced),
    Blocked(TaskBlocked),
    Completed(TaskCompleted),
    Cancelled(TaskCancelled),
    SnapshotImported(TaskSnapshotImported),
}

impl TaskEvent {
    pub fn task_id(&self) -> &str {
        match self {
            TaskEvent::Created(e) => &e.task_id,
            TaskEvent::Advanced(e) => &e.task_id,
            TaskEvent::Blocked(e) => &e.task_id,
            TaskEvent::Completed(e) => &e.task_id,
            TaskEvent::Cancelled(e) => &e.task_id,
            TaskEvent::SnapshotImported(e) => &e.task.id,
        }
    }

    pub fn sequence(&self) -> u64 {
        match self {
            TaskEvent::Created(_) => 0,
            TaskEvent::Advanced(event) => event.new_epoch,
            TaskEvent::Blocked(event) => event.new_epoch,
            TaskEvent::Completed(event) => event.final_epoch,
            TaskEvent::Cancelled(event) => event.final_epoch,
            TaskEvent::SnapshotImported(event) => event.task.epoch,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            TaskEvent::Created(_) => "task_created",
            TaskEvent::Advanced(_) => "task_advanced",
            TaskEvent::Blocked(_) => "task_blocked",
            TaskEvent::Completed(_) => "task_completed",
            TaskEvent::Cancelled(_) => "task_cancelled",
            TaskEvent::SnapshotImported(_) => "task_snapshot_imported",
        }
    }
}
