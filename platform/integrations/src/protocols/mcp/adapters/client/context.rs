use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContext {
    pub session_id: String,
    pub tool_call_request_id: String,
    pub working_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permit_id: Option<String>,
}

impl ToolCallContext {
    pub fn new(session_id: impl Into<String>, tool_call_request_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            tool_call_request_id: tool_call_request_id.into(),
            working_dir: None,
            task_id: None,
            run_id: None,
            permit_id: None,
        }
    }

    pub fn with_working_dir(mut self, path: PathBuf) -> Self {
        self.working_dir = Some(path);
        self
    }

    /// Attach SADE execution governance identifiers (Task, Run, and ExecutionPermit)
    pub fn with_sade_scope(
        mut self,
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        permit_id: Option<String>,
    ) -> Self {
        self.task_id = Some(task_id.into());
        self.run_id = Some(run_id.into());
        self.permit_id = permit_id;
        self
    }
}
