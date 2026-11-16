//! Workflow DAG Engine & Dependency Resolver (Gate 5)
//!
//! Validates directed acyclic graphs, performs cycle detection (Kahn's algorithm),
//! computes topological execution waves, and orchestrates dependency-gated step advancement.

use custos_domain::{new_id, DomainError, WorkflowPlan};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    WaitingApproval,
    Uncertain,
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

    /// Returns the strongly-typed WorkflowNodeId for this node.
    pub fn typed_id(&self) -> custos_domain::WorkflowNodeId {
        custos_domain::WorkflowNodeId::new(&self.id)
    }

    /// Returns strongly-typed WorkflowNodeId vector for all dependencies.
    pub fn typed_dependencies(&self) -> Vec<custos_domain::WorkflowNodeId> {
        self.depends_on
            .iter()
            .map(custos_domain::WorkflowNodeId::new)
            .collect()
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
    #[serde(default)]
    pub task_id: Option<String>,
    pub title: String,
    nodes: HashMap<String, DagNode>,
}

impl DagGraph {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: new_id("dag"),
            task_id: None,
            title: title.into(),
            nodes: HashMap::new(),
        }
    }

    pub fn from_plan(plan: &WorkflowPlan) -> Result<Self, DomainError> {
        let mut graph = Self::new(&plan.title);
        graph.id = plan.id.clone();
        graph.task_id = Some(plan.task_id.clone());

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
                self.id,
                visited_count,
                self.nodes.len()
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
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "DagNode".into(),
                id: node_id.into(),
            })?;
        node.status = NodeStatus::Running;
        Ok(())
    }

    /// Holds a node without releasing dependents or making it eligible for automatic retry.
    pub fn mark_blocked(
        &mut self,
        node_id: &str,
        status: NodeStatus,
        output: serde_json::Value,
    ) -> Result<(), DomainError> {
        if !matches!(status, NodeStatus::WaitingApproval | NodeStatus::Uncertain) {
            return Err(DomainError::Validation(
                "Invalid blocked node status".into(),
            ));
        }
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "DagNode".into(),
                id: node_id.into(),
            })?;
        node.status = status;
        node.output = Some(output);
        Ok(())
    }

    /// Marks a node as completed with an optional output payload.
    pub fn mark_completed(
        &mut self,
        node_id: &str,
        output: Option<serde_json::Value>,
    ) -> Result<(), DomainError> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "DagNode".into(),
                id: node_id.into(),
            })?;
        node.status = NodeStatus::Completed;
        node.output = output;
        Ok(())
    }

    /// Marks a node as failed, and cascades `Skipped` status to all downstream dependent nodes.
    pub fn mark_failed(
        &mut self,
        node_id: &str,
        reason: impl Into<String>,
    ) -> Result<(), DomainError> {
        let err_msg = reason.into();
        {
            let node = self
                .nodes
                .get_mut(node_id)
                .ok_or_else(|| DomainError::NotFound {
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
                    node.error = Some(format!(
                        "Skipped due to upstream failure in node '{}'",
                        failed_parent
                    ));
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
        self.nodes
            .values()
            .all(|n| n.status == NodeStatus::Completed)
    }
}
