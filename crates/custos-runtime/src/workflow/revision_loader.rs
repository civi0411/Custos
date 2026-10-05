use super::dag::{DagGraph, DagNode};
use custos_domain::DomainError;
use custos_domain::WorkflowRevision;
use std::collections::HashMap;

pub struct RevisionLoader;

impl RevisionLoader {
    pub fn load_into_dag(revision: &WorkflowRevision) -> Result<DagGraph, DomainError> {
        let mut graph = DagGraph::new(format!("DAG for task {}", revision.task_id));
        graph.id = revision.revision_id.clone();

        let mut deps: HashMap<String, Vec<String>> = HashMap::new();
        for (from, to) in &revision.dependencies {
            deps.entry(to.clone()).or_default().push(from.clone());
        }

        for rev_node in &revision.nodes {
            let node_deps = deps.get(&rev_node.node_id).cloned().unwrap_or_default();

            let inputs = serde_json::json!({
                "role": rev_node.role,
                "harness_id": rev_node.harness_id,
                "allocated_budget_tokens": rev_node.allocated_budget_tokens,
                "read_set": rev_node.read_set,
                "write_set": rev_node.write_set,
                "required_capabilities": rev_node.required_capabilities,
            });

            let dag_node = DagNode::new(
                rev_node.node_id.clone(),
                rev_node.step_name.clone(),
                "execute_worker",
                inputs,
                node_deps,
            );

            graph.add_node(dag_node)?;
        }

        graph.validate_dag()?;
        Ok(graph)
    }
}
