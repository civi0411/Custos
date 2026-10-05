//! T4 Worktree Workers & Integrator Coordinator (RFC 004 §2A, G10)
//!
//! Allocates isolated workspace leases for concurrent branch workers, prevents write-set
//! conflicts prior to execution, and merges clean diffs into base workspace via Integrator.

use crate::workflow::lease::{WorkspaceLease, WorkspaceLeaseManager};
use custos_core::oi::join_policy::{
    JoinEvaluation, JoinEvaluator, JoinPolicy, WriteSetConflictChecker,
};
use custos_domain::oi::{NodePlacement, WorkerResult};
use custos_domain::workflow::RevisionNode;
use custos_domain::DomainError;

pub struct T4WorktreeCoordinator;

impl T4WorktreeCoordinator {
    /// Validates write-set isolation, prepares isolated leases, and coordinates execution.
    pub async fn prepare_and_verify_leases(
        nodes: &[RevisionNode],
        dependencies: &[(String, String)],
        placements: &[NodePlacement],
        lease_manager: &WorkspaceLeaseManager,
    ) -> Result<Vec<WorkspaceLease>, DomainError> {
        // Enforce Gate 5 Write-Set Conflict Checker: must fail if concurrent nodes lack leases
        WriteSetConflictChecker::check_conflicts(nodes, dependencies, Some(placements))?;

        let mut acquired = Vec::new();
        for placement in placements {
            if placement.workspace_lease_id.is_some() {
                let lease = lease_manager.acquire_lease("t4_task", &placement.node_id)?;
                acquired.push(lease);
            }
        }

        Ok(acquired)
    }

    /// Integrator merges diffs from all branch worker leases and evaluates via IntegrationOracle.
    pub async fn integrate_branches(
        leases: &[WorkspaceLease],
        lease_manager: &WorkspaceLeaseManager,
        branch_results: &[WorkerResult],
    ) -> Result<JoinEvaluation, DomainError> {
        // Check integration oracle on worker results
        let eval = JoinEvaluator::evaluate(&JoinPolicy::IntegrationOracle, branch_results);
        if let JoinEvaluation::Approved { .. } = &eval {
            // Merge isolated worktrees into base
            for lease in leases {
                lease_manager.merge_lease(lease, None)?;
            }
        }
        Ok(eval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_t4_worktree_coordination() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lease_manager = WorkspaceLeaseManager::new(temp_dir.path());

        let node_a = RevisionNode {
            node_id: "branch_a".into(),
            step_name: "Branch A".into(),
            role: "worker".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 2000,
            read_set: vec![],
            write_set: vec!["src/a.rs".into()],
            required_capabilities: vec![],
        };
        let node_b = RevisionNode {
            node_id: "branch_b".into(),
            step_name: "Branch B".into(),
            role: "worker".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 2000,
            read_set: vec![],
            write_set: vec!["tests/b.rs".into()],
            required_capabilities: vec![],
        };

        let placements = vec![
            NodePlacement {
                node_id: "branch_a".into(),
                role: "worker".into(),
                backend_harness: "claude".into(),
                budget_tokens_slice: 2000,
                workspace_lease_id: Some("lease_a".into()),
            },
            NodePlacement {
                node_id: "branch_b".into(),
                role: "worker".into(),
                backend_harness: "claude".into(),
                budget_tokens_slice: 2000,
                workspace_lease_id: Some("lease_b".into()),
            },
        ];

        let leases = T4WorktreeCoordinator::prepare_and_verify_leases(
            &[node_a, node_b],
            &[],
            &placements,
            &lease_manager,
        )
        .await
        .unwrap();

        assert_eq!(leases.len(), 2);

        let r1 = WorkerResult::success("branch_a", "Branch A diff ready");
        let r2 = WorkerResult::success("branch_b", "Branch B diff ready");

        let join_res =
            T4WorktreeCoordinator::integrate_branches(&leases, &lease_manager, &[r1, r2])
                .await
                .unwrap();

        assert!(matches!(join_res, JoinEvaluation::Approved { .. }));
    }
}
