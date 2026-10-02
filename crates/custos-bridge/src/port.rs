use async_trait::async_trait;
use custos_domain::{DomainError, SessionId, Task, TaskContract, TaskId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachMode {
    Observe,
    Steer,
    ApproveOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteerReceipt {
    pub task_id: TaskId,
    pub session_id: SessionId,
    pub accepted: bool,
    pub message: String,
    pub timestamp: String,
}

#[async_trait]
pub trait BridgePort: Send + Sync {
    /// Promotes an active interactive Session to a durable, governed Task
    async fn promote(
        &self,
        session_id: SessionId,
        contract: TaskContract,
    ) -> Result<Task, DomainError>;

    /// Attaches an interactive session to a running task
    async fn attach(
        &self,
        session_id: SessionId,
        task_id: TaskId,
        mode: AttachMode,
    ) -> Result<(), DomainError>;

    /// Steers an attached running task with guidance from the session
    async fn steer(
        &self,
        task_id: TaskId,
        session_id: SessionId,
        message: String,
    ) -> Result<SteerReceipt, DomainError>;

    /// Observes the current execution state of an autonomous Task without steering
    async fn observe(&self, task_id: TaskId) -> Result<TaskObservation, DomainError>;

    /// Detaches an interactive session from an attached task
    async fn detach(&self, session_id: SessionId, task_id: TaskId) -> Result<(), DomainError>;

    /// Recalls an autonomous task back into an interactive session
    async fn recall(
        &self,
        task_id: TaskId,
        session_id: SessionId,
    ) -> Result<RecallReceipt, DomainError>;

    /// Forks a running task and its context into a new branched task and session
    async fn fork(
        &self,
        task_id: TaskId,
        new_session_id: SessionId,
    ) -> Result<ForkReceipt, DomainError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskObservation {
    pub task_id: TaskId,
    pub title: String,
    pub status: String,
    pub epoch: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallReceipt {
    pub task_id: TaskId,
    pub session_id: SessionId,
    pub accepted: bool,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkReceipt {
    pub original_task_id: TaskId,
    pub forked_task: Task,
    pub forked_session_id: SessionId,
    pub timestamp: String,
}
