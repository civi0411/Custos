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

pub struct WorkflowDispatcher {
    run_store: Option<Arc<dyn RunPort>>,
    /// All recorded claims indexed by claim_id
    claims: Arc<RwLock<HashMap<String, DispatchClaim>>>,
    /// Active claims index: key is `"{task_id}::{node_id_or_empty}"`, value is claim_id
    active_index: Arc<RwLock<HashMap<String, String>>>,
}

impl WorkflowDispatcher {
    pub fn new() -> Self {
        Self {
            run_store: None,
            claims: Arc::new(RwLock::new(HashMap::new())),
            active_index: Arc::new(RwLock::new(HashMap::new())),
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

        // 2. Check persistence for active worker runs if run_store is attached
        if let Some(ref store) = self.run_store {
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

        // 3. Atomic claim check under write lock
        let key = Self::index_key(task_id, node_id);
        let mut index_guard = self.active_index.write().await;
        let mut claims_guard = self.claims.write().await;

        if let Some(existing_claim_id) = index_guard.get(&key) {
            if let Some(existing_claim) = claims_guard.get(existing_claim_id) {
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

        // 4. Create and persist claim
        let mut claim = DispatchClaim::new(task_id, assignee, depth);
        if let Some(nid) = node_id {
            claim = claim.with_node_id(nid);
        }

        index_guard.insert(key, claim.id.clone());
        claims_guard.insert(claim.id.clone(), claim.clone());

        Ok(claim)
    }

    /// Releases an active claim, allowing the task/node to be claimed again.
    pub async fn release_claim(&self, claim_id: &str) -> Result<(), DomainError> {
        let mut claims_guard = self.claims.write().await;
        let claim = claims_guard
            .get_mut(claim_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "DispatchClaim".into(),
                id: claim_id.to_string(),
            })?;

        claim.release()?;

        let key = Self::index_key(&claim.task_id, claim.node_id.as_deref());
        let mut index_guard = self.active_index.write().await;
        if let Some(current_id) = index_guard.get(&key) {
            if current_id == claim_id {
                index_guard.remove(&key);
            }
        }

        Ok(())
    }

    /// Marks a pending claim as Dispatched and binds it to a WorkerRun ID.
    pub async fn mark_dispatched(
        &self,
        claim_id: &str,
        worker_run_id: &str,
    ) -> Result<(), DomainError> {
        let mut claims_guard = self.claims.write().await;
        let claim = claims_guard
            .get_mut(claim_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "DispatchClaim".into(),
                id: claim_id.to_string(),
            })?;

        claim.mark_dispatched(worker_run_id)
    }

    /// Retrieves a claim by its ID.
    pub async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError> {
        let guard = self.claims.read().await;
        Ok(guard.get(claim_id).cloned())
    }

    /// Retrieves an active claim for a given task and optional node.
    pub async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Option<DispatchClaim> {
        let key = Self::index_key(task_id, node_id);
        let index_guard = self.active_index.read().await;
        let claim_id = index_guard.get(&key)?;
        let claims_guard = self.claims.read().await;
        claims_guard.get(claim_id).cloned()
    }

    /// Lists all currently active claims (Pending or Dispatched).
    pub async fn list_active_claims(&self) -> Vec<DispatchClaim> {
        let guard = self.claims.read().await;
        guard
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
}
