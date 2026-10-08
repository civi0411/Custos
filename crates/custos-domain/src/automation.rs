//! Headless Automation Domain Models
//!
//! Models for automated tasks, cron / event triggers, scheduled batch jobs,
//! and contract evidence verification.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum HeadlessTrigger {
    Manual,
    Cron { schedule: String },
    Webhook { endpoint: String },
    OnCommit { branch: String },
    PipelineStep { parent_task_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessTaskSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    pub target_type: String, // "local_process" | "remote_ssh" | "browser_session"
    pub command_or_script: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub required_evidence: Vec<String>,
}

fn default_timeout_secs() -> u64 {
    300
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessAutomationJob {
    pub id: String,
    pub name: String,
    pub spec: HeadlessTaskSpec,
    pub trigger: HeadlessTrigger,
    pub status: HeadlessJobStatus,
    pub exit_code: Option<i32>,
    pub output_log: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

impl HeadlessAutomationJob {
    pub fn new(
        name: impl Into<String>,
        spec: HeadlessTaskSpec,
        trigger: HeadlessTrigger,
    ) -> Result<Self, DomainError> {
        let name_str = name.into();
        if name_str.trim().is_empty() {
            return Err(DomainError::Validation("Job name cannot be empty.".to_string()));
        }
        if spec.command_or_script.trim().is_empty() {
            return Err(DomainError::Validation("Job command or script cannot be empty.".to_string()));
        }

        let now = chrono::Utc::now().timestamp_millis();
        Ok(Self {
            id: new_id("job"),
            name: name_str,
            spec,
            trigger,
            status: HeadlessJobStatus::Pending,
            exit_code: None,
            output_log: String::new(),
            created_at: now,
            started_at: None,
            completed_at: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateHeadlessJobParams {
    pub name: String,
    pub spec: HeadlessTaskSpec,
    pub trigger: Option<HeadlessTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHeadlessJobResult {
    pub job_id: String,
    pub status: HeadlessJobStatus,
    pub exit_code: Option<i32>,
    pub output_log: String,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_job_creation() {
        let spec = HeadlessTaskSpec {
            workspace_id: Some("ws_1".to_string()),
            target_type: "local_process".to_string(),
            command_or_script: "cargo test --all".to_string(),
            timeout_secs: 60,
            env: HashMap::new(),
            required_evidence: vec!["TestResult".to_string()],
        };

        let job = HeadlessAutomationJob::new(
            "nightly-test-gate",
            spec,
            HeadlessTrigger::Cron {
                schedule: "0 0 * * *".to_string(),
            },
        ).unwrap();

        assert!(job.id.starts_with("job_"));
        assert_eq!(job.status, HeadlessJobStatus::Pending);
    }
}
