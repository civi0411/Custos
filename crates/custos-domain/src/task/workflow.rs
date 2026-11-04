//! Workflow Plan & Execution Step Specifications
//!
//! Typed, reviewable plans decomposed from intent for governed task execution.

use crate::ids::new_id;
use chrono::{DateTime, Utc};
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

/// Typed, reviewable workflow revision produced by the compiler (RFC 004 §2C)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRevision {
    pub revision_id: String,
    pub task_id: String,
    pub proposal_id: String,
    pub revision_number: u32,
    pub nodes: Vec<RevisionNode>,
    pub dependencies: Vec<(String, String)>,
    pub obligations: Vec<String>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionNode {
    pub node_id: String,
    pub step_name: String,
    pub role: String,
    pub harness_id: String,
    pub allocated_budget_tokens: u64,
    pub read_set: Vec<String>,
    pub write_set: Vec<String>,
    pub required_capabilities: Vec<String>,
}

impl WorkflowRevision {
    pub fn new(
        task_id: impl Into<String>,
        proposal_id: impl Into<String>,
        revision_number: u32,
    ) -> Self {
        Self {
            revision_id: new_id("wrev"),
            task_id: task_id.into(),
            proposal_id: proposal_id.into(),
            revision_number,
            nodes: Vec::new(),
            dependencies: Vec::new(),
            obligations: Vec::new(),
            created_at: Utc::now(),
        }
    }
}

/// Specification for multi-candidate race execution across isolated worktrees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRaceSpec {
    pub id: String,
    pub task_id: String,
    pub base_workspace_id: String,
    pub candidates: Vec<ModelCandidateConfig>,
    pub test_command: Option<String>,
    pub timeout_seconds: u64,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCandidateConfig {
    pub candidate_id: String,
    pub model_id: String,
    pub harness_id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub temperature: Option<f32>,
}

impl ModelCandidateConfig {
    pub fn new(
        candidate_id: impl Into<String>,
        model_id: impl Into<String>,
        harness_id: impl Into<String>,
    ) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            model_id: model_id.into(),
            harness_id: harness_id.into(),
            workspace_id: None,
            temperature: None,
        }
    }

    pub fn with_workspace_id(mut self, ws_id: impl Into<String>) -> Self {
        self.workspace_id = Some(ws_id.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }
}

impl CandidateRaceSpec {
    pub fn new(
        task_id: impl Into<String>,
        base_workspace_id: impl Into<String>,
        candidates: Vec<ModelCandidateConfig>,
    ) -> Self {
        Self {
            id: new_id("race"),
            task_id: task_id.into(),
            base_workspace_id: base_workspace_id.into(),
            candidates,
            test_command: None,
            timeout_seconds: 600,
            created_at: Utc::now(),
        }
    }

    pub fn with_test_command(mut self, cmd: impl Into<String>) -> Self {
        self.test_command = Some(cmd.into());
        self
    }

    pub fn with_timeout_seconds(mut self, timeout: u64) -> Self {
        self.timeout_seconds = timeout;
        self
    }
}

/// Individual candidate execution and verification scorecard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateExecutionReport {
    pub candidate_id: String,
    pub model_id: String,
    pub workspace_id: String,
    pub worktree_path: String,
    pub test_passed: bool,
    pub tests_succeeded: u32,
    pub tests_failed: u32,
    pub test_output: String,
    pub modified_files: Vec<String>,
    pub total_tokens: u64,
    pub execution_duration_ms: u64,
}

/// Aggregated multi-candidate race outcome and winning verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRaceResult {
    pub race_id: String,
    pub task_id: String,
    pub winner_candidate_id: Option<String>,
    pub candidate_reports: Vec<CandidateExecutionReport>,
    #[serde(default = "Utc::now")]
    pub completed_at: DateTime<Utc>,
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

    #[test]
    fn test_candidate_race_spec_construction() {
        let candidates = vec![
            ModelCandidateConfig::new("cand_claude", "claude-3-5-sonnet", "claude_code"),
            ModelCandidateConfig::new("cand_gpt4o", "gpt-4o", "generic_cli"),
        ];
        let spec = CandidateRaceSpec::new("task_race_01", "ws_base_01", candidates)
            .with_test_command("cargo test")
            .with_timeout_seconds(300);

        assert!(spec.id.starts_with("race_"));
        assert_eq!(spec.task_id, "task_race_01");
        assert_eq!(spec.candidates.len(), 2);
        assert_eq!(spec.test_command, Some("cargo test".into()));
        assert_eq!(spec.timeout_seconds, 300);
    }
}
