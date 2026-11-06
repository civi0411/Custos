//! Golden Acceptance Test G12: Measured Orchestration Intelligence & S1 vs. Pinned Baseline (F11)
//!
//! Invariant: "OI/S1 so pinned baseline, failures/human/recovery included | Không claim cheaper/quality-preserved nếu chưa đạt margin"
//!
//! Core Scenarios:
//! 1. Pinned Baseline Comparison & Invariant Enforcement:
//!    - Candidate runs cannot claim cost reduction if total economic cost (including failures, retries, and human time)
//!      exceeds baseline or fails the cost reduction margin.
//!    - Quality degradation beyond the non-inferiority tolerance margin is strictly rejected.
//!    - Small sample sizes output Inconclusive, preserving pinned baseline as default.
//!    - Genuine improvements meeting both quality and cost margins are verified.
//! 2. S1 Fast Judge Calibration & Uncompressed Escalation:
//!    - Evaluates clean structured evidence with high confidence (>= 0.75).
//!    - Abstains (< 0.75) on uncertain/ambiguous signals, escalating to S2/human review with uncompressed evidence.
//! 3. Delta Replanning Authority Preservation & INV-03 Guardrail:
//!    - Delta replanning isolates affected nodes while preserving settled nodes and committed obligations.
//!    - INV-03 guardrail: Uncertain outbox side-effects strictly block automated blind retries.
//! 4. Hard Filters & Candidate Catalog Pruning:
//!    - Candidates not present in configured available executors catalog are pruned and rejected.
//!    - Model pin, local-only, and egress constraints are strictly enforced.
//!    - Explicit representation of uncertainty: Estimator factors in unknowns and recovery buffers (never 0 instead of unknown).

use custos_core::contracts::judgment::{JudgmentPort, JudgmentRequest};
use custos_core::contracts::storage::{OutboxEntry, OutboxStatus};
use custos_core::oi::hard_filters::HardFilters;
use custos_domain::oi::{
    ComparisonMarginConfig, DecisionSnapshot, OptimizationClaimOutcome, OptimizationEvaluator,
    RejectionReason, ReplanTrigger, StrategyProposal, TaskExecutionMetrics,
};
use custos_domain::workflow::{RevisionNode, WorkflowRevision};
use custos_runtime::cognitive::s1::{S1Judge, ABSTAIN_CONFIDENCE_THRESHOLD};
use custos_runtime::oi::{CandidateBuilder, Estimator, Replanner};
use serde_json::json;

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1: Pinned Baseline Comparison & Invariant Enforcement (G12 Invariant)
// ─────────────────────────────────────────────────────────────────────────────
mod baseline_comparison_and_margin_enforcement {
    use super::*;

    fn make_run(
        task: &str,
        harness: &str,
        quality: f64,
        cost: f64,
        human_sec: u64,
        attempts: usize,
        recovery_cost: f64,
    ) -> TaskExecutionMetrics {
        TaskExecutionMetrics {
            task_id: task.into(),
            harness_id: harness.into(),
            success: quality >= 0.9,
            quality_score: quality,
            tokens_used: 2500,
            cost_usd: cost,
            latency_ms: 3500,
            human_intervention_seconds: human_sec,
            attempts_count: attempts,
            recovery_cost_usd: recovery_cost,
        }
    }

    #[test]
    fn test_g12_cannot_claim_cheaper_when_human_recovery_exceeds_token_savings() {
        // Pinned baseline (e.g. Claude 3.5 Sonnet / Governed native):
        // Highly competent, $0.12 per run, 0 human intervention seconds, 1 attempt, 100% quality.
        let baseline = vec![
            make_run("task-1", "claude-code", 1.0, 0.12, 0, 1, 0.0),
            make_run("task-2", "claude-code", 1.0, 0.12, 0, 1, 0.0),
            make_run("task-3", "claude-code", 1.0, 0.12, 0, 1, 0.0),
        ];

        // Candidate cheap model / naive agent:
        // Superficially claims $0.02 in tokens, but each task required 300 seconds (5 mins) of human fixing
        // and 2 retry attempts costing $0.04 extra.
        // At $60/hr, 300s = $5.00 of human time per task!
        let candidate = vec![
            make_run("task-1", "cheap-agent", 1.0, 0.02, 300, 3, 0.04),
            make_run("task-2", "cheap-agent", 1.0, 0.02, 300, 3, 0.04),
            make_run("task-3", "cheap-agent", 1.0, 0.02, 300, 3, 0.04),
        ];

        let config = ComparisonMarginConfig {
            quality_tolerance_margin: 0.05,
            cost_reduction_margin: 0.15, // Required >= 15% cost savings
            min_sample_size: 3,
            human_hourly_rate_usd: 60.0,
        };

        let report = OptimizationEvaluator::evaluate(&baseline, &candidate, &config);

        // G12 Assertion: Candidate must NOT claim cheaper when human recovery is factored in!
        assert!(matches!(
            report.outcome,
            OptimizationClaimOutcome::NotCheaper { .. }
        ));
        assert!(
            !report.outcome.is_admissible_optimization(),
            "Candidate with heavy human recovery cannot claim optimization"
        );
        assert!(report.avg_candidate_total_cost > report.avg_baseline_total_cost);
    }

    #[test]
    fn test_g12_cannot_claim_quality_preserved_when_criteria_fail_margin() {
        let baseline = vec![
            make_run("task-1", "claude-code", 1.0, 0.10, 0, 1, 0.0),
            make_run("task-2", "claude-code", 1.0, 0.10, 0, 1, 0.0),
            make_run("task-3", "claude-code", 1.0, 0.10, 0, 1, 0.0),
        ];

        // Candidate is cheap ($0.03) and 0 human time, but quality drops to 0.70 (violates 0.05 tolerance margin)
        let candidate = vec![
            make_run("task-1", "mini-model", 0.70, 0.03, 0, 1, 0.0),
            make_run("task-2", "mini-model", 0.72, 0.03, 0, 1, 0.0),
            make_run("task-3", "mini-model", 0.68, 0.03, 0, 1, 0.0),
        ];

        let config = ComparisonMarginConfig::default();
        let report = OptimizationEvaluator::evaluate(&baseline, &candidate, &config);

        // G12 Assertion: Must reject with QualityInferior
        match report.outcome {
            OptimizationClaimOutcome::QualityInferior {
                baseline_quality,
                candidate_quality,
                quality_delta,
            } => {
                assert_eq!(baseline_quality, 1.0);
                assert!(candidate_quality < 0.75);
                assert!(quality_delta < -config.quality_tolerance_margin);
            }
            other => panic!("Expected QualityInferior, got {:?}", other),
        }
        assert!(!report.outcome.is_admissible_optimization());
    }

    #[test]
    fn test_g12_inconclusive_sample_size_preserves_pinned_baseline_default() {
        // Only 1 paired sample; config requires at least 3
        let baseline = vec![make_run("task-1", "claude-code", 1.0, 0.10, 0, 1, 0.0)];
        let candidate = vec![make_run("task-1", "oi-candidate", 1.0, 0.05, 0, 1, 0.0)];

        let config = ComparisonMarginConfig {
            min_sample_size: 3,
            ..Default::default()
        };

        let report = OptimizationEvaluator::evaluate(&baseline, &candidate, &config);

        // G12 Assertion: Inconclusive samples fall back to baseline default
        assert!(matches!(
            report.outcome,
            OptimizationClaimOutcome::Inconclusive {
                samples_observed: 1,
                samples_required: 3
            }
        ));
        assert!(!report.outcome.is_admissible_optimization());
    }

    #[test]
    fn test_g12_approves_genuine_optimization_meeting_quality_and_cost_margins() {
        let baseline = vec![
            make_run("task-1", "claude-code", 1.0, 0.12, 0, 1, 0.0),
            make_run("task-2", "claude-code", 1.0, 0.12, 0, 1, 0.0),
            make_run("task-3", "claude-code", 1.0, 0.12, 0, 1, 0.0),
        ];

        // Candidate genuinely maintains 100% quality and cuts cost by 50% ($0.06 vs $0.12)
        // with 0 human intervention and 1 attempt
        let candidate = vec![
            make_run("task-1", "oi-triaged-harness", 1.0, 0.06, 0, 1, 0.0),
            make_run("task-2", "oi-triaged-harness", 1.0, 0.06, 0, 1, 0.0),
            make_run("task-3", "oi-triaged-harness", 1.0, 0.06, 0, 1, 0.0),
        ];

        let config = ComparisonMarginConfig {
            quality_tolerance_margin: 0.05,
            cost_reduction_margin: 0.15, // 15% required, 50% achieved
            min_sample_size: 3,
            human_hourly_rate_usd: 60.0,
        };

        let report = OptimizationEvaluator::evaluate(&baseline, &candidate, &config);

        match report.outcome {
            OptimizationClaimOutcome::QualityPreservedAndCheaper {
                quality_delta,
                cost_savings_ratio,
            } => {
                assert_eq!(quality_delta, 0.0);
                assert!((cost_savings_ratio - 0.50).abs() < 1e-3);
            }
            other => panic!("Expected QualityPreservedAndCheaper, got {:?}", other),
        }
        assert!(report.outcome.is_admissible_optimization());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2: S1 Fast Judge Calibration & Uncompressed Escalation
// ─────────────────────────────────────────────────────────────────────────────
mod s1_judge_calibration_and_escalation {
    use super::*;

    #[tokio::test]
    async fn test_g12_s1_judge_passes_clean_evidence_above_threshold() {
        let judge = S1Judge::new();
        let request = JudgmentRequest {
            judgment_id: "jdg-clean-001".into(),
            task_id: "task-clean".into(),
            criteria: vec!["exit code 0".into(), "contains: test result: ok".into()],
            candidate: json!({
                "status": "completed",
                "exit_code": 0,
                "stdout": "running 10 tests\ntest result: ok. 10 passed; 0 failed"
            }),
            context: None,
        };

        let result = judge.judge(request).await.expect("judge evaluation");
        assert!(result.passed);
        assert!(result.confidence >= ABSTAIN_CONFIDENCE_THRESHOLD);
        assert_eq!(result.score, 1.0);
        assert!(result.violations.is_none());
        assert!(result.rationale.contains("S1 Verified"));
    }

    #[tokio::test]
    async fn test_g12_s1_judge_abstains_on_uncertain_signal_preserving_evidence() {
        let judge = S1Judge::new();
        let request = JudgmentRequest {
            judgment_id: "jdg-abstain-002".into(),
            task_id: "task-ambiguous".into(),
            criteria: vec!["exit code 0".into(), "clean build".into()],
            candidate: json!({
                "status": "uncertain",
                "output": "partial execution interrupted by provider stream disconnect"
            }),
            context: None,
        };

        let result = judge.judge(request).await.expect("judge evaluation");

        // G12 Invariant: Confidence < 0.75 must abstain, NOT pass or fabricate verification
        assert!(!result.passed);
        assert!(result.confidence < ABSTAIN_CONFIDENCE_THRESHOLD);
        assert!(result.rationale.contains("S1 Abstain"));
        assert!(result.rationale.contains("escalating to S2"));
    }

    #[tokio::test]
    async fn test_g12_s1_judge_rejects_explicit_violations_with_detailed_breakdown() {
        let judge = S1Judge::new();
        let request = JudgmentRequest {
            judgment_id: "jdg-fail-003".into(),
            task_id: "task-failing".into(),
            criteria: vec!["exit code 0".into(), "contains: BUILD SUCCESSFUL".into()],
            candidate: json!({
                "status": "failed",
                "exit_code": 1,
                "stderr": "error: cannot find symbol in workspace"
            }),
            context: None,
        };

        let result = judge.judge(request).await.expect("judge evaluation");
        assert!(!result.passed);
        assert!(result.confidence >= ABSTAIN_CONFIDENCE_THRESHOLD);
        assert!(result.violations.is_some());
        let violations = result.violations.unwrap();
        assert!(violations.iter().any(|v| v.contains("exit_code is 1")));
        assert!(violations
            .iter()
            .any(|v| v.contains("missing required token 'build successful'")));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3: Delta Replanning Authority Preservation & INV-03 Guardrail
// ─────────────────────────────────────────────────────────────────────────────
mod delta_replanning_and_authority_preservation {
    use super::*;

    fn make_revision_node(id: &str, role: &str) -> RevisionNode {
        RevisionNode {
            node_id: id.into(),
            step_name: format!("Step {}", id),
            role: role.into(),
            harness_id: "claude-code".into(),
            allocated_budget_tokens: 3000,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        }
    }

    #[tokio::test]
    async fn test_g12_replan_preserves_settled_nodes_and_obligations() {
        let mut rev = WorkflowRevision::new("task-replan-001", "prop-1", 1);
        rev.nodes = vec![
            make_revision_node("node_read_repo", "reader"),
            make_revision_node("node_modify_code", "worker"),
            make_revision_node("node_run_tests", "verifier"),
        ];
        rev.dependencies = vec![
            ("node_read_repo".into(), "node_modify_code".into()),
            ("node_modify_code".into(), "node_run_tests".into()),
        ];
        rev.obligations = vec!["permit.fs.write".into(), "criteria.tests_pass".into()];

        // Downstream verifier fails due to test assertion failure
        let brief = Replanner::create_brief(
            "task-replan-001",
            &rev,
            ReplanTrigger::TestFailure,
            Some("node_modify_code"),
            "Unit tests failed on modified file",
            &[],
        )
        .expect("create replan brief");

        // Upstream reader is preserved; worker and downstream verifier are isolated for replan
        assert_eq!(brief.preserved_node_ids, vec!["node_read_repo"]);
        assert!(brief.affected_node_ids.contains(&"node_modify_code".into()));
        assert!(brief.affected_node_ids.contains(&"node_run_tests".into()));

        // Apply replacement delta
        let repaired_worker = make_revision_node("node_modify_code_v2", "worker");
        let repaired_verifier = make_revision_node("node_run_tests_v2", "verifier");
        let new_nodes = vec![repaired_worker, repaired_verifier];
        let new_deps = vec![
            ("node_read_repo".into(), "node_modify_code_v2".into()),
            ("node_modify_code_v2".into(), "node_run_tests_v2".into()),
        ];

        let new_rev = Replanner::apply_delta(&rev, &brief, "prop-2", new_nodes, new_deps)
            .expect("apply delta");

        // G12 Invariant: Revision bumped, settled node preserved, committed obligations carried forward!
        assert_eq!(new_rev.revision_number, 2);
        assert_eq!(new_rev.nodes.len(), 3);
        assert!(new_rev.nodes.iter().any(|n| n.node_id == "node_read_repo"));
        assert!(new_rev
            .nodes
            .iter()
            .any(|n| n.node_id == "node_modify_code_v2"));
        assert!(new_rev
            .nodes
            .iter()
            .any(|n| n.node_id == "node_run_tests_v2"));
        assert_eq!(new_rev.obligations, rev.obligations);
    }

    #[tokio::test]
    async fn test_g12_replan_inv03_blocks_blind_retry_on_uncertain_outbox_effect() {
        let mut rev = WorkflowRevision::new("task-inv03-g12", "prop-1", 1);
        rev.nodes = vec![make_revision_node("node_cloud_api", "worker")];

        // An outbox entry was left in Uncertain state (e.g. gateway timeout after socket write)
        let outbox = vec![OutboxEntry::new(
            "out-g12-001",
            "task-inv03-g12",
            "node_cloud_api",
            "permit-api-write",
            "digest-abc",
            None,
            OutboxStatus::Uncertain,
            chrono::Utc::now(),
        )];

        let brief = Replanner::create_brief(
            "task-inv03-g12",
            &rev,
            ReplanTrigger::Timeout,
            Some("node_cloud_api"),
            "HTTP gateway timed out waiting for server response",
            &outbox,
        )
        .expect("create brief");

        // G12 Invariant: INV-03 triggers guardrail, forbidding automated blind retry
        assert!(brief.reason.contains("[INV-03 Guardrail]"));
        assert!(brief.reason.contains("Uncertain external effect detected"));
        assert!(brief.reason.contains("Blind retry forbidden"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 4: Hard Filters, Absent Executor Pruning & Explicit Uncertainty
// ─────────────────────────────────────────────────────────────────────────────
mod hard_filters_catalog_and_uncertainty_estimation {
    use super::*;

    #[test]
    fn test_g12_absent_executors_pruned_and_rejected_by_hard_filters() {
        let mut snapshot = DecisionSnapshot::new("task-catalog-g12");
        // Only custos-local and claude-code are configured/available in this workspace
        snapshot.available_executors = vec!["custos-local".into(), "claude-code".into()];

        // 1. CandidateBuilder must NOT suggest unconfigured executors (e.g. goose, custos-agent, claude-3-5-sonnet)
        let candidates = CandidateBuilder::build(&snapshot);
        assert!(!candidates.is_empty());
        for c in &candidates {
            assert!(
                snapshot.available_executors.contains(&c.harness_id),
                "Candidate harness {} was suggested but is not in available_executors",
                c.harness_id
            );
        }

        // 2. HardFilters strictly rejects any rogue proposal targeting an unconfigured executor
        let unconfigured_proposal =
            StrategyProposal::native_baseline("task-catalog-g12", "unconfigured-gpu-runner");
        let filter_res = HardFilters::evaluate(&unconfigured_proposal, &snapshot);
        assert!(filter_res.is_err());
        let reasons = filter_res.unwrap_err();
        assert!(reasons.iter().any(|r| match r {
            RejectionReason::CapabilityMissing(msg) =>
                msg.contains("unavailable or not configured"),
            _ => false,
        }));
    }

    #[test]
    fn test_g12_hard_filters_enforce_model_pin_and_local_only() {
        let mut snapshot = DecisionSnapshot::new("task-constraints-g12");
        snapshot.pinned_model = Some("claude-code".into());
        snapshot.required_capabilities = vec!["LocalOnly".into()];

        // Pinned model mismatch
        let mismatch_proposal =
            StrategyProposal::native_baseline("task-constraints-g12", "custos-local");
        let res1 = HardFilters::evaluate(&mismatch_proposal, &snapshot);
        assert!(res1.is_err());
        assert!(res1
            .unwrap_err()
            .contains(&RejectionReason::ModelPinMismatch));

        // Local-only violation
        let cloud_proposal =
            StrategyProposal::native_baseline("task-constraints-g12", "claude-code");
        let res2 = HardFilters::evaluate(&cloud_proposal, &snapshot);
        assert!(res2.is_err());
        assert!(res2
            .unwrap_err()
            .contains(&RejectionReason::LocalOnlyConstraint));
    }

    #[test]
    fn test_g12_estimator_explicit_uncertainty_representation() {
        let mut snapshot = DecisionSnapshot::new("task-uncertainty-g12");
        snapshot.unknowns = vec![
            "Remote Checkpoint Shard #2 Unverified".into(),
            "External S3 Upload Bandwidth Unknown".into(),
        ];
        snapshot.uncertain_effects_count = 1;
        snapshot.criteria = vec!["exit code 0".into(), "eval loss < 1.5".into()];

        let mut candidates = CandidateBuilder::build(&snapshot);
        Estimator::estimate_with_snapshot(&mut candidates, Some(&snapshot));

        for cand in &candidates {
            // G12 Invariant: Unknown spend and latency must NOT be 0!
            assert!(
                cand.est_tokens > 2000,
                "Tokens must reflect unknowns buffer"
            );
            assert!(
                cand.est_latency_ms > 4000,
                "Latency must reflect recovery and verification"
            );
            if !cand.harness_id.contains("local") {
                assert!(cand.est_cost_usd_max > cand.est_cost_usd_min);
            }
        }
    }
}
