//! Orchestration Intelligence Replanner & S1 Fabric Slice (OI-6)
//!
//! Organized by thematic test modules:
//! - `delta_replanning_and_reachability`: Transitive reachability, node preservation, and delta replacement.
//! - `inv03_side_effects_guardrails`: Enforces blocking automated blind retries on Uncertain outbox effects.
//! - `s1_judge_calibration`: Calibrated confidence evaluations, criteria satisfaction, and S2 escalation.
//! - `s1_scout_and_micro_executor`: Fast read-only discovery and zero-risk preview execution with authority denial.

use custos_domain::workflow::RevisionNode;

fn make_node(id: &str, role: &str) -> RevisionNode {
    RevisionNode {
        node_id: id.into(),
        step_name: format!("Step {}", id),
        role: role.into(),
        harness_id: "claude_code".into(),
        allocated_budget_tokens: 2000,
        read_set: vec![],
        write_set: vec![],
        required_capabilities: vec![],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 1: Delta Replanning & Transitive Reachability
// ─────────────────────────────────────────────────────────────────────────────
mod delta_replanning_and_reachability {
    use super::*;
    use custos_domain::oi::ReplanTrigger;
    use custos_domain::workflow::WorkflowRevision;
    use custos_runtime::oi::replanner::Replanner;

    #[tokio::test]
    async fn test_transitive_delta_isolation_and_node_preservation() {
        let mut rev = WorkflowRevision::new("task-e2e-001", "prop-1", 1);
        rev.nodes = vec![
            make_node("node_a", "worker"),
            make_node("node_b", "worker"),
            make_node("node_c", "verifier"),
        ];
        rev.dependencies = vec![
            ("node_a".into(), "node_b".into()),
            ("node_b".into(), "node_c".into()),
        ];

        let brief = Replanner::create_brief(
            "task-e2e-001",
            &rev,
            ReplanTrigger::TestFailure,
            Some("node_b"),
            "Cargo build failed with exit code 1",
            &[],
        )
        .expect("create brief");

        assert_eq!(brief.preserved_node_ids, vec!["node_a"]);
        assert!(brief.affected_node_ids.contains(&"node_b".into()));
        assert!(brief.affected_node_ids.contains(&"node_c".into()));

        let repaired_b = make_node("node_b_repaired", "worker");
        let new_deps = vec![("node_a".into(), "node_b_repaired".into())];

        let new_rev = Replanner::apply_delta(&rev, &brief, "prop-2", vec![repaired_b], new_deps)
            .expect("apply delta");

        assert_eq!(new_rev.revision_number, 2);
        assert_eq!(new_rev.nodes.len(), 2);
        assert!(new_rev.nodes.iter().any(|n| n.node_id == "node_a"));
        assert!(new_rev.nodes.iter().any(|n| n.node_id == "node_b_repaired"));
        assert!(!new_rev.nodes.iter().any(|n| n.node_id == "node_b"));
        assert!(!new_rev.nodes.iter().any(|n| n.node_id == "node_c"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 2: INV-03 Side-Effect Guardrails
// ─────────────────────────────────────────────────────────────────────────────
mod inv03_side_effects_guardrails {
    use super::*;
    use custos_core::contracts::storage::{OutboxEntry, OutboxStatus};
    use custos_domain::oi::ReplanTrigger;
    use custos_domain::workflow::WorkflowRevision;
    use custos_runtime::oi::replanner::Replanner;

    #[tokio::test]
    async fn test_uncertain_outbox_effect_blocks_blind_retry() {
        let mut rev = WorkflowRevision::new("task-inv03-002", "prop-1", 1);
        rev.nodes = vec![make_node("node_dispatch_api", "worker")];

        let outbox = vec![OutboxEntry {
            id: "out-001".into(),
            task_id: "task-inv03-002".into(),
            action_id: "node_dispatch_api".into(),
            permit_id: "permit-001".into(),
            argument_digest: "digest-001".into(),
            idempotency_key: None,
            status: OutboxStatus::Uncertain,
            created_at: chrono::Utc::now(),
            receipt: None,
        }];

        let brief = Replanner::create_brief(
            "task-inv03-002",
            &rev,
            ReplanTrigger::Timeout,
            Some("node_dispatch_api"),
            "Connection reset while acknowledging API call",
            &outbox,
        )
        .expect("create brief with outbox");

        assert!(brief.reason.contains("[INV-03 Guardrail]"));
        assert!(brief.reason.contains("Uncertain external effect detected"));
        assert!(brief.reason.contains("Blind retry forbidden"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 3: S1 Judge Calibration & Abstain (< 0.75)
// ─────────────────────────────────────────────────────────────────────────────
mod s1_judge_calibration {
    use custos_core::contracts::judgment::{JudgmentPort, JudgmentRequest};
    use custos_runtime::cognitive::s1::{S1Judge, ABSTAIN_CONFIDENCE_THRESHOLD};
    use serde_json::json;

    #[tokio::test]
    async fn test_s1_judge_passes_clean_evidence() {
        let judge = S1Judge::new();
        let req = JudgmentRequest {
            judgment_id: "judge-001".into(),
            task_id: "task-001".into(),
            criteria: vec!["exit code 0".into(), "contains: built in".into()],
            candidate: json!({
                "status": "success",
                "exit_code": 0,
                "stdout": "cargo build finished, built in 1.4s"
            }),
            context: None,
        };
        let res = judge.judge(req).await.expect("judge pass");
        assert!(res.passed);
        assert!(res.confidence >= ABSTAIN_CONFIDENCE_THRESHOLD);
    }

    #[tokio::test]
    async fn test_s1_judge_abstains_on_uncertain_signal() {
        let judge = S1Judge::new();
        let req = JudgmentRequest {
            judgment_id: "judge-002".into(),
            task_id: "task-002".into(),
            criteria: vec!["success".into()],
            candidate: json!({
                "status": "uncertain",
                "stdout": "Process paused unexpectedly"
            }),
            context: None,
        };
        let res = judge.judge(req).await.expect("judge abstain");
        assert!(!res.passed);
        assert!(res.confidence < ABSTAIN_CONFIDENCE_THRESHOLD);
        assert!(res.rationale.contains("S1 Abstain"));
    }

    #[tokio::test]
    async fn test_s1_judge_rejects_explicit_violations() {
        let judge = S1Judge::new();
        let req = JudgmentRequest {
            judgment_id: "judge-003".into(),
            task_id: "task-003".into(),
            criteria: vec!["exit code 0".into()],
            candidate: json!({
                "status": "failed",
                "exit_code": 101,
                "stdout": "error[E0425]: cannot find value"
            }),
            context: None,
        };
        let res = judge.judge(req).await.expect("judge fail");
        assert!(!res.passed);
        assert!(res.violations.is_some());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 4: S1 Scout & Micro-Executor (Zero-Risk & INV-04/06 Invariants)
// ─────────────────────────────────────────────────────────────────────────────
mod s1_scout_and_micro_executor {
    use custos_domain::DomainError;
    use custos_runtime::cognitive::s1::{MicroExecutor, MicroTask, ProjectEcosystem, S1Scout};
    use tempfile::tempdir;

    #[test]
    fn test_s1_scout_read_only_discovery() {
        let dir = tempdir().expect("tempdir");
        let workspace_path = dir.path();

        std::fs::write(
            workspace_path.join("Cargo.toml"),
            "[workspace]\nmembers = [\"core\", \"runtime\"]",
        )
        .expect("write cargo toml");

        let scout = S1Scout::new();
        let report = scout.inspect(workspace_path).expect("scout report");

        assert_eq!(report.ecosystem, ProjectEcosystem::RustCargo);
        assert!(report.is_workspace);
        assert_eq!(report.recommended_topology, "T4Worktree");
        assert!(scout.write_set().is_empty());
        assert!(report.write_set.is_empty());
    }

    #[tokio::test]
    async fn test_micro_executor_preview_diff() {
        let dir = tempdir().expect("tempdir");
        let executor = MicroExecutor::new();

        let diff_receipt = executor
            .execute(
                MicroTask::PreviewDiff {
                    path: "src/lib.rs".into(),
                    original: "pub fn old() {}".into(),
                    proposed: "pub fn new() {}".into(),
                },
                dir.path(),
            )
            .await
            .expect("preview diff");

        assert!(diff_receipt.success);
        assert!(diff_receipt
            .preview_diff
            .unwrap()
            .contains("+pub fn new() {}"));
    }

    #[tokio::test]
    async fn test_micro_executor_inv04_inv06_authority_denial() {
        let dir = tempdir().expect("tempdir");
        let executor = MicroExecutor::new();

        let permit_err = executor
            .execute(
                MicroTask::IssuePermit {
                    capability: "authority.grant".into(),
                },
                dir.path(),
            )
            .await
            .expect_err("must block permit issuance");

        match permit_err {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("INV-04/INV-06 Violation"));
            }
            other => panic!("Expected DomainError::Unauthorized, got {:?}", other),
        }
    }
}
