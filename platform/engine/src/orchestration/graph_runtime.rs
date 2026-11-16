//! Graph Execution Engine (PR 4 - Gate 5)
//!
//! Orchestrates the execution of a `DagGraph` by dispatching ready nodes in parallel waves,
//! cascading failure/skip handling, and generating execution reports.

use async_trait::async_trait;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::contracts::{ExecutionContext, StepObservation};
use crate::workflow::dag::{DagGraph, NodeStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExecutionReport {
    pub graph_id: String,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub failed_nodes: usize,
    pub skipped_nodes: usize,
    #[serde(default)]
    pub waiting_nodes: usize,
    #[serde(default)]
    pub uncertain_nodes: usize,
    pub success: bool,
}

#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute_step(
        &self,
        ctx: &super::contracts::ExecutionContext,
        action_type: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError>;

    async fn execute_step_observation(
        &self,
        ctx: &ExecutionContext,
        action_type: &str,
        inputs: &serde_json::Value,
    ) -> Result<StepObservation, DomainError> {
        StepObservation::from_output(
            &ctx.node_id,
            self.execute_step(ctx, action_type, inputs).await?,
        )
    }
}

pub struct GraphRuntime {
    graph: Arc<RwLock<DagGraph>>,
    executor: Arc<dyn StepExecutor>,
    execution_lock: Mutex<()>,
}

impl GraphRuntime {
    pub fn new(graph: DagGraph, executor: Arc<dyn StepExecutor>) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
            executor,
            execution_lock: Mutex::new(()),
        }
    }

    pub fn graph(&self) -> &Arc<RwLock<DagGraph>> {
        &self.graph
    }

    /// Executes the entire DAG to completion wave by wave, executing independent nodes in parallel.
    pub async fn execute_all(&self) -> Result<GraphExecutionReport, DomainError> {
        let _execution = self.execution_lock.lock().await;
        // Gate 5: Strict validation before start
        {
            let graph = self.graph.read().await;
            graph.validate_dag()?;
        }

        loop {
            let ready_node_ids = {
                let graph = self.graph.read().await;
                if graph.is_finished() {
                    break;
                }
                graph.ready_nodes()
            };

            if ready_node_ids.is_empty() {
                let graph = self.graph.read().await;
                if graph.is_finished() {
                    break;
                } else if graph.nodes().values().any(|node| {
                    matches!(
                        node.status,
                        NodeStatus::WaitingApproval | NodeStatus::Uncertain
                    )
                }) {
                    // Only an explicit approval/reconciliation path may release a blocked node.
                    break;
                } else {
                    return Err(DomainError::InvariantViolation(
                        "DAG execution stalled: uncompleted nodes exist but none are ready to execute"
                            .into(),
                    ));
                }
            }

            // Mark ready nodes as Running and gather node data
            let mut nodes_to_run = Vec::new();
            {
                let mut graph = self.graph.write().await;
                for id in &ready_node_ids {
                    graph.mark_running(id)?;
                    if let Some(node) = graph.get_node(id) {
                        let ctx = super::contracts::ExecutionContext::new(
                            graph.task_id.clone().unwrap_or_default(),
                            graph.id.clone(),
                            node.id.clone(),
                            "worker_executor",
                        );
                        nodes_to_run.push((ctx, node.action_type.clone(), node.inputs.clone()));
                    }
                }
            }

            // Execute this wave concurrently
            let mut join_set = tokio::task::JoinSet::new();
            for (ctx, action_type, inputs) in nodes_to_run {
                let executor = self.executor.clone();
                join_set.spawn(async move {
                    let result = executor
                        .execute_step_observation(&ctx, &action_type, &inputs)
                        .await;
                    (ctx.node_id, result)
                });
            }

            // Collect results and update DAG state
            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok((node_id, Ok(observation))) => {
                        let mut graph = self.graph.write().await;
                        match observation {
                            StepObservation::ToolExecuted { receipt, .. } => {
                                let output = serde_json::to_value(receipt)
                                    .map_err(|error| DomainError::Validation(error.to_string()))?;
                                graph.mark_completed(&node_id, Some(output))?;
                            }
                            StepObservation::NodeComplete { output, .. } => {
                                graph.mark_completed(&node_id, Some(output))?;
                            }
                            StepObservation::ToolFailed { error, .. } => {
                                graph.mark_failed(&node_id, error)?;
                            }
                            StepObservation::ToolPending { attempt_id } => {
                                graph.mark_blocked(
                                    &node_id,
                                    NodeStatus::WaitingApproval,
                                    serde_json::json!({"attempt_id": attempt_id}),
                                )?;
                            }
                            StepObservation::ToolUncertain { attempt_id, error } => {
                                graph.mark_blocked(
                                    &node_id,
                                    NodeStatus::Uncertain,
                                    serde_json::json!({"attempt_id": attempt_id, "error": error}),
                                )?;
                            }
                        }
                    }
                    Ok((node_id, Err(e))) => {
                        let mut graph = self.graph.write().await;
                        graph.mark_failed(&node_id, e.to_string())?;
                    }
                    Err(join_err) => {
                        return Err(DomainError::InvariantViolation(format!(
                            "Join error during DAG step execution: {join_err}"
                        )));
                    }
                }
            }
        }

        // Generate report
        let graph = self.graph.read().await;
        let mut completed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut waiting = 0;
        let mut uncertain = 0;

        for node in graph.nodes().values() {
            match node.status {
                NodeStatus::Completed => completed += 1,
                NodeStatus::Failed => failed += 1,
                NodeStatus::Skipped => skipped += 1,
                NodeStatus::WaitingApproval => waiting += 1,
                NodeStatus::Uncertain => uncertain += 1,
                _ => {}
            }
        }

        let total = graph.nodes().len();
        let success = completed == total;

        Ok(GraphExecutionReport {
            graph_id: graph.id.clone(),
            total_nodes: total,
            completed_nodes: completed,
            failed_nodes: failed,
            skipped_nodes: skipped,
            waiting_nodes: waiting,
            uncertain_nodes: uncertain,
            success,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::dag::DagNode;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct OutputExecutor {
        output: serde_json::Value,
        calls: AtomicUsize,
    }

    #[async_trait]
    impl StepExecutor for OutputExecutor {
        async fn execute_step(
            &self,
            ctx: &ExecutionContext,
            _action_type: &str,
            _inputs: &serde_json::Value,
        ) -> Result<serde_json::Value, DomainError> {
            assert_eq!(ctx.task_id, "task-bound");
            assert_ne!(ctx.run_id, "forged-run");
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.output.clone())
        }
    }

    fn outcome_graph(output: serde_json::Value) -> (GraphRuntime, Arc<OutputExecutor>) {
        let mut graph = DagGraph::new("outcome test");
        graph.task_id = Some("task-bound".into());
        graph
            .add_node(DagNode::new(
                "effect",
                "Effect",
                "tool_call",
                serde_json::json!({"task_id": "forged-task", "run_id": "forged-run"}),
                vec![],
            ))
            .unwrap();
        graph
            .add_node(DagNode::new(
                "dependent",
                "Dependent",
                "work",
                serde_json::json!({}),
                vec!["effect".into()],
            ))
            .unwrap();
        let executor = Arc::new(OutputExecutor {
            output,
            calls: AtomicUsize::new(0),
        });
        (GraphRuntime::new(graph, executor.clone()), executor)
    }

    #[tokio::test]
    async fn pending_and_uncertain_effects_block_without_reexecution() {
        for (output, expected) in [
            (
                serde_json::json!({"status": "pending_approval", "attempt_id": "attempt-1"}),
                NodeStatus::WaitingApproval,
            ),
            (
                serde_json::json!({"status": "uncertain", "attempt_id": "attempt-1"}),
                NodeStatus::Uncertain,
            ),
        ] {
            let (runtime, executor) = outcome_graph(output);
            for _ in 0..2 {
                let report = runtime.execute_all().await.unwrap();
                assert!(!report.success);
                assert_eq!(report.completed_nodes, 0);
                assert_eq!(report.failed_nodes, 0);
                assert_eq!(report.waiting_nodes + report.uncertain_nodes, 1);
                let graph = runtime.graph().read().await;
                assert_eq!(graph.get_node("effect").unwrap().status, expected);
                assert_eq!(
                    graph.get_node("dependent").unwrap().status,
                    NodeStatus::Pending
                );
            }
            assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn all_receipt_statuses_have_truthful_graph_outcomes() {
        for (status, expected) in [
            ("success", NodeStatus::Completed),
            ("failure", NodeStatus::Failed),
            ("rejected", NodeStatus::Failed),
            ("timeout", NodeStatus::Uncertain),
        ] {
            let (runtime, _) = outcome_graph(serde_json::json!({
                "receipt_id": "receipt-1", "permit_id": "permit-1", "action_id": "action-1",
                "status": status, "output_digest": "digest", "executed_at": chrono::Utc::now(),
            }));
            let report = runtime.execute_all().await.unwrap();
            assert_eq!(report.success, expected == NodeStatus::Completed);
            let graph = runtime.graph().read().await;
            assert_eq!(graph.get_node("effect").unwrap().status, expected);
            assert_eq!(
                graph.get_node("dependent").unwrap().status,
                match expected {
                    NodeStatus::Completed => NodeStatus::Completed,
                    NodeStatus::Failed => NodeStatus::Skipped,
                    _ => NodeStatus::Pending,
                }
            );
        }
    }

    #[tokio::test]
    async fn malformed_receipts_and_missing_approval_identity_cannot_complete() {
        for output in [
            serde_json::json!({"receipt_id": "partial", "status": "success"}),
            serde_json::json!({"status": "pending_approval"}),
        ] {
            let (runtime, _) = outcome_graph(output);
            let report = runtime.execute_all().await.unwrap();
            assert_eq!(report.failed_nodes, 1);
            assert_eq!(report.skipped_nodes, 1);
            assert!(!report.success);
        }
    }

    #[tokio::test]
    async fn concurrent_execute_calls_do_not_dispatch_twice() {
        let (runtime, executor) = outcome_graph(serde_json::json!({"status": "success"}));
        let (first, second) = tokio::join!(runtime.execute_all(), runtime.execute_all());
        assert!(first.unwrap().success);
        assert!(second.unwrap().success);
        assert_eq!(executor.calls.load(Ordering::SeqCst), 2); // One call for each of two nodes.
    }

    struct MockStepExecutor;

    #[async_trait]
    impl StepExecutor for MockStepExecutor {
        async fn execute_step(
            &self,
            ctx: &crate::workflow::contracts::ExecutionContext,
            _action_type: &str,
            _inputs: &serde_json::Value,
        ) -> Result<serde_json::Value, DomainError> {
            if ctx.node_id == "fail_step" {
                return Err(DomainError::Validation("Simulated step failure".into()));
            }
            Ok(serde_json::json!({
                "status": "success",
                "node": ctx.node_id
            }))
        }
    }

    #[tokio::test]
    async fn test_graph_runtime_parallel_waves_execution() {
        let mut graph = DagGraph::new("Parallel Waves Test");
        graph
            .add_node(DagNode::new(
                "start",
                "Start",
                "init",
                serde_json::json!({}),
                vec![],
            ))
            .unwrap();
        graph
            .add_node(DagNode::new(
                "task1",
                "Task 1",
                "work",
                serde_json::json!({}),
                vec!["start".into()],
            ))
            .unwrap();
        graph
            .add_node(DagNode::new(
                "task2",
                "Task 2",
                "work",
                serde_json::json!({}),
                vec!["start".into()],
            ))
            .unwrap();
        graph
            .add_node(DagNode::new(
                "join",
                "Join",
                "merge",
                serde_json::json!({}),
                vec!["task1".into(), "task2".into()],
            ))
            .unwrap();

        let executor = Arc::new(MockStepExecutor);
        let runtime = GraphRuntime::new(graph, executor);

        let report = runtime.execute_all().await.unwrap();
        assert!(report.success);
        assert_eq!(report.total_nodes, 4);
        assert_eq!(report.completed_nodes, 4);
        assert_eq!(report.failed_nodes, 0);
        assert_eq!(report.skipped_nodes, 0);
    }

    #[tokio::test]
    async fn test_graph_runtime_failure_and_skip_cascade() {
        let mut graph = DagGraph::new("Failure Test");
        graph
            .add_node(DagNode::new(
                "root",
                "Root",
                "init",
                serde_json::json!({}),
                vec![],
            ))
            .unwrap();
        graph
            .add_node(DagNode::new(
                "fail_step",
                "Failing Step",
                "work",
                serde_json::json!({}),
                vec!["root".into()],
            ))
            .unwrap();
        graph
            .add_node(DagNode::new(
                "downstream",
                "Downstream",
                "merge",
                serde_json::json!({}),
                vec!["fail_step".into()],
            ))
            .unwrap();

        let executor = Arc::new(MockStepExecutor);
        let runtime = GraphRuntime::new(graph, executor);

        let report = runtime.execute_all().await.unwrap();
        assert!(!report.success);
        assert_eq!(report.total_nodes, 3);
        assert_eq!(report.completed_nodes, 1); // root
        assert_eq!(report.failed_nodes, 1); // fail_step
        assert_eq!(report.skipped_nodes, 1); // downstream
    }
}
