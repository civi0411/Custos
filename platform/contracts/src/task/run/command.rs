use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::RunStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRunCommand {
    pub task_id: String,
    pub actor: String,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub workflow_revision: Option<String>,
    #[serde(default)]
    pub preferred_mode: Option<String>,
    #[serde(default)]
    pub harness_id: Option<String>,
    #[serde(default)]
    pub workspace_root: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub provider_account_id: Option<String>,
}

impl StartRunCommand {
    pub fn new(task_id: impl Into<String>, actor: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            actor: actor.into(),
            node_id: None,
            workflow_revision: None,
            preferred_mode: None,
            harness_id: None,
            workspace_root: None,
            workspace_id: None,
            session_id: None,
            turn_id: None,
            prompt: None,
            model: None,
            reasoning_effort: None,
            provider_account_id: None,
        }
    }

    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }

    pub fn with_workflow_revision(mut self, revision: impl Into<String>) -> Self {
        self.workflow_revision = Some(revision.into());
        self
    }

    pub fn with_preferred_mode(mut self, mode: impl Into<String>) -> Self {
        self.preferred_mode = Some(mode.into());
        self
    }

    pub fn with_harness_id(mut self, harness_id: impl Into<String>) -> Self {
        self.harness_id = Some(harness_id.into());
        self
    }

    pub fn with_workspace_root(mut self, root: impl Into<String>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_turn_id(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHandle {
    pub run_id: String,
    pub task_id: String,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RunHandle {
    pub fn new(
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        status: RunStatus,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            task_id: task_id.into(),
            status,
            started_at,
            completed_at: None,
            output: None,
        }
    }

    pub fn with_completion(
        mut self,
        completed_at: Option<DateTime<Utc>>,
        output: Option<String>,
    ) -> Self {
        self.completed_at = completed_at;
        self.output = output;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelReceipt {
    pub run_id: String,
    pub cancelled_at: DateTime<Utc>,
    pub reason: String,
    pub uncertain_effects_count: u32,
}
