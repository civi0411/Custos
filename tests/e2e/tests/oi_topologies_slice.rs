//! Orchestration Intelligence Topologies Slice (OI-5)
//!
//! Organized by thematic test modules:
//! - `t3_read_fan_out_topology`: T3 Read Fan-Out with JoinPolicy::CoverageThreshold.
//! - `t4_worktree_integrator_topology`: T4 Worktree Workers + Integrator with WorkspaceLease isolation and clean merge.
//! - `write_set_isolation_invariants`: Gate 5 compile-time rejection of concurrent overlapping write-sets.
//! - `t6_repair_loop_topology`: T6 Bounded Repair Loop with max iteration termination.

// ─────────────────────────────────────────────────────────────────────────────
// Theme 1: T3 Read Fan-Out Topology
// ─────────────────────────────────────────────────────────────────────────────
mod t3_read_fan_out_topology {
    use custos_core::oi::compiler::PlanCompiler;
    use custos_core::oi::join_policy::JoinEvaluation;
    use custos_domain::oi::{ExecutionTopology, StrategyProposal, WorkPacket};
    use custos_runtime::oi::topologies::T3ReadFanOutCoordinator;

    #[tokio::test]
    async fn test_coverage_threshold_fan_out_and_join() {
        let mut proposal = StrategyProposal::native_baseline("task_t3_e2e", "claude_code");
        proposal.chosen_topology = ExecutionTopology::T3ReadFanOut;
        proposal.estimated_tokens = 9000;

        let revision = PlanCompiler::compile(&proposal, 1);
        assert_eq!(revision.nodes.len(), 3);
        assert_eq!(revision.dependencies.len(), 2);

        let p1 = WorkPacket::new("task_t3_e2e", "reader_src", "Inspect src files", 1000);
        let p2 = WorkPacket::new("task_t3_e2e", "reader_tests", "Inspect test suites", 1000);
        let p3 = WorkPacket::new("task_t3_e2e", "reader_docs", "Inspect documentation", 1000);

        let eval = T3ReadFanOutCoordinator::execute_fan_out(&[p1, p2, p3], 0.75, "claude_code")
            .await
            .expect("fan out execution");

        match eval {
            JoinEvaluation::Approved { merged_payload, .. } => {
                assert!(merged_payload.contains("reader_src"));
                assert!(merged_payload.contains("reader_tests"));
            }
            _ => panic!("Expected approved coverage join evaluation"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 2: T4 Worktree Integrator Topology (WorkspaceLease Isolation)
// ─────────────────────────────────────────────────────────────────────────────
mod t4_worktree_integrator_topology {
    use std::fs;
    use custos_core::oi::compiler::PlanCompiler;
    use custos_core::oi::join_policy::JoinEvaluation;
    use custos_domain::oi::{ExecutionTopology, StrategyProposal, WorkerResult};
    use custos_runtime::oi::topologies::T4WorktreeCoordinator;
    use custos_runtime::workflow::lease::WorkspaceLeaseManager;

    #[tokio::test]
    async fn test_workspace_lease_isolation_and_clean_merge() {
        let temp_workspace = tempfile::tempdir().expect("create temp workspace");
        let lease_manager = WorkspaceLeaseManager::new(temp_workspace.path());

        let mut proposal = StrategyProposal::native_baseline("task_t4_e2e", "claude_code");
        proposal.chosen_topology = ExecutionTopology::T4Worktree;
        proposal.estimated_tokens = 12_000;

        let (revision, placements) = PlanCompiler::compile_with_placements(&proposal, 1);
        assert_eq!(revision.nodes.len(), 3);
        assert!(revision.obligations.contains(&"worktree_clean_merge".into()));

        let leases = T4WorktreeCoordinator::prepare_and_verify_leases(
            &revision.nodes,
            &revision.dependencies,
            &placements,
            &lease_manager,
        )
        .await
        .expect("leases must be verified and allocated without write conflict");

        assert_eq!(leases.len(), 2, "2 branch worker leases allocated");

        // Worker 1 writes in isolated Lease A
        let file_a = leases[0].lease_path.join("src").join("feature_a.rs");
        fs::create_dir_all(file_a.parent().unwrap()).unwrap();
        fs::write(&file_a, "pub fn feature_a() -> i32 { 42 }").unwrap();

        // Worker 2 writes in isolated Lease B
        let file_b = leases[1].lease_path.join("tests").join("test_b.rs");
        fs::create_dir_all(file_b.parent().unwrap()).unwrap();
        fs::write(&file_b, "#[test] fn test_b() { assert!(true); }").unwrap();

        let r1 = WorkerResult::success("branch_a", "Feature A generated");
        let r2 = WorkerResult::success("branch_b", "Test B generated");

        let join_eval =
            T4WorktreeCoordinator::integrate_branches(&leases, &lease_manager, &[r1, r2])
                .await
                .expect("integrate branches");

        assert!(matches!(join_eval, JoinEvaluation::Approved { .. }));

        let base_a = temp_workspace.path().join("src").join("feature_a.rs");
        let base_b = temp_workspace.path().join("tests").join("test_b.rs");
        assert!(base_a.exists(), "Feature A must be merged into base workspace");
        assert!(base_b.exists(), "Test B must be merged into base workspace");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 3: Write-Set Conflict Invariants (Gate 5 Invariant Enforcement)
// ─────────────────────────────────────────────────────────────────────────────
mod write_set_isolation_invariants {
    use custos_core::oi::join_policy::WriteSetConflictChecker;
    use custos_domain::workflow::RevisionNode;

    #[test]
    fn test_overlapping_concurrent_writes_blocked_without_leases() {
        let node_alpha = RevisionNode {
            node_id: "node_alpha".into(),
            step_name: "Coder Alpha".into(),
            role: "worker".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 3000,
            read_set: vec![],
            write_set: vec!["workspace/src/main.rs".into()],
            required_capabilities: vec![],
        };

        let node_beta = RevisionNode {
            node_id: "node_beta".into(),
            step_name: "Coder Beta".into(),
            role: "worker".into(),
            harness_id: "claude".into(),
            allocated_budget_tokens: 3000,
            read_set: vec![],
            write_set: vec!["workspace/src/main.rs".into()],
            required_capabilities: vec![],
        };

        let check = WriteSetConflictChecker::check_conflicts(
            &[node_alpha, node_beta],
            &[],
            None,
        );

        assert!(
            check.is_err(),
            "Must strictly block un-isolated write-set collision before execution starts"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 4: T6 Bounded Repair Loop Topology
// ─────────────────────────────────────────────────────────────────────────────
mod t6_repair_loop_topology {
    use custos_core::oi::join_policy::JoinEvaluation;
    use custos_domain::oi::WorkerResult;
    use custos_runtime::oi::topologies::T6RepairLoopCoordinator;

    #[test]
    fn test_bounded_iterations_and_retry_exhaustion() {
        // 1. Initial attempt fails unit test
        let eval_fail = WorkerResult::failed("eval_step", "assertion failed: `expected == actual`");
        let eval_round1 = T6RepairLoopCoordinator::evaluate_iteration(3, 1, &eval_fail);

        match eval_round1 {
            JoinEvaluation::RetryRequired { next_iteration, feedback } => {
                assert_eq!(next_iteration, 2);
                assert!(feedback.contains("assertion failed"));
            }
            _ => panic!("Expected repair retry required"),
        }

        // 2. Fix applied and evaluator succeeds
        let eval_success = WorkerResult::success("eval_step", "All test assertions passed successfully");
        let eval_round2 = T6RepairLoopCoordinator::evaluate_iteration(3, 2, &eval_success);

        assert!(matches!(eval_round2, JoinEvaluation::Approved { .. }));

        // 3. Persistent failure terminates at max iterations (3) without infinite retries
        let eval_exhausted = T6RepairLoopCoordinator::evaluate_iteration(3, 3, &eval_fail);
        assert!(
            matches!(eval_exhausted, JoinEvaluation::Rejected { .. }),
            "Must reject upon exceeding max bounded iterations"
        );
    }
}
