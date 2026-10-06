//! Graph Execution Engine (PR 4 - Gate 5)
//!
//! Orchestrates the execution of a `DagGraph` by dispatching ready nodes in parallel waves,
//! cascading failure/skip handling, and generating execution reports.

use async_trait::async_trait;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::workflow::dag::{DagGraph, NodeStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExecutionReport {
    pub graph_id: String,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub failed_nodes: usize,
    pub skipped_nodes: usize,
    pub success: bool,
}

#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn execute_step(
        &self,
        node_id: &str,
        action_type: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError>;
}

pub struct GraphRuntime {
    graph: Arc<RwLock<DagGraph>>,
    executor: Arc<dyn StepExecutor>,
}

impl GraphRuntime {
    pub fn new(graph: DagGraph, executor: Arc<dyn StepExecutor>) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
            executor,
        }
    }

    pub fn graph(&self) -> &Arc<RwLock<DagGraph>> {
        &self.graph
    }

    /// Executes the entire DAG to completion wave by wave, executing independent nodes in parallel.
    pub async fn execute_all(&self) -> Result<GraphExecutionReport, DomainError> {
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
                        nodes_to_run.push((
                            node.id.clone(),
                            node.action_type.clone(),
                            node.inputs.clone(),
                        ));
                    }
                }
            }

            // Execute this wave concurrently
            let mut join_set = tokio::task::JoinSet::new();
            for (node_id, action_type, inputs) in nodes_to_run {
                let executor = self.executor.clone();
                join_set.spawn(async move {
                    let result = executor.execute_step(&node_id, &action_type, &inputs).await;
                    (node_id, result)
                });
            }

            // Collect results and update DAG state
            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok((node_id, Ok(output))) => {
                        let mut graph = self.graph.write().await;
                        graph.mark_completed(&node_id, Some(output))?;
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

        for node in graph.nodes().values() {
            match node.status {
                NodeStatus::Completed => completed += 1,
                NodeStatus::Failed => failed += 1,
                NodeStatus::Skipped => skipped += 1,
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
            success,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::dag::DagNode;

    struct MockStepExecutor;

    #[async_trait]
    impl StepExecutor for MockStepExecutor {
        async fn execute_step(
            &self,
            node_id: &str,
            _action_type: &str,
            _inputs: &serde_json::Value,
        ) -> Result<serde_json::Value, DomainError> {
            if node_id == "fail_step" {
                return Err(DomainError::Validation("Simulated step failure".into()));
            }
            Ok(serde_json::json!({
                "status": "success",
                "node": node_id
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
