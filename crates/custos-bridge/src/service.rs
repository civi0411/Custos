use crate::port::{
    AttachMode, BridgePort, ForkReceipt, RecallReceipt, SteerReceipt, TaskObservation,
};
use async_trait::async_trait;
use custos_core::{CreateTask, TaskService};
use custos_domain::{DomainError, SessionId, SessionStatus, Task, TaskContract, TaskId};
use custos_runtime::session::SessionManager;
use std::sync::Arc;

pub struct BridgeService {
    session_manager: Arc<SessionManager>,
    task_service: Arc<TaskService>,
}

impl BridgeService {
    pub fn new(session_manager: Arc<SessionManager>, task_service: Arc<TaskService>) -> Self {
        Self {
            session_manager,
            task_service,
        }
    }
}

#[async_trait]
impl BridgePort for BridgeService {
    async fn promote(
        &self,
        session_id: SessionId,
        contract: TaskContract,
    ) -> Result<Task, DomainError> {
        let session = self
            .session_manager
            .get_session(&session_id)
            .await
            .ok_or_else(|| DomainError::NotFound {
                kind: "Session".into(),
                id: session_id.to_string(),
            })?;

        if !session.is_active() {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", session.status),
                to: "Promoted".to_string(),
            });
        }

        // Create Task in Kernel
        let (task, _) = self
            .task_service
            .execute_create(CreateTask {
                title: contract.name.clone(),
                metadata: None,
                contract: Some(contract),
            })
            .await?;
        let task_id = task.id.clone();

        // Transition session to Promoted
        self.session_manager
            .transition(
                &session_id,
                SessionStatus::Promoted {
                    task_id: task_id.clone(),
                },
            )
            .await?;

        tracing::info!(
            session_id = %session_id,
            task_id = %task_id,
            "Successfully promoted Session to autonomous Task"
        );

        Ok(task)
    }

    async fn attach(
        &self,
        session_id: SessionId,
        task_id: TaskId,
        _mode: AttachMode,
    ) -> Result<(), DomainError> {
        self.session_manager
            .attach_to_task(&session_id, task_id.clone())
            .await?;

        tracing::info!(
            session_id = %session_id,
            task_id = %task_id,
            "Successfully attached Session to Task"
        );

        Ok(())
    }

    async fn steer(
        &self,
        task_id: TaskId,
        session_id: SessionId,
        message: String,
    ) -> Result<SteerReceipt, DomainError> {
        let now = chrono::Utc::now().to_rfc3339();

        // Record steering intent in session manager
        self.session_manager
            .append_user_message(&session_id, &format!("[STEER TASK {task_id}] {message}"))
            .await?;

        Ok(SteerReceipt {
            task_id,
            session_id,
            accepted: true,
            message,
            timestamp: now,
        })
    }

    async fn observe(&self, task_id: TaskId) -> Result<TaskObservation, DomainError> {
        let task =
            self.task_service
                .get_task(&task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: task_id.clone(),
                })?;

        Ok(TaskObservation {
            task_id: task.id.clone(),
            title: task.title,
            status: format!("{:?}", task.status),
            epoch: task.epoch,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn detach(&self, session_id: SessionId, task_id: TaskId) -> Result<(), DomainError> {
        self.session_manager.detach_from_task(&session_id).await?;
        tracing::info!(
            session_id = %session_id,
            task_id = %task_id,
            "Successfully detached Session from Task"
        );
        Ok(())
    }

    async fn recall(
        &self,
        task_id: TaskId,
        session_id: SessionId,
    ) -> Result<RecallReceipt, DomainError> {
        self.session_manager.detach_from_task(&session_id).await?;
        let now = chrono::Utc::now().to_rfc3339();
        tracing::info!(
            session_id = %session_id,
            task_id = %task_id,
            "Successfully recalled Task into Session"
        );
        Ok(RecallReceipt {
            task_id,
            session_id,
            accepted: true,
            timestamp: now,
        })
    }

    async fn fork(
        &self,
        task_id: TaskId,
        new_session_id: SessionId,
    ) -> Result<ForkReceipt, DomainError> {
        let original_task =
            self.task_service
                .get_task(&task_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "Task".into(),
                    id: task_id.clone(),
                })?;

        let forked_contract = original_task.contract.clone().map(|mut c| {
            c.name = format!("{}-fork", c.name);
            c
        });

        let (forked_task, _) = self
            .task_service
            .execute_create(CreateTask {
                title: format!("{}-fork", original_task.title),
                metadata: Some(original_task.metadata.clone()),
                contract: forked_contract,
            })
            .await?;

        self.session_manager
            .attach_to_task(&new_session_id, forked_task.id.clone())
            .await?;

        let now = chrono::Utc::now().to_rfc3339();
        Ok(ForkReceipt {
            original_task_id: task_id,
            forked_task,
            forked_session_id: new_session_id,
            timestamp: now,
        })
    }
}
