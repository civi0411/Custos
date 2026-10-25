//! Sovereign Workflow Dispatcher (RFC 004 / Orca Fencing Parity)
//!
//! Provides atomic task and work packet claiming, generation fencing,
//! depth stamping, and double-dispatch prevention.
//!
//! Key Invariants:
//! 1. Atomic claim: Exactly one worker can claim a ready task/node at any time.
//! 2. Depth stamping: Nesting depth must be >= 1 (unstamped or root-violating rows refused).
//! 3. Fencing: Double-dispatch is refused with `DomainError::Conflict`.
//! 4. Release & Reclaim: A released claim frees the task/node for subsequent claims.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use custos_core::contracts::storage::RunPort;
use custos_domain::{ClaimStatus, DispatchClaim, DomainError, RunStatus};

#[derive(Default)]
struct DispatcherState {
    /// All recorded claims indexed by claim_id
    claims: HashMap<String, DispatchClaim>,
    /// Active claims index: key is `"{task_id}::{node_id_or_empty}"`, value is claim_id
    active_index: HashMap<String, String>,
}

pub struct WorkflowDispatcher {
    run_store: Option<Arc<dyn RunPort>>,
    state: Arc<RwLock<DispatcherState>>,
}

impl WorkflowDispatcher {
    pub fn new() -> Self {
        Self {
            run_store: None,
            state: Arc::new(RwLock::new(DispatcherState::default())),
        }
    }

    pub fn with_run_store(mut self, run_store: Arc<dyn RunPort>) -> Self {
        self.run_store = Some(run_store);
        self
    }

    pub fn run_store(&self) -> Option<&Arc<dyn RunPort>> {
        self.run_store.as_ref()
    }

    fn index_key(task_id: &str, node_id: Option<&str>) -> String {
        format!("{}::{}", task_id, node_id.unwrap_or(""))
    }

    /// Synchronously restores active claims into in-memory state on startup.
    pub fn restore_from_active_claims(&self, claims: Vec<DispatchClaim>) -> usize {
        let count = claims.len();
        if let Ok(mut state) = self.state.try_write() {
            for claim in claims {
                let key = Self::index_key(&claim.task_id, claim.node_id.as_deref());
                state.active_index.insert(key, claim.id.clone());
                state.claims.insert(claim.id.clone(), claim);
            }
        }
        count
    }

    /// Restores active claims from persistent storage into in-memory state on startup.
    pub async fn restore_from_store(&self) -> Result<usize, DomainError> {
        if let Some(ref store) = self.run_store {
            let active = store.list_active_claims().await?;
            let mut state = self.state.write().await;
            let count = active.len();
            for claim in active {
                let key = Self::index_key(&claim.task_id, claim.node_id.as_deref());
                state.active_index.insert(key, claim.id.clone());
                state.claims.insert(claim.id.clone(), claim);
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Atomically claims a task or node for execution.
    ///
    /// Refuses claim if:
    /// - `depth < 1` (Orca `assertStampedDepth` requirement)
    /// - An active claim already exists for `(task_id, node_id)`
    /// - If `RunPort` is present and an active `WorkerRun` exists for `task_id`
    pub async fn claim_ready_task(
        &self,
        task_id: &str,
        node_id: Option<&str>,
        assignee: &str,
        depth: u32,
    ) -> Result<DispatchClaim, DomainError> {
        // 1. Invariant: Nesting depth must be stamped and >= 1
        if depth < 1 {
            return Err(DomainError::Validation(format!(
                "Refusing dispatch claim for task '{}': depth {} is invalid; expected >= 1",
                task_id, depth
            )));
        }

        // 2. Check persistence for active worker runs and active claims if run_store is attached
        if let Some(ref store) = self.run_store {
            // For whole-task claims (node_id is None), verify no active worker run exists
            if node_id.is_none() {
                let active_wruns = store.list_worker_runs_for_task(task_id).await?;
                if let Some(active) = active_wruns
                    .into_iter()
                    .find(|w| w.status == RunStatus::Active)
                {
                    return Err(DomainError::Conflict(format!(
                        "Task '{}' is already claimed and actively running under worker run '{}' (assigned to '{}')",
                        task_id, active.id, active.worker_id
                    )));
                }
            }

            if let Ok(Some(active_claim)) = store.get_active_claim(task_id, node_id).await {
                return Err(DomainError::Conflict(format!(
                    "Task '{}'{} is already claimed by '{}' (claim '{}', status {}) in persistent store",
                    task_id,
                    node_id
                        .map(|n| format!(" [node: {}]", n))
                        .unwrap_or_default(),
                    active_claim.assignee,
                    active_claim.id,
                    active_claim.status
                )));
            }
        }

        // 3. Check in-memory state under read lock
        let key = Self::index_key(task_id, node_id);
        {
            let state = self.state.read().await;
            if let Some(existing_claim_id) = state.active_index.get(&key) {
                if let Some(existing_claim) = state.claims.get(existing_claim_id) {
                    if existing_claim.status == ClaimStatus::Pending
                        || existing_claim.status == ClaimStatus::Dispatched
                    {
                        return Err(DomainError::Conflict(format!(
                            "Task '{}'{} is already claimed by '{}' (claim '{}', status {})",
                            task_id,
                            node_id
                                .map(|n| format!(" [node: {}]", n))
                                .unwrap_or_default(),
                            existing_claim.assignee,
                            existing_claim.id,
                            existing_claim.status
                        )));
                    }
                }
            }
        }

        // 4. Create claim
        let mut claim = DispatchClaim::new(task_id, assignee, depth);
        if let Some(nid) = node_id {
            claim = claim.with_node_id(nid);
        }

        // 5. Persist to store WITHOUT holding in-memory write lock
        if let Some(ref store) = self.run_store {
            store.claim_ready(&claim).await?;
        }

        // 6. Record in memory state under brief write lock
        let mut state = self.state.write().await;
        if self.run_store.is_none() {
            if let Some(existing_claim_id) = state.active_index.get(&key) {
                if let Some(existing_claim) = state.claims.get(existing_claim_id) {
                    if existing_claim.status == ClaimStatus::Pending
                        || existing_claim.status == ClaimStatus::Dispatched
                    {
                        return Err(DomainError::Conflict(format!(
                            "Task '{}'{} is already claimed by '{}' (claim '{}', status {})",
                            task_id,
                            node_id
                                .map(|n| format!(" [node: {}]", n))
                                .unwrap_or_default(),
                            existing_claim.assignee,
                            existing_claim.id,
                            existing_claim.status
                        )));
                    }
                }
            }
        }
        state.active_index.insert(key, claim.id.clone());
        state.claims.insert(claim.id.clone(), claim.clone());

        Ok(claim)
    }

    /// Releases an active claim, allowing the task/node to be claimed again.
    /// Does NOT hold in-memory cache locks across storage I/O.
    pub async fn release_claim(&self, claim_id: &str) -> Result<(), DomainError> {
        let claim_opt = {
            let state = self.state.read().await;
            state.claims.get(claim_id).cloned()
        };

        let mut claim = if let Some(c) = claim_opt {
            c
        } else if let Some(ref store) = self.run_store {
            store
                .get_claim(claim_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "DispatchClaim".into(),
                    id: claim_id.to_string(),
                })?
        } else {
            return Err(DomainError::NotFound {
                kind: "DispatchClaim".into(),
                id: claim_id.to_string(),
            });
        };

        claim.release()?;

        // Propagate store error: if persistence fails, in-memory active state remains untouched.
        // Storage I/O is performed WITHOUT holding the in-memory write lock.
        if let Some(ref store) = self.run_store {
            store.update_claim(&claim).await?;
        }

        let key = Self::index_key(&claim.task_id, claim.node_id.as_deref());
        let mut state = self.state.write().await;
        if let Some(current_id) = state.active_index.get(&key) {
            if current_id == claim_id {
                state.active_index.remove(&key);
            }
        }
        state.claims.insert(claim.id.clone(), claim);

        Ok(())
    }

    /// Marks a pending claim as Dispatched and binds it to a WorkerRun ID.
    /// Does NOT hold in-memory cache locks across storage I/O.
    pub async fn mark_dispatched(
        &self,
        claim_id: &str,
        worker_run_id: &str,
    ) -> Result<(), DomainError> {
        let claim_opt = {
            let state = self.state.read().await;
            state.claims.get(claim_id).cloned()
        };

        let mut claim = if let Some(c) = claim_opt {
            c
        } else if let Some(ref store) = self.run_store {
            store
                .get_claim(claim_id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    kind: "DispatchClaim".into(),
                    id: claim_id.to_string(),
                })?
        } else {
            return Err(DomainError::NotFound {
                kind: "DispatchClaim".into(),
                id: claim_id.to_string(),
            });
        };

        claim.mark_dispatched(worker_run_id)?;

        // Propagate store error: if persistence fails, in-memory state remains untouched.
        // Storage I/O is performed WITHOUT holding the in-memory write lock.
        if let Some(ref store) = self.run_store {
            store.update_claim(&claim).await?;
        }

        let mut state = self.state.write().await;
        state.claims.insert(claim.id.clone(), claim);
        Ok(())
    }

    /// Retrieves a claim by its ID, reading through to persistent store if not in memory.
    pub async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError> {
        let state = self.state.read().await;
        if let Some(claim) = state.claims.get(claim_id) {
            return Ok(Some(claim.clone()));
        }
        drop(state);

        if let Some(ref store) = self.run_store {
            return store.get_claim(claim_id).await;
        }
        Ok(None)
    }

    /// Retrieves an active claim for a given task and optional node.
    pub async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Option<DispatchClaim> {
        let key = Self::index_key(task_id, node_id);
        let state = self.state.read().await;
        if let Some(claim_id) = state.active_index.get(&key) {
            if let Some(claims) = state.claims.get(claim_id) {
                return Some(claims.clone());
            }
        }
        drop(state);

        if let Some(ref store) = self.run_store {
            if let Ok(Some(claim)) = store.get_active_claim(task_id, node_id).await {
                return Some(claim);
            }
        }
        None
    }

    /// Lists all currently active claims (Pending or Dispatched).
    pub async fn list_active_claims(&self) -> Vec<DispatchClaim> {
        let state = self.state.read().await;
        state
            .claims
            .values()
            .filter(|c| c.status == ClaimStatus::Pending || c.status == ClaimStatus::Dispatched)
            .cloned()
            .collect()
    }
}

impl Default for WorkflowDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        async fn save_worker_run(&self, _: &mut custos_domain::WorkerRun) -> Result<(), DomainError> {
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
            if self.fail_update.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(DomainError::Validation("Simulated storage write error".into()));
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
                .filter(|c| {
                    c.status == ClaimStatus::Pending || c.status == ClaimStatus::Dispatched
                })
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
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Attempt release -> must return Err
        let release_err = dispatcher.release_claim(&claim.id).await;
        assert!(release_err.is_err(), "release_claim must return Err on DB error");

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
            .store(false, std::sync::atomic::Ordering::SeqCst);
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
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let err = dispatcher.mark_dispatched(&claim.id, "worker_run_999").await;
        assert!(err.is_err(), "mark_dispatched must fail on store error");

        // In-memory claim status must remain Pending
        let active = dispatcher.get_active_claim("task_dispatch_err", None).await.unwrap();
        assert_eq!(active.status, ClaimStatus::Pending);
    }
}
