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
