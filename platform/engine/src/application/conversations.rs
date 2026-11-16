//! Conversation & Interactive Session Application Service

use std::sync::Arc;
use custos_contracts::*;
use crate::conversation::SessionManager;
use crate::conversation::bridge::{BridgePort, BridgeService, SteerReceipt};

/// Application service orchestrating conversation sessions and external harness bridging.
pub struct ConversationApplicationService {
    session_manager: Arc<SessionManager>,
    bridge_service: Option<Arc<BridgeService>>,
}

impl ConversationApplicationService {
    pub fn new(
        session_manager: Arc<SessionManager>,
        bridge_service: Option<Arc<BridgeService>>,
    ) -> Self {
        Self {
            session_manager,
            bridge_service,
        }
    }

    pub async fn create_session(&self, mode: SessionMode) -> Session {
        self.session_manager.create_session(mode).await
    }

    pub async fn get_session(&self, id: &SessionId) -> Option<Session> {
        self.session_manager.get_session(id).await
    }

    pub async fn steer_task(
        &self,
        task_id: TaskId,
        session_id: SessionId,
        message: String,
    ) -> Result<SteerReceipt, DomainError> {
        if let Some(bridge) = &self.bridge_service {
            bridge.steer(task_id, session_id, message).await
        } else {
            Err(DomainError::Validation("Bridge service not initialized".to_string()))
        }
    }
}
