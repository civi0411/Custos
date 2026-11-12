use super::*;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn test_workflow_dispatcher_atomic_claim_and_fencing() {
    let dispatcher = WorkflowDispatcher::new();

    // 1. Worker Alpha claims task_1
    let claim_alpha = dispatcher
        .claim_ready_task("task_1", None, "worker_alpha", 1)
        .await
        .expect("Worker Alpha should successfully claim");
    assert_eq!(claim_alpha.status, ClaimStatus::Pending);
    assert_eq!(claim_alpha.assignee, "worker_alpha");
    assert_eq!(claim_alpha.depth, 1);

    // 2. Worker Beta tries to claim the same task_1 -> Must be REFUSED (Fencing)
    let err_beta = dispatcher
        .claim_ready_task("task_1", None, "worker_beta", 1)
        .await
        .expect_err("Worker Beta claim should be refused with Conflict");

    match err_beta {
        DomainError::Conflict(msg) => {
            assert!(msg.contains("task_1"));
            assert!(msg.contains("worker_alpha"));
        }
        other => panic!("Expected DomainError::Conflict, got: {:?}", other),
    }

    // 3. Mark dispatched
    assert!(dispatcher
        .mark_dispatched(&claim_alpha.id, "wrun_001")
        .await
        .is_ok());
    let updated = dispatcher
        .get_claim(&claim_alpha.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, ClaimStatus::Dispatched);
    assert_eq!(updated.worker_run_id, Some("wrun_001".into()));

    // Still active, Worker Gamma still blocked
    assert!(dispatcher
        .claim_ready_task("task_1", None, "worker_gamma", 1)
        .await
        .is_err());

    // 4. Release claim -> Now Worker Beta can claim
    assert!(dispatcher.release_claim(&claim_alpha.id).await.is_ok());

    let claim_beta = dispatcher
        .claim_ready_task("task_1", None, "worker_beta", 1)
        .await
        .expect("Worker Beta should succeed after release");
    assert_eq!(claim_beta.assignee, "worker_beta");
}

#[tokio::test]
async fn test_workflow_dispatcher_depth_validation() {
    let dispatcher = WorkflowDispatcher::new();

    // Depth 0 must be rejected
    let err = dispatcher
        .claim_ready_task("task_x", None, "worker_1", 0)
        .await
        .expect_err("Depth 0 must be rejected");

    match err {
        DomainError::Validation(msg) => assert!(msg.contains("depth 0 is invalid")),
        other => panic!("Expected DomainError::Validation, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_workflow_dispatcher_node_granularity() {
    let dispatcher = WorkflowDispatcher::new();

    // Distinct nodes for the same task can be claimed concurrently
    let claim_node1 = dispatcher
        .claim_ready_task("task_dag", Some("node_1"), "worker_1", 1)
        .await
        .unwrap();
    let claim_node2 = dispatcher
        .claim_ready_task("task_dag", Some("node_2"), "worker_2", 1)
        .await
        .unwrap();

    assert_eq!(claim_node1.node_id, Some("node_1".into()));
    assert_eq!(claim_node2.node_id, Some("node_2".into()));

    // Double claim on node_1 must be refused
    assert!(dispatcher
        .claim_ready_task("task_dag", Some("node_1"), "worker_3", 1)
        .await
        .is_err());
}

struct MockRunStore {
    fail_update: std::sync::atomic::AtomicBool,
    claims: Arc<RwLock<HashMap<String, DispatchClaim>>>,
}

impl MockRunStore {
    fn new() -> Self {
        Self {
            fail_update: std::sync::atomic::AtomicBool::new(false),
            claims: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl RunPort for MockRunStore {
    async fn save_run(&self, _: &mut custos_domain::Run) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_run(&self, _: &str) -> Result<Option<custos_domain::Run>, DomainError> {
        Ok(None)
    }
    async fn list_runs_for_task(
        &self,
        _: &str,
    ) -> Result<Vec<custos_domain::Run>, DomainError> {
        Ok(Vec::new())
    }
    async fn save_worker_run(
        &self,
        _: &mut custos_domain::WorkerRun,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_worker_run(
        &self,
        _: &str,
    ) -> Result<Option<custos_domain::WorkerRun>, DomainError> {
        Ok(None)
    }
    async fn list_worker_runs_for_task(
        &self,
        _: &str,
    ) -> Result<Vec<custos_domain::WorkerRun>, DomainError> {
        Ok(Vec::new())
    }
    async fn list_worker_runs_for_run(
        &self,
        _: &str,
    ) -> Result<Vec<custos_domain::WorkerRun>, DomainError> {
        Ok(Vec::new())
    }
    async fn claim_ready(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        self.claims
            .write()
            .await
            .insert(claim.id.clone(), claim.clone());
        Ok(())
    }
    async fn update_claim(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        if self.fail_update.load(Ordering::SeqCst) {
            return Err(DomainError::Validation(
                "Simulated storage write error".into(),
            ));
        }
        self.claims
            .write()
            .await
            .insert(claim.id.clone(), claim.clone());
        Ok(())
    }
    async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError> {
        Ok(self.claims.read().await.get(claim_id).cloned())
    }
    async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Result<Option<DispatchClaim>, DomainError> {
        let guard = self.claims.read().await;
        for c in guard.values() {
            if c.task_id == task_id
                && c.node_id.as_deref() == node_id
                && (c.status == ClaimStatus::Pending || c.status == ClaimStatus::Dispatched)
            {
                return Ok(Some(c.clone()));
            }
        }
        Ok(None)
    }
    async fn save_launch_attempt(
        &self,
        _: &custos_domain::LaunchAttempt,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_launch_attempt(
        &self,
        _: &str,
    ) -> Result<Option<custos_domain::LaunchAttempt>, DomainError> {
        Ok(None)
    }
    async fn list_active_claims(&self) -> Result<Vec<DispatchClaim>, DomainError> {
        let guard = self.claims.read().await;
        Ok(guard
            .values()
            .filter(|c| c.status == ClaimStatus::Pending || c.status == ClaimStatus::Dispatched)
            .cloned()
            .collect())
    }
}

#[tokio::test]
async fn test_storage_error_propagation_preserves_claim_state() {
    let store = Arc::new(MockRunStore::new());
    let dispatcher = WorkflowDispatcher::new().with_run_store(store.clone());

    let claim = dispatcher
        .claim_ready_task("task_err_test", None, "worker_1", 1)
        .await
        .expect("Initial claim succeeds");

    // Simulate database failure
    store
        .fail_update
        .store(true, Ordering::SeqCst);

    // Attempt release -> must return Err
    let release_err = dispatcher.release_claim(&claim.id).await;
    assert!(
        release_err.is_err(),
        "release_claim must return Err on DB error"
    );

    // In-memory claim must NOT be released and must remain active
    let active = dispatcher.get_active_claim("task_err_test", None).await;
    assert!(active.is_some(), "Claim must still be active in memory");
    assert_eq!(active.unwrap().status, ClaimStatus::Pending);

    // Conflicting claim from worker 2 must still be refused
    assert!(dispatcher
        .claim_ready_task("task_err_test", None, "worker_2", 1)
        .await
        .is_err());

    // Recover DB and retry release -> must succeed
    store
        .fail_update
        .store(false, Ordering::SeqCst);
    assert!(dispatcher.release_claim(&claim.id).await.is_ok());
    assert!(dispatcher
        .get_active_claim("task_err_test", None)
        .await
        .is_none());
}

#[tokio::test]
async fn test_read_through_and_startup_restore_from_store() {
    let store = Arc::new(MockRunStore::new());

    // Instance 1 claims a task
    let dispatcher1 = WorkflowDispatcher::new().with_run_store(store.clone());
    let claim = dispatcher1
        .claim_ready_task("task_restart", None, "worker_1", 1)
        .await
        .unwrap();

    // Instance 2 simulates fresh daemon startup with empty memory
    let dispatcher2 = WorkflowDispatcher::new().with_run_store(store.clone());

    // Read-through: get_claim reads from store
    let retrieved = dispatcher2.get_claim(&claim.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().assignee, "worker_1");

    // restore_from_store loads all active claims into memory
    let restored_count = dispatcher2.restore_from_store().await.unwrap();
    assert_eq!(restored_count, 1);

    // Active claims in dispatcher2 now blocks new claims
    let conflict = dispatcher2
        .claim_ready_task("task_restart", None, "worker_2", 1)
        .await;
    assert!(conflict.is_err(), "Should refuse conflict after restore");
}

#[tokio::test]
async fn test_restore_from_active_claims_synchronous() {
    let dispatcher = WorkflowDispatcher::new();
    let claim = DispatchClaim::new("task_sync_restore", "worker_1", 1);
    let restored = dispatcher.restore_from_active_claims(vec![claim.clone()]);
    assert_eq!(restored, 1);

    let active = dispatcher.get_active_claim("task_sync_restore", None).await;
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, claim.id);

    let conflict = dispatcher
        .claim_ready_task("task_sync_restore", None, "worker_2", 1)
        .await;
    assert!(conflict.is_err());
}

#[tokio::test]
async fn test_mark_dispatched_storage_error_preserves_state() {
    let store = Arc::new(MockRunStore::new());
    let dispatcher = WorkflowDispatcher::new().with_run_store(store.clone());

    let claim = dispatcher
        .claim_ready_task("task_dispatch_err", None, "worker_1", 1)
        .await
        .expect("Initial claim succeeds");

    store
        .fail_update
        .store(true, Ordering::SeqCst);

    let err = dispatcher
        .mark_dispatched(&claim.id, "worker_run_999")
        .await;
    assert!(err.is_err(), "mark_dispatched must fail on store error");

    // In-memory claim status must remain Pending
    let active = dispatcher
        .get_active_claim("task_dispatch_err", None)
        .await
        .unwrap();
    assert_eq!(active.status, ClaimStatus::Pending);
}
