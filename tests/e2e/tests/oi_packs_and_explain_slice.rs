//! Orchestration Intelligence Packs, Explain API, and Evaluation Slice (OI-7)
//!
//! Organized by thematic test modules:
//! - `domain_pack_specifications`: Profiles, capabilities, verifiers, and task fixtures.
//! - `explain_service_engine`: Core strategy explanation, admissibility, and DAG preview.
//! - `local_api_rpc_transport`: Daemon local API dispatch and strongly-typed client integration.
//! - `comparative_benchmarks`: OI vs Baseline benchmark suite across canonical pack fixtures.

use custos_bridge::BridgeService;
use custos_core::TaskService;
use custos_daemon::custos_local_api::{ExplainPlanRequest, LocalApiClient};
use custos_daemon::LocalApiDispatcher;
use custos_domain::oi::{DecisionSnapshot, ExecutionTopology};
use custos_persistence::SqliteTaskStore;
use custos_runtime::oi::ExplainService;
use custos_runtime::session::SessionManager;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Theme 1: Domain Pack Profiles & Fixtures (RFC 003 §4)
// ─────────────────────────────────────────────────────────────────────────────
mod domain_pack_specifications {
    use super::*;
    use custos_packs::assistant::fixtures::{
        assistant_agenda_fixture, assistant_draft_reply_fixture,
    };
    use custos_packs::assistant::profile::assistant_profile;
    use custos_packs::engineering::fixtures::{
        engineering_refactor_fixture, engineering_repair_fixture,
    };
    use custos_packs::engineering::profile::engineering_profile;
    use custos_packs::research::fixtures::{research_survey_fixture, research_synthesis_fixture};
    use custos_packs::research::profile::research_profile;

    #[test]
    fn test_engineering_pack_invariants() {
        let profile = engineering_profile();
        assert_eq!(profile.pack_id, "engineering");
        assert_eq!(profile.default_topology, ExecutionTopology::T4Worktree);
        assert!(profile.supports_topology(ExecutionTopology::T4Worktree));
        assert!(profile.supports_topology(ExecutionTopology::T6RepairLoop));
        assert!(
            !profile.allow_blind_retry,
            "Engineering pack strictly prohibits blind retry"
        );

        let repair_fix = engineering_repair_fixture();
        assert_eq!(repair_fix.pack_id, "engineering");
        assert!(repair_fix
            .required_capabilities
            .contains(&"fs.write".into()));

        let refactor_fix = engineering_refactor_fixture();
        assert_eq!(refactor_fix.pack_id, "engineering");
        assert!(refactor_fix
            .required_capabilities
            .contains(&"ast.index".into()));
    }

    #[test]
    fn test_research_pack_invariants() {
        let profile = research_profile();
        assert_eq!(profile.pack_id, "research");
        assert_eq!(profile.default_topology, ExecutionTopology::T3ReadFanOut);
        assert!(profile.supports_topology(ExecutionTopology::T3ReadFanOut));
        assert!(
            profile.allow_blind_retry,
            "Idempotent read research tasks allow retry"
        );

        let synth_fix = research_synthesis_fixture();
        assert_eq!(synth_fix.pack_id, "research");
        assert!(synth_fix
            .required_capabilities
            .contains(&"web.search".into()));

        let survey_fix = research_survey_fixture();
        assert_eq!(survey_fix.pack_id, "research");
    }

    #[test]
    fn test_assistant_pack_invariants() {
        let profile = assistant_profile();
        assert_eq!(profile.pack_id, "assistant");
        assert_eq!(profile.default_topology, ExecutionTopology::T0Direct);
        assert!(profile.supports_topology(ExecutionTopology::T0Direct));

        let agenda_fix = assistant_agenda_fixture();
        assert_eq!(agenda_fix.pack_id, "assistant");
        assert!(agenda_fix
            .required_capabilities
            .contains(&"calendar.read".into()));

        let reply_fix = assistant_draft_reply_fixture();
        assert_eq!(reply_fix.pack_id, "assistant");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 2: Explain Service Engine (Core Strategy Transparency)
// ─────────────────────────────────────────────────────────────────────────────
mod explain_service_engine {
    use super::*;

    #[test]
    fn test_explain_service_candidate_breakdown_and_admissibility() {
        let mut snapshot = DecisionSnapshot::new("task-explain-direct");
        snapshot.remaining_budget_tokens = 50_000;

        let report = ExplainService::explain(&snapshot).expect("generate explain report");

        assert_eq!(report.task_id, "task-explain-direct");
        assert!(!report.candidates.is_empty(), "Must evaluate candidates");
        assert!(
            report.candidates.iter().any(|c| c.admissible),
            "At least one candidate admissible"
        );
        assert!(
            !report.reasoning.is_empty(),
            "Planner reasoning must be present"
        );
    }

    #[test]
    fn test_explain_service_cost_bounding_and_dag_preview() {
        let mut snapshot = DecisionSnapshot::new("task-cost-bounding");
        snapshot.remaining_budget_tokens = 20_000;

        let report = ExplainService::explain(&snapshot).expect("explain report");

        assert!(
            report.cost_range_tokens.0 <= report.cost_range_tokens.1,
            "Valid token cost range"
        );
        assert!(
            report.cost_range_usd.0 <= report.cost_range_usd.1,
            "Valid USD cost range"
        );
        assert!(
            !report.compiled_nodes_preview.is_empty(),
            "Compiled DAG nodes preview required"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 3: Local API RPC Protocol (v1.oi.explain via LocalApiClient)
// ─────────────────────────────────────────────────────────────────────────────
mod local_api_rpc_transport {
    use super::*;
    use custos_packs::engineering::fixtures::engineering_repair_fixture;

    struct LocalApiTestHarness {
        client: LocalApiClient,
    }

    impl LocalApiTestHarness {
        fn new() -> Self {
            let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory task store"));
            let task_service = Arc::new(TaskService::new(store));
            let session_manager = Arc::new(SessionManager::new());
            let bridge_service = Arc::new(BridgeService::new(
                session_manager.clone(),
                task_service.clone(),
            ));

            let dispatcher = LocalApiDispatcher::new(task_service, session_manager, bridge_service);

            Self {
                client: LocalApiClient::new(Box::new(dispatcher)),
            }
        }
    }

    #[tokio::test]
    async fn test_rpc_explain_plan_roundtrip() {
        let harness = LocalApiTestHarness::new();
        let fixture = engineering_repair_fixture();

        let req = ExplainPlanRequest {
            task_id: Some("task-rpc-explain-001".into()),
            title: "Repair Workspace".into(),
            contract: Some(fixture),
            estimated_files_count: Some(20),
            estimated_complexity: Some(5),
            budget_limit_tokens: Some(30_000),
        };

        let report = harness
            .client
            .explain_plan("req-explain-01", req)
            .await
            .expect("local api explain RPC");

        assert_eq!(report.task_id, "task-rpc-explain-001");
        assert!(!report.candidates.is_empty());
        assert!(!report.compiled_nodes_preview.is_empty());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 4: Comparative Benchmarks (OI vs Baseline Evaluation Suite)
// ─────────────────────────────────────────────────────────────────────────────
mod comparative_benchmarks {
    use super::*;
    use custos_packs::benchmark::OiBenchmarkRunner;

    #[test]
    fn test_benchmark_suite_across_all_domain_packs() {
        let suite_report = OiBenchmarkRunner::run_suite();

        assert_eq!(
            suite_report.total_packs_evaluated, 3,
            "3 domain packs evaluated"
        );
        assert!(
            suite_report.all_benchmarks_passed,
            "All comparative benchmarks must pass"
        );

        for comparison in &suite_report.comparisons {
            assert_eq!(
                comparison.baseline_topology,
                ExecutionTopology::NativeBaseline
            );
            assert!(
                !comparison.assurance_guarantee.is_empty(),
                "Assurance guarantee must be declared"
            );
            assert!(comparison.oi_tokens > 0, "Token count must be positive");
            assert!(
                comparison.oi_parallelism >= 1,
                "Parallelism must be at least 1"
            );
        }
    }
}
