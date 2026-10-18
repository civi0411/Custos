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
    Preempted,
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
    pub checkpoint: Option<crate::compute::TrainingCheckpointManifest>,
    pub resume_from_attempt_id: Option<String>,
    pub resumed_from_checkpoint_id: Option<String>,
    pub backend: Option<crate::compute::ComputeBackendKind>,
    pub output_namespace: Option<String>,
    pub latest_observed_step: Option<u64>,
    pub latest_local_checkpoint_step: Option<u64>,
    pub latest_protected_checkpoint_step: Option<u64>,
    pub last_protected_at: Option<DateTime<Utc>>,
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
            checkpoint: None,
            resume_from_attempt_id: None,
            resumed_from_checkpoint_id: None,
            backend: None,
            output_namespace: None,
            latest_observed_step: None,
            latest_local_checkpoint_step: None,
            latest_protected_checkpoint_step: None,
            last_protected_at: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_backend(mut self, backend: crate::compute::ComputeBackendKind) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn with_checkpoint(mut self, checkpoint: crate::compute::TrainingCheckpointManifest) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    pub fn with_resume_from(mut self, previous_attempt_id: impl Into<String>) -> Self {
        self.resume_from_attempt_id = Some(previous_attempt_id.into());
        self
    }

    pub fn with_resumed_from_checkpoint(mut self, checkpoint_id: impl Into<String>) -> Self {
        self.resumed_from_checkpoint_id = Some(checkpoint_id.into());
        self
    }

    pub fn with_output_namespace(mut self, ns: impl Into<String>) -> Self {
        self.output_namespace = Some(ns.into());
        self
    }

    pub fn with_milestones(
        mut self,
        observed: u64,
        local: Option<u64>,
        protected: Option<u64>,
    ) -> Self {
        self.latest_observed_step = Some(observed);
        self.latest_local_checkpoint_step = local;
        self.latest_protected_checkpoint_step = protected;
        if protected.is_some() {
            self.last_protected_at = Some(Utc::now());
        }
        self
    }

    pub fn update_observed_step(&mut self, step: u64) {
        self.latest_observed_step = Some(step);
    }

    pub fn commit_protected_checkpoint(
        &mut self,
        step: u64,
        checkpoint: crate::compute::TrainingCheckpointManifest,
    ) {
        self.latest_local_checkpoint_step = Some(step);
        self.latest_protected_checkpoint_step = Some(step);
        self.last_protected_at = Some(Utc::now());
        self.checkpoint = Some(checkpoint);
    }

    pub fn transition(&mut self, next: ExecutionRecordStatus) {
        self.status = next;
        if matches!(
            next,
            ExecutionRecordStatus::Completed
                | ExecutionRecordStatus::Failed
                | ExecutionRecordStatus::Cancelled
                | ExecutionRecordStatus::Preempted
        ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_record_durability_milestones() {
        let mut record = ExecutionRecord::new("recipe_finetune_llama3")
            .with_session_id("sess_001")
            .with_backend(crate::compute::ComputeBackendKind::Colab)
            .with_output_namespace("attempt_a_colab")
            .with_milestones(450, Some(440), Some(400));

        assert_eq!(record.latest_observed_step, Some(450));
        assert_eq!(record.latest_local_checkpoint_step, Some(440));
        assert_eq!(record.latest_protected_checkpoint_step, Some(400));
        assert!(record.last_protected_at.is_some());

        // Now simulate new attempt continuing from protected step 400
        let attempt_b = ExecutionRecord::new("recipe_finetune_llama3")
            .with_session_id("sess_001")
            .with_backend(crate::compute::ComputeBackendKind::RunPod)
            .with_output_namespace("attempt_b_runpod")
            .with_resume_from(&record.id)
            .with_resumed_from_checkpoint("ckpt_400")
            .with_milestones(400, Some(400), Some(400));

        assert_eq!(attempt_b.resume_from_attempt_id, Some(record.id));
        assert_eq!(attempt_b.resumed_from_checkpoint_id, Some("ckpt_400".into()));
        assert_eq!(attempt_b.output_namespace, Some("attempt_b_runpod".into()));
        assert_eq!(attempt_b.latest_observed_step, Some(400));
    }
}
