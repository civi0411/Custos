//! Orchestration Intelligence Persistence & Crash Resilience Slice (OI-4)
//!
//! Reorganized into clear thematic test modules:
//! 1. `workflow_revision_crash_resilience`: Durable revision & placement persistence, crash reboot, idempotent resave, DAG resumption.
//! 2. `replan_record_crash_resilience`: ReplanBrief persistence and recovery across multiple crash/reboot cycles.
//! 3. `decision_ledger_crash_resilience`: Audit trail recovery of DecisionRecord across store reboots.

use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::oi::OiPlannerPort;
use custos_core::contracts::storage::{DecisionPort, ReplanPort, WorkflowRevisionPort};
use custos_core::decision::DecisionSnapshotExtractor;
use custos_core::oi::admissibility::{AdmissibilityEvaluator, AdmissibilityResult};
use custos_domain::{
    DecisionRecord, ExecutionTopology, NodePlacement, ReplanBrief, ReplanRecord, ReplanTrigger,
    RevisionNode, Task, TaskContract, WorkflowRevision,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::oi::engine::OiEngine;
use custos_runtime::workflow::graph_runtime::GraphRuntime;
use custos_runtime::workflow::revision_loader::RevisionLoader;
use custos_runtime::workflow::worker_executor::WorkerExecutor;
use std::sync::Arc;

async fn setup_task(store: &Arc<SqliteTaskStore>, title: &str) -> Task {
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));
    let contract = TaskContract {
        pack_id: "engineering".into(),
        name: "test_oi_persistence".into(),
        description: "Verify OI durable persistence and crash recovery".into(),
        required_capabilities: vec!["fs_read".into()],
        evidence_requirements: vec![],
    };
    kernel
        .create_task_with_contract(title.into(), contract)
        .await
        .expect("create task")
}

// =========================================================================
// THEME 1: Workflow Revision & Placement Crash Resilience & DAG Resumption
// =========================================================================
mod workflow_revision_crash_resilience {
    use super::*;

    #[tokio::test]
    async fn test_workflow_revision_and_placements_crash_recovery_and_resumed_execution() {
        let temp_db = tempfile::NamedTempFile::new().expect("create temp sqlite file");
        let db_path = temp_db.path().to_str().expect("valid db path").to_string();
        let (revision_id, task_id, proposal_id);

        // Phase 1: Planning, Compilation, and Durable Persistence
        {
            let store = Arc::new(SqliteTaskStore::new(&db_path).expect("open file-backed store"));
            let task = setup_task(&store, "OI Crash Task").await;
            task_id = task.id.clone();

            let snapshot = DecisionSnapshotExtractor::extract_snapshot(
                &task,
                Some(store.outbox()),
                50_000,
                5.0,
            )
            .await
            .expect("extract snapshot");
            let proposal = OiEngine::new()
                .plan(&snapshot)
                .await
                .expect("OiEngine planning failed");
            proposal_id = proposal.id.clone();
            assert_eq!(proposal.chosen_topology, ExecutionTopology::NativeBaseline);

            let admitted = match AdmissibilityEvaluator::evaluate(proposal.clone(), &snapshot) {
                AdmissibilityResult::Admitted(p) => p,
                AdmissibilityResult::Rejected(reasons) => {
                    panic!("Proposal rejected: {:?}", reasons)
                }
            };

            let mut revision = WorkflowRevision::new(task.id.clone(), admitted.id.clone(), 1);
            revision.nodes.push(RevisionNode {
                node_id: "node_prep".into(),
                step_name: "Prepare Context".into(),
                role: "assistant".into(),
                harness_id: admitted.candidate_harness.clone(),
                allocated_budget_tokens: 2000,
                read_set: vec![],
                write_set: vec![],
                required_capabilities: vec![],
            });
            revision.nodes.push(RevisionNode {
                node_id: "node_audit".into(),
                step_name: "Audit Source".into(),
                role: "auditor".into(),
                harness_id: admitted.candidate_harness.clone(),
                allocated_budget_tokens: 4000,
                read_set: vec!["src/lib.rs".into()],
                write_set: vec![],
                required_capabilities: vec!["fs_read".into()],
            });
            revision
                .dependencies
                .push(("node_prep".into(), "node_audit".into()));
            revision.obligations.push("audit_log_verified".into());
            revision_id = revision.revision_id.clone();

            let placements = vec![
                NodePlacement {
                    node_id: "node_prep".into(),
                    role: "assistant".into(),
                    backend_harness: admitted.candidate_harness.clone(),
                    budget_tokens_slice: 2000,
                    workspace_lease_id: None,
                },
                NodePlacement {
                    node_id: "node_audit".into(),
                    role: "auditor".into(),
                    backend_harness: admitted.candidate_harness.clone(),
                    budget_tokens_slice: 4000,
                    workspace_lease_id: Some("lease_resilience_01".into()),
                },
            ];

            store
                .save_revision(&revision, &placements)
                .await
                .expect("save revision and placements");
            // Store is dropped here, simulating daemon crash / SIGKILL
        }

        // Phase 2: Reboot & State Verification
        {
            let store2 =
                Arc::new(SqliteTaskStore::new(&db_path).expect("reopen store after crash"));

            let recovered_rev = store2
                .get_revision(&revision_id)
                .await
                .expect("query rev")
                .expect("must survive");
            assert_eq!(recovered_rev.revision_id, revision_id);
            assert_eq!(recovered_rev.task_id, task_id);
            assert_eq!(recovered_rev.proposal_id, proposal_id);
            assert_eq!(recovered_rev.nodes.len(), 2);
            assert_eq!(recovered_rev.dependencies.len(), 1);
            assert_eq!(recovered_rev.obligations[0], "audit_log_verified");

            let recovered_placements = store2
                .get_placements_for_revision(&revision_id)
                .await
                .expect("query placements");
            assert_eq!(recovered_placements.len(), 2);
            assert_eq!(recovered_placements[0].node_id, "node_audit");
            assert_eq!(
                recovered_placements[0].workspace_lease_id.as_deref(),
                Some("lease_resilience_01")
            );

            // Verify Idempotent Resave
            store2
                .save_revision(&recovered_rev, &recovered_placements)
                .await
                .expect("idempotent resave succeeds");

            // Resume execution from recovered revision in DAG
            let dag = RevisionLoader::load_into_dag(&recovered_rev)
                .expect("compile recovered revision into DAG");
            assert_eq!(dag.nodes().len(), 2);

            let executor = Arc::new(WorkerExecutor::new());
            let report = GraphRuntime::new(dag, executor)
                .execute_all()
                .await
                .expect("execute recovered workflow");
            assert!(report.success, "Resumed workflow execution must succeed");
            assert_eq!(report.completed_nodes, 2);
        }
    }
}

// =========================================================================
// THEME 2: Replan Record Persistence & Multi-Crash Durability
// =========================================================================
mod replan_record_crash_resilience {
    use super::*;

    #[tokio::test]
    async fn test_replan_record_persistence_across_multiple_crashes() {
        let temp_db = tempfile::NamedTempFile::new().expect("create temp sqlite file");
        let db_path = temp_db.path().to_str().expect("valid db path").to_string();
        let (task_id, replan_id);

        // Phase 1: Create task and persist ReplanRecord
        {
            let store = Arc::new(SqliteTaskStore::new(&db_path).expect("open store"));
            let task = setup_task(&store, "Replan Durability Task").await;
            task_id = task.id.clone();

            let brief = ReplanBrief::new(
                &task_id,
                ReplanTrigger::TestFailure,
                Some("node_audit".into()),
                "Security vulnerability flagged",
            );
            let replan = ReplanRecord::new(&task_id, brief, "prop_replan_followup");
            replan_id = replan.id.clone();

            store.record_replan(&replan).await.expect("record replan");
            // Store dropped (Crash 1)
        }

        // Phase 2: Reboot 1 and verify recovery
        {
            let store2 = Arc::new(SqliteTaskStore::new(&db_path).expect("reopen store 1"));
            let replans = store2
                .list_replans_for_task(&task_id)
                .await
                .expect("list replans");
            assert_eq!(replans.len(), 1);
            assert_eq!(replans[0].id, replan_id);
            assert_eq!(replans[0].brief.trigger, ReplanTrigger::TestFailure);
            assert_eq!(
                replans[0].brief.failed_node_id.as_deref(),
                Some("node_audit")
            );
            assert_eq!(replans[0].new_proposal_id, "prop_replan_followup");
            // Store dropped (Crash 2)
        }

        // Phase 3: Reboot 2 and assert persistent integrity
        {
            let store3 = Arc::new(SqliteTaskStore::new(&db_path).expect("reopen store 2"));
            let replans = store3
                .list_replans_for_task(&task_id)
                .await
                .expect("list replans 2");
            assert_eq!(replans.len(), 1);
            assert_eq!(replans[0].brief.reason, "Security vulnerability flagged");
            assert_eq!(replans[0].new_proposal_id, "prop_replan_followup");
        }
    }
}

// =========================================================================
// THEME 3: Decision Ledger Durability Across Reboots
// =========================================================================
mod decision_ledger_crash_resilience {
    use super::*;

    #[tokio::test]
    async fn test_decision_record_durability_across_reboots() {
        let temp_db = tempfile::NamedTempFile::new().expect("create temp sqlite file");
        let db_path = temp_db.path().to_str().expect("valid db path").to_string();
        let (task_id, proposal_id);

        // Phase 1: Record decision and crash
        {
            let store = Arc::new(SqliteTaskStore::new(&db_path).expect("open store"));
            let task = setup_task(&store, "Decision Audit Task").await;
            task_id = task.id.clone();

            let snapshot = DecisionSnapshotExtractor::extract_snapshot(
                &task,
                Some(store.outbox()),
                30_000,
                3.0,
            )
            .await
            .expect("snapshot");
            let proposal = OiEngine::new().plan(&snapshot).await.expect("plan");
            proposal_id = proposal.id.clone();

            let record = DecisionRecord::new(snapshot, proposal, 15);
            store
                .record_decision(&record)
                .await
                .expect("record decision");
            // Store dropped (Crash)
        }

        // Phase 2: Reboot and assert audit trail recovery
        {
            let store2 = Arc::new(SqliteTaskStore::new(&db_path).expect("reopen store"));
            let decisions = store2
                .list_decisions_for_task(&task_id)
                .await
                .expect("list decisions");
            assert_eq!(decisions.len(), 1);
            assert_eq!(decisions[0].task_id, task_id);
            assert_eq!(decisions[0].proposal.id, proposal_id);
            assert_eq!(decisions[0].overhead_ms, 15);
        }
    }
}
