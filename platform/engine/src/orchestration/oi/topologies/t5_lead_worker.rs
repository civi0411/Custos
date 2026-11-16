//! T5 Lead-Worker DAG Coordinator (RFC 004 §2A, G12)
//!
//! A Lead Agent decomposes a task into a WorkflowRevision DAG, enforces topological ordering
//! via Kahn's algorithm, and synthesises worker results through JoinPolicy::CoverageThreshold.
//!
//! shortcut: worker execution is simulated (payload strings, not real LLM calls). Upgrade when
//!   runtime has AgentDriver injection support inside topology coordinators.

use crate::oi::join_policy::{JoinEvaluation, JoinEvaluator, JoinPolicy};
use custos_domain::oi::{WorkerResult, WorkerStatus};
use custos_domain::workflow::{RevisionNode, WorkflowRevision};
use custos_domain::DomainError;
use std::collections::{HashMap, VecDeque};

pub struct T5LeadWorkerCoordinator;

/// A node enriched with its resolved topological position.
#[derive(Debug, Clone)]
pub struct OrderedNode<'a> {
    pub node: &'a RevisionNode,
    pub topo_index: usize,
    /// IDs of nodes that must complete before this one.
    pub depends_on: Vec<String>,
}

/// Output of Lead decomposition: nodes in guaranteed execution order.
#[derive(Debug)]
pub struct LeadDecomposition<'a> {
    pub ordered: Vec<OrderedNode<'a>>,
}

/// Final result after Lead synthesises all worker results.
#[derive(Debug)]
pub struct T5SynthesisResult {
    pub payload: String,
    pub worker_results: Vec<WorkerResult>,
    pub join_evaluation: JoinEvaluation,
}

impl T5LeadWorkerCoordinator {
    /// Lead phase: validates the revision DAG, detects cycles, returns topological ordering.
    ///
    /// Uses Kahn's BFS algorithm — O(V + E).
    pub fn lead_decompose<'a>(
        revision: &'a WorkflowRevision,
    ) -> Result<LeadDecomposition<'a>, DomainError> {
        let nodes = &revision.nodes;
        let deps = &revision.dependencies;

        if nodes.is_empty() {
            return Err(DomainError::Validation(
                "T5: revision has no nodes".into(),
            ));
        }

        // Build in-degree and adjacency for Kahn's sort
        let node_map: HashMap<&str, &RevisionNode> =
            nodes.iter().map(|n| (n.node_id.as_str(), n)).collect();

        let mut in_degree: HashMap<&str, usize> =
            node_map.keys().map(|k| (*k, 0usize)).collect();
        let mut adj: HashMap<&str, Vec<&str>> =
            node_map.keys().map(|k| (*k, vec![])).collect();

        for (from, to) in deps {
            if !node_map.contains_key(from.as_str()) || !node_map.contains_key(to.as_str()) {
                return Err(DomainError::Validation(format!(
                    "T5: dependency references unknown node: {from} → {to}"
                )));
            }
            adj.get_mut(from.as_str()).unwrap().push(to.as_str());
            *in_degree.get_mut(to.as_str()).unwrap() += 1;
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut topo: Vec<&str> = Vec::with_capacity(nodes.len());

        while let Some(id) = queue.pop_front() {
            topo.push(id);
            for &next in adj.get(id).unwrap_or(&vec![]) {
                let d = in_degree.get_mut(next).unwrap();
                *d -= 1;
                if *d == 0 {
                    queue.push_back(next);
                }
            }
        }

        if topo.len() != nodes.len() {
            return Err(DomainError::Validation(
                "T5: cycle detected in DAG — cannot decompose".into(),
            ));
        }

        // Build predecessor map for depends_on metadata
        let mut pred_map: HashMap<&str, Vec<String>> =
            node_map.keys().map(|k| (*k, vec![])).collect();
        for (from, to) in deps {
            pred_map.get_mut(to.as_str()).unwrap().push(from.clone());
        }

        let ordered = topo
            .iter()
            .enumerate()
            .map(|(i, &id)| OrderedNode {
                node: node_map[id],
                topo_index: i,
                depends_on: pred_map[id].clone(),
            })
            .collect();

        Ok(LeadDecomposition { ordered })
    }

    /// Verifies that each node's dependencies completed successfully before it ran.
    ///
    /// `results` must correspond 1:1 with `ordered` (same index).
    pub fn verify_dag_execution(
        ordered: &[OrderedNode<'_>],
        results: &[WorkerResult],
    ) -> Result<(), DomainError> {
        if results.len() != ordered.len() {
            return Err(DomainError::Validation(format!(
                "T5: expected {} results, got {}",
                ordered.len(),
                results.len()
            )));
        }

        // Map packet_id → result for dependency lookups
        let result_by_packet: HashMap<&str, &WorkerResult> =
            results.iter().map(|r| (r.packet_id.as_str(), r)).collect();

        // Map node_id → packet_id (index position)
        let packet_id_by_node: HashMap<&str, &str> = ordered
            .iter()
            .zip(results.iter())
            .map(|(on, r)| (on.node.node_id.as_str(), r.packet_id.as_str()))
            .collect();

        for (on, _) in ordered.iter().zip(results.iter()) {
            for dep_node_id in &on.depends_on {
                if let Some(&pkt_id) = packet_id_by_node.get(dep_node_id.as_str()) {
                    if let Some(dep_result) = result_by_packet.get(pkt_id) {
                        if dep_result.status == WorkerStatus::Failed {
                            return Err(DomainError::Validation(format!(
                                "T5: node '{}' blocked — dependency '{}' failed",
                                on.node.node_id, dep_node_id
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Synthesis phase: aggregates worker results; uses CoverageThreshold(0.8) as gate.
    ///
    /// shortcut: payload concatenation; replace with LLM Lead synthesis when AgentDriver
    ///   injection is available inside topology coordinators.
    pub fn synthesise(
        worker_results: Vec<WorkerResult>,
        task_id: &str,
    ) -> Result<T5SynthesisResult, DomainError> {
        let join_evaluation = JoinEvaluator::evaluate(
            &JoinPolicy::CoverageThreshold { threshold: 0.8 },
            &worker_results,
        );

        match &join_evaluation {
            JoinEvaluation::Rejected { reason } => {
                return Err(DomainError::Validation(format!(
                    "T5 synthesis rejected: {reason}"
                )));
            }
            JoinEvaluation::RetryRequired { next_iteration, feedback } => {
                return Err(DomainError::Validation(format!(
                    "T5 synthesis needs retry (iteration {next_iteration}): {feedback}"
                )));
            }
            JoinEvaluation::Approved { .. } => {}
        }

        let fragment = worker_results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("[W{}] {}", i + 1, r.payload))
            .collect::<Vec<_>>()
            .join(" | ");

        let payload = format!(
            "[T5 Lead-Worker] task={task_id} workers={} synthesis={}",
            worker_results.len(),
            &fragment[..fragment.len().min(300)]
        );

        Ok(T5SynthesisResult { payload, worker_results, join_evaluation })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::workflow::{RevisionNode, WorkflowRevision};
    use custos_domain::oi::WorkerResult;

    fn revision_with(node_ids: &[&str], deps: &[(&str, &str)]) -> WorkflowRevision {
        let mut rev = WorkflowRevision::new("task-t", "prop-p", 1);
        for &id in node_ids {
            rev.nodes.push(RevisionNode {
                node_id: id.to_string(),
                step_name: id.to_string(),
                role: "worker".into(),
                harness_id: "goose".into(),
                allocated_budget_tokens: 1000,
                read_set: vec![],
                write_set: vec![],
                required_capabilities: vec![],
            });
        }
        for &(from, to) in deps {
            rev.dependencies.push((from.into(), to.into()));
        }
        rev
    }

    fn ok_result(packet_id: &str) -> WorkerResult {
        WorkerResult::success(packet_id, format!("done-{packet_id}"))
    }

    fn fail_result(packet_id: &str) -> WorkerResult {
        WorkerResult::failed(packet_id, "boom")
    }

    #[test]
    fn linear_dag_a_before_b_before_c() {
        let rev = revision_with(&["c", "b", "a"], &[("a", "b"), ("b", "c")]);
        let decomp = T5LeadWorkerCoordinator::lead_decompose(&rev).unwrap();
        let order: Vec<&str> = decomp.ordered.iter().map(|n| n.node.node_id.as_str()).collect();
        let pos = |x: &str| order.iter().position(|&v| v == x).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("b") < pos("c"));
    }

    #[test]
    fn cycle_returns_error() {
        let rev = revision_with(&["x", "y"], &[("x", "y"), ("y", "x")]);
        let err = T5LeadWorkerCoordinator::lead_decompose(&rev).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn synthesis_rejected_when_all_workers_fail() {
        let results = vec![fail_result("p1"), fail_result("p2")];
        let err = T5LeadWorkerCoordinator::synthesise(results, "t").unwrap_err();
        assert!(err.to_string().contains("rejected") || err.to_string().contains("retry"));
    }

    #[test]
    fn synthesis_succeeds_on_sufficient_workers() {
        let results = vec![ok_result("p1"), ok_result("p2"), ok_result("p3")];
        let res = T5LeadWorkerCoordinator::synthesise(results, "t").unwrap();
        assert!(res.payload.contains("T5"));
    }
}
