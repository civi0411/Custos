//! Workflow Plan & Execution Step Specifications
//!
//! Typed, reviewable plans decomposed from intent for governed task execution.

use crate::ids::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action_type: String,
    pub inputs: serde_json::Value,
    pub depends_on: Vec<String>,
}

impl WorkflowStep {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        action_type: impl Into<String>,
        inputs: serde_json::Value,
    ) -> Self {
        Self {
            id: new_id("step"),
            name: name.into(),
            description: description.into(),
            action_type: action_type.into(),
            inputs,
            depends_on: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPlan {
    pub id: String,
    pub task_id: String,
    pub domain: String,
    pub title: String,
    pub steps: Vec<WorkflowStep>,
    pub metadata: serde_json::Value,
}

impl WorkflowPlan {
    pub fn new(
        task_id: impl Into<String>,
        domain: impl Into<String>,
        title: impl Into<String>,
        steps: Vec<WorkflowStep>,
    ) -> Self {
        Self {
            id: new_id("plan"),
            task_id: task_id.into(),
            domain: domain.into(),
            title: title.into(),
            steps,
            metadata: serde_json::json!({}),
        }
    }
}

/// Intermediate Representation (IR) for compiled DAG execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIR {
    pub plan_id: String,
    pub root_task_id: String,
    pub nodes: Vec<WorkflowNodeIR>,
    pub dependencies: Vec<(String, String)>, // (from_node_id, to_node_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeIR {
    pub node_id: String,
    pub step_name: String,
    pub action_type: String,
    pub allocated_budget_tokens: u64,
}

impl WorkflowIR {
    pub fn new(plan_id: impl Into<String>, root_task_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            root_task_id: root_task_id.into(),
            nodes: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn add_node(
        &mut self,
        node_id: impl Into<String>,
        step_name: impl Into<String>,
        action_type: impl Into<String>,
        allocated_budget_tokens: u64,
    ) {
        self.nodes.push(WorkflowNodeIR {
            node_id: node_id.into(),
            step_name: step_name.into(),
            action_type: action_type.into(),
            allocated_budget_tokens,
        });
    }

    pub fn add_dependency(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.dependencies.push((from.into(), to.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_ir_construction() {
        let mut ir = WorkflowIR::new("plan_01", "task_01");
        ir.add_node("node_1", "Analyze AST", "read_ast", 1000);
        ir.add_node("node_2", "Generate Patch", "write_patch", 2000);
        ir.add_dependency("node_1", "node_2");

        assert_eq!(ir.nodes.len(), 2);
        assert_eq!(ir.dependencies.len(), 1);
        assert_eq!(ir.dependencies[0], ("node_1".into(), "node_2".into()));
    }
}
