//! Orchestration Intelligence D0 Decision Slice (RFC 003 §2)
//!
//! Organized by thematic test modules:
//! - `reality_snapshot_extraction`: Extracting DecisionSnapshot from task state, outbox, and budget limits.
//! - `d0_planner_baseline_strategy`: D0 rule-based baseline planning with sub-50ms latency overhead.
//! - `durable_decision_ledger`: Appending and querying DecisionRecords from SQLite ledger.

use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_domain::task::{ContractEvidence, EvidenceKind, TaskContract};
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;

fn make_engineering_contract() -> TaskContract {
    TaskContract {
        pack_id: "engineering".into(),
        name: "refactor_auth".into(),
        description: "Refactor authority engine to use RFC 003 decisions".into(),
        required_capabilities: vec!["shell".into(), "file_write".into()],
        evidence_requirements: vec![
            ContractEvidence {
                kind: EvidenceKind::FileAnchor,
                required: true,
            },
            ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            },
        ],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 1: Decision Snapshot Extraction (RFC 003 §2A)
// ─────────────────────────────────────────────────────────────────────────────
mod reality_snapshot_extraction {
    use super::*;
    use custos_core::decision::DecisionSnapshotExtractor;
    use custos_domain::TaskStatus;

    #[tokio::test]
    async fn test_extract_snapshot_from_contract_and_budget() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
        let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

        let task = kernel
            .create_task_with_contract("OI Snapshot Task".into(), make_engineering_contract())
            .await
            .expect("create task");

        assert_eq!(task.status, TaskStatus::Draft);

        let snapshot =
            DecisionSnapshotExtractor::extract_snapshot(&task, Some(store.outbox()), 75_000, 7.50)
                .await
                .expect("extract snapshot");

        assert_eq!(snapshot.task_id, task.id);
        assert_eq!(snapshot.remaining_budget_tokens, 75_000);
        assert_eq!(snapshot.remaining_budget_usd, 7.50);
        assert_eq!(snapshot.pending_effects_count, 0);
        assert_eq!(snapshot.uncertain_effects_count, 0);
        assert_eq!(snapshot.criteria.len(), 2);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 2: D0 Planner Baseline Strategy (RFC 003 §2B, §3)
// ─────────────────────────────────────────────────────────────────────────────
mod d0_planner_baseline_strategy {
    use custos_domain::oi::{DecisionSnapshot, ExecutionTopology};
    use custos_runtime::cognitive::dumb_oi::DumbOiPlanner;

    #[test]
    fn test_d0_planner_proposes_baseline_with_minimal_overhead() {
        let snapshot = DecisionSnapshot::new("task-d0-test");
        let planner = DumbOiPlanner::new("claude-code");

        let record = planner.plan(snapshot).expect("plan must succeed");

        assert_eq!(record.task_id, "task-d0-test");
        assert_eq!(
            record.proposal.chosen_topology,
            ExecutionTopology::NativeBaseline
        );
        assert_eq!(record.proposal.candidate_harness, "claude-code");
        assert!(record.proposal.alternatives_considered.len() >= 2);
        assert!(
            record.overhead_ms < 50,
            "OI D0 decision overhead must be sub-50ms (got {}ms)",
            record.overhead_ms
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 3: Durable Decision Ledger (RFC 003 §2D)
// ─────────────────────────────────────────────────────────────────────────────
mod durable_decision_ledger {
    use super::*;
    use custos_core::contracts::storage::DecisionPort;
    use custos_domain::oi::{
        DecisionRecord, DecisionSnapshot, ExecutionTopology, StrategyProposal,
    };

    #[tokio::test]
    async fn test_decision_record_ledger_persistence_and_auditability() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
        let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

        let task = kernel
            .create_task_with_contract("Audit Task".into(), make_engineering_contract())
            .await
            .expect("create task");

        let snapshot = DecisionSnapshot::new(&task.id);
        let proposal = StrategyProposal::native_baseline(&task.id, "claude-code");
        let record = DecisionRecord::new(snapshot, proposal, 12);

        store
            .record_decision(&record)
            .await
            .expect("record decision in sqlite");

        let loaded = store
            .get_decision(&record.id)
            .await
            .expect("query decision")
            .expect("decision record must exist");

        assert_eq!(loaded.id, record.id);
        assert_eq!(loaded.task_id, task.id);
        assert_eq!(
            loaded.proposal.chosen_topology,
            ExecutionTopology::NativeBaseline
        );

        let all_decisions = store
            .list_decisions_for_task(&task.id)
            .await
            .expect("list decisions for task");
        assert_eq!(all_decisions.len(), 1);
        assert_eq!(all_decisions[0].id, record.id);
    }
}
