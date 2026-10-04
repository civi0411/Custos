//! Orchestration Intelligence Engine Slice (OI-3)
//!
//! Organized by thematic test modules:
//! - `engine_planning_overhead`: Validates sub-100ms planning latency constraint.
//! - `admissibility_and_compilation`: Validates admissibility gate and DAG loading.
//! - `graph_runtime_execution`: Validates full execution through GraphRuntime & WorkerExecutor.

use std::sync::Arc;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::decision::DecisionSnapshotExtractor;
use custos_domain::task::TaskContract;
use custos_domain::oi::DecisionSnapshot;
use custos_persistence::SqliteTaskStore;

async fn setup_task_and_snapshot(task_name: &str) -> (DecisionSnapshot, String) {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    let contract = TaskContract {
        pack_id: "engineering".into(),
        name: task_name.into(),
        description: "Test the OI engine flow".into(),
        required_capabilities: vec![],
        evidence_requirements: vec![],
    };

    let task = kernel
        .create_task_with_contract(task_name.into(), contract)
        .await
        .expect("create task");

    let snapshot = DecisionSnapshotExtractor::extract_snapshot(
        &task,
        Some(store.outbox()),
        75_000,
        7.50,
    )
    .await
    .expect("extract snapshot");

    (snapshot, task.id)
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 1: Engine Planning Overhead (< 100ms)
// ─────────────────────────────────────────────────────────────────────────────
mod engine_planning_overhead {
    use super::*;
    use std::time::Instant;
    use custos_core::contracts::oi::OiPlannerPort;
    use custos_domain::oi::ExecutionTopology;
    use custos_runtime::oi::engine::OiEngine;

    #[tokio::test]
    async fn test_oi_engine_planning_latency_and_topology() {
        let (snapshot, task_id) = setup_task_and_snapshot("Latency Test Task").await;

        let start_plan = Instant::now();
        let engine = OiEngine::new();
        let proposal = engine.plan(&snapshot).await.expect("OiEngine planning failed");
        let planning_time_ms = start_plan.elapsed().as_millis();

        assert!(
            planning_time_ms < 100,
            "Planning overhead must be < 100ms (got {}ms)",
            planning_time_ms
        );
        assert_eq!(proposal.task_id, task_id);
        assert_eq!(proposal.chosen_topology, ExecutionTopology::NativeBaseline);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 2: Admissibility Gate & Compilation Flow
// ─────────────────────────────────────────────────────────────────────────────
mod admissibility_and_compilation {
    use super::*;
    use custos_core::contracts::oi::OiPlannerPort;
    use custos_core::oi::admissibility::{AdmissibilityEvaluator, AdmissibilityResult};
    use custos_domain::workflow::{RevisionNode, WorkflowRevision};
    use custos_runtime::oi::engine::OiEngine;
    use custos_runtime::workflow::revision_loader::RevisionLoader;

    #[tokio::test]
    async fn test_admissibility_and_dag_compilation() {
        let (snapshot, task_id) = setup_task_and_snapshot("Compile Test Task").await;
        let engine = OiEngine::new();
        let proposal = engine.plan(&snapshot).await.expect("OiEngine planning failed");

        let admit_result = AdmissibilityEvaluator::evaluate(proposal.clone(), &snapshot);
        let admitted_proposal = match admit_result {
            AdmissibilityResult::Admitted(p) => p,
            AdmissibilityResult::Rejected(reasons) => panic!("Proposal rejected: {:?}", reasons),
        };

        let mut revision = WorkflowRevision::new(task_id, admitted_proposal.id.clone(), 1);
        revision.nodes.push(RevisionNode {
            node_id: "worker_1".into(),
            step_name: "Execute Baseline".into(),
            role: "assistant".into(),
            harness_id: admitted_proposal.candidate_harness.clone(),
            allocated_budget_tokens: admitted_proposal.estimated_tokens,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        });

        let dag = RevisionLoader::load_into_dag(&revision).expect("load revision into DAG");
        assert_eq!(dag.nodes().len(), 1);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme 3: GraphRuntime Execution
// ─────────────────────────────────────────────────────────────────────────────
mod graph_runtime_execution {
    use super::*;
    use custos_core::contracts::oi::OiPlannerPort;
    use custos_domain::workflow::{RevisionNode, WorkflowRevision};
    use custos_runtime::oi::engine::OiEngine;
    use custos_runtime::workflow::graph_runtime::GraphRuntime;
    use custos_runtime::workflow::revision_loader::RevisionLoader;
    use custos_runtime::workflow::worker_executor::WorkerExecutor;

    #[tokio::test]
    async fn test_full_graph_runtime_execution_slice() {
        let (snapshot, task_id) = setup_task_and_snapshot("Execute Test Task").await;
        let engine = OiEngine::new();
        let proposal = engine.plan(&snapshot).await.expect("OiEngine planning failed");

        let mut revision = WorkflowRevision::new(task_id, proposal.id.clone(), 1);
        revision.nodes.push(RevisionNode {
            node_id: "worker_1".into(),
            step_name: "Execute Baseline Step".into(),
            role: "assistant".into(),
            harness_id: proposal.candidate_harness.clone(),
            allocated_budget_tokens: proposal.estimated_tokens,
            read_set: vec![],
            write_set: vec![],
            required_capabilities: vec![],
        });

        let dag = RevisionLoader::load_into_dag(&revision).expect("load into DAG");
        let executor = Arc::new(WorkerExecutor::new());
        let graph_runtime = GraphRuntime::new(dag, executor);

        let report = graph_runtime.execute_all().await.expect("graph execution failed");
        assert!(report.success, "Graph execution must succeed");
        assert_eq!(report.completed_nodes, 1);
    }
}
