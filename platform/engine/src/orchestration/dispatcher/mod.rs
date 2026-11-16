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

#[cfg(test)]
mod tests;

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
