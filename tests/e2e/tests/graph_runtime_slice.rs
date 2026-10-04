//! Graph Runtime & Scheduler Vertical Slice (PR 4 - Gate 5 Verification)
//!
//! Reorganized into clear thematic test modules:
//! 1. `topological_waves_execution`: Topological wave compilation and multi-step parallel execution order.
//! 2. `cycle_detection_and_invariants`: Gate 5 cycle detection and cyclic dependency rejection.
//! 3. `periodic_workflow_scheduler`: Interval recurrence scheduling and due job execution triggers.

use std::sync::Arc;
use async_trait::async_trait;
use custos_domain::{DomainError, WorkflowPlan, WorkflowStep};
use custos_runtime::workflow::dag::{DagGraph, DagNode};
use custos_runtime::workflow::graph_runtime::{GraphRuntime, StepExecutor};
use custos_runtime::workflow::scheduler::{JobRecurrence, WorkflowScheduler};

struct RecordingStepExecutor {
    executed_order: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl RecordingStepExecutor {
    fn new() -> (Self, Arc<tokio::sync::Mutex<Vec<String>>>) {
        let order = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        (
            Self {
                executed_order: order.clone(),
            },
            order,
        )
    }
}

#[async_trait]
impl StepExecutor for RecordingStepExecutor {
    async fn execute_step(
        &self,
        node_id: &str,
        _action_type: &str,
        _inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        if node_id == "failing_step" {
            return Err(DomainError::Validation("Explicit node failure in test".into()));
        }
        self.executed_order.lock().await.push(node_id.to_string());
        Ok(serde_json::json!({
            "status": "ok",
            "executed_node": node_id
        }))
    }
}

// =========================================================================
// THEME 1: Multi-Step Topological Waves Parallel Execution
// =========================================================================
mod topological_waves_execution {
    use super::*;

    #[tokio::test]
    async fn test_gate_5_graph_runtime_full_lifecycle() {
        // Build a WorkflowPlan:
        //         Init (Wave 0)
        //        /    \
        //   Build      Lint   (Wave 1 - parallel)
        //        \    /
        //         Test        (Wave 2 - joins Build & Lint)
        //          |
        //        Deploy       (Wave 3)
        let plan = WorkflowPlan::new(
            "task_pipeline_001",
            "ci_cd",
            "Full CI/CD Pipeline",
            vec![
                WorkflowStep::new("Init", "Initialize workspace", "setup", serde_json::json!({})),
                WorkflowStep::new("Build", "Compile code", "build", serde_json::json!({})),
                WorkflowStep::new("Lint", "Run linter", "lint", serde_json::json!({})),
                WorkflowStep::new("Test", "Run test suite", "test", serde_json::json!({})),
                WorkflowStep::new("Deploy", "Deploy to staging", "deploy", serde_json::json!({})),
            ],
        );

        let mut graph = DagGraph::new("CI/CD Pipeline");
        let init_id = plan.steps[0].id.clone();
        let build_id = plan.steps[1].id.clone();
        let lint_id = plan.steps[2].id.clone();
        let test_id = plan.steps[3].id.clone();
        let deploy_id = plan.steps[4].id.clone();

        graph
            .add_node(DagNode::new(&init_id, "Init", "setup", serde_json::json!({}), vec![]))
            .unwrap();
        graph
            .add_node(DagNode::new(&build_id, "Build", "build", serde_json::json!({}), vec![init_id.clone()]))
            .unwrap();
        graph
            .add_node(DagNode::new(&lint_id, "Lint", "lint", serde_json::json!({}), vec![init_id.clone()]))
            .unwrap();
        graph
            .add_node(DagNode::new(&test_id, "Test", "test", serde_json::json!({}), vec![build_id.clone(), lint_id.clone()]))
            .unwrap();
        graph
            .add_node(DagNode::new(&deploy_id, "Deploy", "deploy", serde_json::json!({}), vec![test_id.clone()]))
            .unwrap();

        // Validate Gate 5
        assert!(graph.validate_dag().is_ok());

        let waves = graph.compute_execution_waves().unwrap();
        assert_eq!(waves.len(), 4);
        assert_eq!(waves[0].node_ids, vec![init_id.clone()]);
        assert_eq!(waves[1].node_ids, {
            let mut sorted = vec![build_id.clone(), lint_id.clone()];
            sorted.sort();
            sorted
        });
        assert_eq!(waves[2].node_ids, vec![test_id.clone()]);
        assert_eq!(waves[3].node_ids, vec![deploy_id.clone()]);

        // Execute via GraphRuntime
        let (executor, executed_order) = RecordingStepExecutor::new();
        let runtime = GraphRuntime::new(graph, Arc::new(executor));

        let report = runtime.execute_all().await.expect("execution must succeed");
        assert!(report.success);
        assert_eq!(report.total_nodes, 5);
        assert_eq!(report.completed_nodes, 5);
        assert_eq!(report.failed_nodes, 0);
        assert_eq!(report.skipped_nodes, 0);

        let order = executed_order.lock().await.clone();
        assert_eq!(order.len(), 5);
        assert_eq!(order[0], init_id);
        assert_eq!(order[3], test_id);
        assert_eq!(order[4], deploy_id);
    }
}

// =========================================================================
// THEME 2: Gate 5 Cycle Detection & Invariants
// =========================================================================
mod cycle_detection_and_invariants {
    use super::*;

    #[tokio::test]
    async fn test_gate_5_cycle_rejection_in_graph_runtime() {
        let mut graph = DagGraph::new("Cyclic Graph");
        graph
            .add_node(DagNode::new("A", "Node A", "exec", serde_json::json!({}), vec!["B".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("B", "Node B", "exec", serde_json::json!({}), vec!["A".into()]))
            .unwrap();

        let (executor, _) = RecordingStepExecutor::new();
        let runtime = GraphRuntime::new(graph, Arc::new(executor));

        let result = runtime.execute_all().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("Cycle detected"));
    }
}

// =========================================================================
// THEME 3: Periodic Workflow Scheduler & Due Job Selection
// =========================================================================
mod periodic_workflow_scheduler {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_lifecycle() {
        let scheduler = WorkflowScheduler::new();

        let job_id = scheduler
            .schedule(
                "Hourly Security Sweep",
                JobRecurrence::Interval { seconds: 3600 },
                chrono::Utc::now(),
                serde_json::json!({"scope": "workspace"}),
            )
            .await;

        let job = scheduler.get_job(&job_id).await.expect("job exists");
        assert_eq!(job.name, "Hourly Security Sweep");
        assert_eq!(job.recurrence, JobRecurrence::Interval { seconds: 3600 });

        let due_jobs = scheduler.get_due_jobs(chrono::Utc::now() + chrono::Duration::seconds(10)).await;
        assert_eq!(due_jobs.len(), 1);
        assert_eq!(due_jobs[0].id, job_id);
    }
}
