//! Custos Session-Task Bridge
//!
//! Connects interactive sessions with durable autonomous tasks through:
//! - Promotion: Elevating an interactive conversation into a verifiable task
//! - Attachment: Linking a live session to an existing task for observation/steering
//! - Steering: Sending mid-flight guidance from a session into an active task

pub mod port;
pub mod service;

pub use port::{AttachMode, BridgePort, SteerReceipt};
pub use service::BridgeService;

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core::kernel::{TaskService, TaskStore};
    use custos_domain::{
        ContractEvidence, EvidenceKind, SessionMode, SessionStatus, TaskContract,
    };
    use custos_persistence::SqliteTaskStore;
    use custos_runtime::session::SessionManager;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_bridge_promote_and_steer() {
        let session_manager = Arc::new(SessionManager::new());
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store));
        let bridge = BridgeService::new(session_manager.clone(), task_service.clone());

        // 1. Create a session
        let session = session_manager.create_session(SessionMode::Bare).await;
        assert_eq!(session.status, SessionStatus::Active);

        // 2. Promote session to task
        let contract = TaskContract {
            pack_id: "engineering".to_string(),
            name: "Refactor Auth Module".to_string(),
            description: "Promoted autonomous refactor task".to_string(),
            required_capabilities: vec!["fs_read".into(), "fs_write".into()],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::FileAnchor,
                required: true,
            }],
        };

        let task = bridge.promote(session.id.clone(), contract).await.unwrap();
        assert_eq!(task.title, "Refactor Auth Module");

        // Verify session status updated to Promoted
        let updated_session = session_manager.get_session(&session.id).await.unwrap();
        assert_eq!(
            updated_session.status,
            SessionStatus::Promoted {
                task_id: task.id.clone()
            }
        );

        // 3. Test steering
        let receipt = bridge
            .steer(
                task.id.clone(),
                session.id.clone(),
                "Use Argon2 instead of PBKDF2".to_string(),
            )
            .await
            .unwrap();
        assert!(receipt.accepted);
        assert_eq!(receipt.task_id, task.id);
        assert_eq!(receipt.session_id, session.id);
    }

    #[tokio::test]
    async fn promotion_persists_the_full_task_contract() {
        let session_manager = Arc::new(SessionManager::new());
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let bridge = BridgeService::new(session_manager.clone(), task_service);
        let session = session_manager.create_session(SessionMode::Bare).await;
        let contract = TaskContract {
            pack_id: "research".into(),
            name: "Compare two papers".into(),
            description: "Compare claims and cite sources".into(),
            required_capabilities: vec!["web_read".into()],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::FileAnchor,
                required: true,
            }],
        };

        let task = bridge.promote(session.id, contract).await.unwrap();
        let persisted = store.get_task(&task.id).await.unwrap().unwrap();
        let persisted_contract = persisted.contract.unwrap();
        assert_eq!(persisted_contract.pack_id, "research");
        assert_eq!(persisted_contract.required_capabilities, vec!["web_read"]);
        assert_eq!(persisted_contract.evidence_requirements.len(), 1);
    }

    #[tokio::test]
    async fn test_bridge_observe_detach_recall_fork() {
        let session_manager = Arc::new(SessionManager::new());
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let bridge = BridgeService::new(session_manager.clone(), task_service);

        let session = session_manager.create_session(SessionMode::Bare).await;
        let contract = TaskContract {
            pack_id: "engineering".into(),
            name: "Build pipeline".into(),
            description: "Build steps".into(),
            required_capabilities: vec![],
            evidence_requirements: vec![],
        };

        // 1. Promote
        let task = bridge.promote(session.id.clone(), contract).await.unwrap();

        // 2. Observe
        let observation = bridge.observe(task.id.clone()).await.unwrap();
        assert_eq!(observation.task_id, task.id);
        assert_eq!(observation.title, "Build pipeline");

        // 3. Attach and Detach
        let session2 = session_manager.create_session(SessionMode::Bare).await;
        bridge
            .attach(session2.id.clone(), task.id.clone(), AttachMode::Observe)
            .await
            .unwrap();
        bridge
            .detach(session2.id.clone(), task.id.clone())
            .await
            .unwrap();

        // 4. Recall
        let recall = bridge
            .recall(task.id.clone(), session.id.clone())
            .await
            .unwrap();
        assert!(recall.accepted);

        // 5. Fork
        let session3 = session_manager.create_session(SessionMode::Bare).await;
        let fork = bridge
            .fork(task.id.clone(), session3.id.clone())
            .await
            .unwrap();
        assert_eq!(fork.original_task_id, task.id);
        assert_eq!(fork.forked_task.title, "Build pipeline-fork");
    }
}
