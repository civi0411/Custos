//! Task Domain Events
//!
//! Immutable records of state transitions and lifecycle changes.

use chrono::{DateTime, Utc};
use custos_core_domain::TaskStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCreated {
    pub task_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TaskEvent {
    Created(TaskCreated),
    Advanced(TaskAdvanced),
    Blocked(TaskBlocked),
    Completed(TaskCompleted),
    Cancelled(TaskCancelled),
}

impl TaskEvent {
    pub fn task_id(&self) -> &str {
        match self {
            TaskEvent::Created(e) => &e.task_id,
            TaskEvent::Advanced(e) => &e.task_id,
            TaskEvent::Blocked(e) => &e.task_id,
            TaskEvent::Completed(e) => &e.task_id,
            TaskEvent::Cancelled(e) => &e.task_id,
        }
    }
}
