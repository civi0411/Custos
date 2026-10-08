use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRecordStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// ExecutionRecord captures the *logs and outputs* of a specific run of a Recipe.
/// It separates the execution history (stdout, stderr, exit code) from the reproducible logic (Recipe).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: String,
    pub recipe_id: String,
    pub session_id: Option<String>,
    pub status: ExecutionRecordStatus,
    pub exit_code: Option<i32>,
    pub stdout_cas_uri: Option<String>,
    pub stderr_cas_uri: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub artifacts: Vec<ExecutionArtifact>,
}

impl ExecutionRecord {
    pub fn new(recipe_id: impl Into<String>) -> Self {
        Self {
            id: new_id("exec_rec"),
            recipe_id: recipe_id.into(),
            session_id: None,
            status: ExecutionRecordStatus::Pending,
            exit_code: None,
            stdout_cas_uri: None,
            stderr_cas_uri: None,
            started_at: Utc::now(),
            ended_at: None,
            artifacts: Vec::new(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn transition(&mut self, next: ExecutionRecordStatus) {
        self.status = next;
        if matches!(next, ExecutionRecordStatus::Completed | ExecutionRecordStatus::Failed | ExecutionRecordStatus::Cancelled) {
            self.ended_at = Some(Utc::now());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionArtifact {
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
}

impl ExecutionArtifact {
    pub fn new(path: impl Into<String>, content_hash: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            content_hash: content_hash.into(),
            size_bytes,
        }
    }
}
