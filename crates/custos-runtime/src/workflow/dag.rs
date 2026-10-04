//! Workflow DAG Engine & Dependency Resolver (Gate 5)
//!
//! Validates directed acyclic graphs, performs cycle detection (Kahn's algorithm),
//! computes topological execution waves, and orchestrates dependency-gated step advancement.

use std::collections::{HashMap, VecDeque};
use custos_domain::{new_id, DomainError, WorkflowPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl NodeStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: String,
    pub name: String,
    pub action_type: String,
    pub inputs: serde_json::Value,
    pub depends_on: Vec<String>,
    pub status: NodeStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl DagNode {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        action_type: impl Into<String>,
        inputs: serde_json::Value,
        depends_on: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            action_type: action_type.into(),
            inputs,
            depends_on,
            status: NodeStatus::Pending,
            output: None,
            error: None,
        }
    }
}

/// Execution Wave: A set of node IDs whose dependencies have all completed,
/// meaning they can safely be executed in parallel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWave {
    pub wave_index: usize,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagGraph {
    pub id: String,
    pub title: String,
    nodes: HashMap<String, DagNode>,
}

impl DagGraph {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: new_id("dag"),
            title: title.into(),
            nodes: HashMap::new(),
        }
    }

    pub fn from_plan(plan: &WorkflowPlan) -> Result<Self, DomainError> {
        let mut graph = Self::new(&plan.title);
        graph.id = plan.id.clone();

        for step in &plan.steps {
            graph.add_node(DagNode::new(
                step.id.clone(),
                step.name.clone(),
                step.action_type.clone(),
                step.inputs.clone(),
                step.depends_on.clone(),
            ))?;
        }

        // Validate graph integrity (Gate 5)
        graph.validate_dag()?;

        Ok(graph)
    }

    pub fn add_node(&mut self, node: DagNode) -> Result<(), DomainError> {
        if self.nodes.contains_key(&node.id) {
            return Err(DomainError::Conflict(format!(
                "Node '{}' already exists in DAG graph '{}'",
                node.id, self.id
            )));
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&DagNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut DagNode> {
        self.nodes.get_mut(id)
    }

    pub fn nodes(&self) -> &HashMap<String, DagNode> {
        &self.nodes
    }

    /// Gate 5: Strict DAG validation enforcing no cycles, no self-references,
    /// and all referenced dependencies existing in the graph.
    pub fn validate_dag(&self) -> Result<(), DomainError> {
        // 1. Verify all dependencies exist and no self-loops
        for (node_id, node) in &self.nodes {
            for dep in &node.depends_on {
                if dep == node_id {
                    return Err(DomainError::Validation(format!(
                        "Gate 5 Violation: Self-dependency detected on node '{}'",
                        node_id
                    )));
                }
                if !self.nodes.contains_key(dep) {
                    return Err(DomainError::Validation(format!(
                        "Gate 5 Violation: Node '{}' depends on non-existent prerequisite node '{}'",
                        node_id, dep
                    )));
                }
            }
        }

        // 2. Kahn's algorithm for topological sorting and cycle detection
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        for node_id in self.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
            adj_list.insert(node_id.clone(), Vec::new());
        }

        for (node_id, node) in &self.nodes {
            for dep in &node.depends_on {
                adj_list.get_mut(dep).unwrap().push(node_id.clone());
                *in_degree.get_mut(node_id).unwrap() += 1;
            }
        }

        let mut queue: VecDeque<String> = VecDeque::new();
        for (node_id, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(node_id.clone());
            }
        }

        let mut visited_count = 0;
        while let Some(current) = queue.pop_front() {
            visited_count += 1;
            if let Some(neighbors) = adj_list.get(&current) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        if visited_count != self.nodes.len() {
            return Err(DomainError::Validation(format!(
                "Gate 5 Violation: Cycle detected in workflow graph '{}' (resolved {} of {} nodes)",
                self.id, visited_count, self.nodes.len()
            )));
        }

        Ok(())
    }

    /// Computes parallel execution waves using topological leveling.
    pub fn compute_execution_waves(&self) -> Result<Vec<ExecutionWave>, DomainError> {
        self.validate_dag()?;

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        for node_id in self.nodes.keys() {
            in_degree.insert(node_id.clone(), 0);
            adj_list.insert(node_id.clone(), Vec::new());
        }

        for (node_id, node) in &self.nodes {
            for dep in &node.depends_on {
                adj_list.get_mut(dep).unwrap().push(node_id.clone());
                *in_degree.get_mut(node_id).unwrap() += 1;
            }
        }

        let mut current_wave: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        current_wave.sort();

        let mut waves: Vec<ExecutionWave> = Vec::new();
        let mut wave_index = 0;

        while !current_wave.is_empty() {
            waves.push(ExecutionWave {
                wave_index,
                node_ids: current_wave.clone(),
            });

            let mut next_wave = Vec::new();
            for node_id in current_wave {
                if let Some(neighbors) = adj_list.get(&node_id) {
                    for neighbor in neighbors {
                        let deg = in_degree.get_mut(neighbor).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            next_wave.push(neighbor.clone());
                        }
                    }
                }
            }

            next_wave.sort();
            current_wave = next_wave;
            wave_index += 1;
        }

        Ok(waves)
    }

    /// Returns the list of node IDs that are currently ready to execute
    /// (status == Pending and all dependencies have status == Completed).
    pub fn ready_nodes(&self) -> Vec<String> {
        let mut ready = Vec::new();
        for (id, node) in &self.nodes {
            if node.status == NodeStatus::Pending {
                let all_deps_completed = node.depends_on.iter().all(|dep_id| {
                    self.nodes
                        .get(dep_id)
                        .map(|dep| dep.status == NodeStatus::Completed)
                        .unwrap_or(false)
                });
                if all_deps_completed {
                    ready.push(id.clone());
                }
            }
        }
        ready.sort();
        ready
    }

    /// Marks a node as running.
    pub fn mark_running(&mut self, node_id: &str) -> Result<(), DomainError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| DomainError::NotFound {
            kind: "DagNode".into(),
            id: node_id.into(),
        })?;
        node.status = NodeStatus::Running;
        Ok(())
    }

    /// Marks a node as completed with an optional output payload.
    pub fn mark_completed(
        &mut self,
        node_id: &str,
        output: Option<serde_json::Value>,
    ) -> Result<(), DomainError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| DomainError::NotFound {
            kind: "DagNode".into(),
            id: node_id.into(),
        })?;
        node.status = NodeStatus::Completed;
        node.output = output;
        Ok(())
    }

    /// Marks a node as failed, and cascades `Skipped` status to all downstream dependent nodes.
    pub fn mark_failed(&mut self, node_id: &str, reason: impl Into<String>) -> Result<(), DomainError> {
        let err_msg = reason.into();
        {
            let node = self.nodes.get_mut(node_id).ok_or_else(|| DomainError::NotFound {
                kind: "DagNode".into(),
                id: node_id.into(),
            })?;
            node.status = NodeStatus::Failed;
            node.error = Some(err_msg.clone());
        }

        // Cascade skip to all transitive dependents
        let mut to_skip = VecDeque::new();
        to_skip.push_back(node_id.to_string());

        while let Some(failed_parent) = to_skip.pop_front() {
            for (id, node) in self.nodes.iter_mut() {
                if node.status == NodeStatus::Pending && node.depends_on.contains(&failed_parent) {
                    node.status = NodeStatus::Skipped;
                    node.error = Some(format!("Skipped due to upstream failure in node '{}'", failed_parent));
                    to_skip.push_back(id.clone());
                }
            }
        }

        Ok(())
    }

    /// Returns true when all nodes in the DAG have reached a terminal state
    /// (Completed, Failed, or Skipped).
    pub fn is_finished(&self) -> bool {
        self.nodes.values().all(|n| n.status.is_terminal())
    }

    /// Returns true if all nodes completed successfully.
    pub fn is_all_completed(&self) -> bool {
        self.nodes.values().all(|n| n.status == NodeStatus::Completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_dag_topological_waves() {
        // Diamond Graph:
        //      A
        //     / \
        //    B   C
        //     \ /
        //      D
        let mut graph = DagGraph::new("Diamond Test");
        graph
            .add_node(DagNode::new("A", "Root", "read", serde_json::json!({}), vec![]))
            .unwrap();
        graph
            .add_node(DagNode::new("B", "Branch B", "process", serde_json::json!({}), vec!["A".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("C", "Branch C", "process", serde_json::json!({}), vec!["A".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("D", "Join D", "merge", serde_json::json!({}), vec!["B".into(), "C".into()]))
            .unwrap();

        assert!(graph.validate_dag().is_ok());

        let waves = graph.compute_execution_waves().unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0].node_ids, vec!["A"]);
        assert_eq!(waves[1].node_ids, vec!["B", "C"]);
        assert_eq!(waves[2].node_ids, vec!["D"]);
    }

    #[test]
    fn test_gate_5_cycle_detection_rejected() {
        // Cycle: A -> B -> C -> A
        let mut graph = DagGraph::new("Cycle Test");
        graph
            .add_node(DagNode::new("A", "Node A", "exec", serde_json::json!({}), vec!["C".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("B", "Node B", "exec", serde_json::json!({}), vec!["A".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("C", "Node C", "exec", serde_json::json!({}), vec!["B".into()]))
            .unwrap();

        let err = graph.validate_dag().unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("Cycle detected"));
    }

    #[test]
    fn test_gate_5_self_dependency_rejected() {
        let mut graph = DagGraph::new("Self Loop Test");
        graph
            .add_node(DagNode::new("A", "Node A", "exec", serde_json::json!({}), vec!["A".into()]))
            .unwrap();

        let err = graph.validate_dag().unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("Self-dependency"));
    }

    #[test]
    fn test_gate_5_missing_dependency_rejected() {
        let mut graph = DagGraph::new("Missing Dep Test");
        graph
            .add_node(DagNode::new("A", "Node A", "exec", serde_json::json!({}), vec!["non_existent".into()]))
            .unwrap();

        let err = graph.validate_dag().unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
        assert!(err.to_string().contains("non-existent prerequisite"));
    }

    #[test]
    fn test_failure_cascades_skip_to_downstream_dependents() {
        // A -> B -> D
        // A -> C -> E
        let mut graph = DagGraph::new("Cascade Test");
        graph
            .add_node(DagNode::new("A", "A", "exec", serde_json::json!({}), vec![]))
            .unwrap();
        graph
            .add_node(DagNode::new("B", "B", "exec", serde_json::json!({}), vec!["A".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("C", "C", "exec", serde_json::json!({}), vec!["A".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("D", "D", "exec", serde_json::json!({}), vec!["B".into()]))
            .unwrap();
        graph
            .add_node(DagNode::new("E", "E", "exec", serde_json::json!({}), vec!["C".into()]))
            .unwrap();

        // 1. Initial ready nodes is just A
        assert_eq!(graph.ready_nodes(), vec!["A"]);
        graph.mark_completed("A", None).unwrap();

        // 2. Ready nodes are now B and C
        assert_eq!(graph.ready_nodes(), vec!["B", "C"]);

        // 3. B fails!
        graph.mark_failed("B", "Network timeout").unwrap();
        assert_eq!(graph.get_node("B").unwrap().status, NodeStatus::Failed);

        // 4. D must be automatically marked as Skipped!
        assert_eq!(graph.get_node("D").unwrap().status, NodeStatus::Skipped);

        // 5. C is unaffected and still ready
        assert_eq!(graph.ready_nodes(), vec!["C"]);
        graph.mark_completed("C", None).unwrap();

        // 6. E is now ready
        assert_eq!(graph.ready_nodes(), vec!["E"]);
        graph.mark_completed("E", None).unwrap();

        // 7. Graph finished
        assert!(graph.is_finished());
        assert!(!graph.is_all_completed()); // because B failed and D skipped
    }
}
