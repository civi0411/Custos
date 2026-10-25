use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::new_id;
use crate::{CheckpointCommitStatus, DomainError};

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

/// Inventory declarations are not proof of remote durability or engine restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointResumeAssessment {
    pub metadata_valid: bool,
    pub state_inventory_declared: bool,
    pub can_start: bool,
    pub blockers: Vec<String>,
    pub unverified_checks: Vec<String>,
}

impl ExecutionRecord {
    pub fn assess_checkpoint_resume(&self) -> CheckpointResumeAssessment {
        let mut blockers = Vec::new();
        let metadata_valid = match self.validate_compute_state() {
            Ok(()) => true,
            Err(error) => { blockers.push(error.to_string()); false }
        };
        let inventory_valid = match &self.checkpoint {
            Some(checkpoint) => match checkpoint.validate_for_state_resume() {
                Ok(()) => true,
                Err(error) => { blockers.push(error.to_string()); false }
            },
            None => { blockers.push("No checkpoint has been recorded for this attempt".into()); false }
        };
        CheckpointResumeAssessment {
            metadata_valid,
            state_inventory_declared: metadata_valid && inventory_valid,
            can_start: false,
            blockers,
            unverified_checks: vec![
                "Checkpoint bytes and external durability have not been verified by this assessment".into(),
                "Destination hardware, environment, data access, quota and authorization require preflight".into(),
                "Training runner and engine restore validation are not connected to this read-only inspector".into(),
            ],
        }
    }

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
    ) -> Result<(), DomainError> {
        checkpoint.validate()?;
        if checkpoint.commit_status != CheckpointCommitStatus::CommittedProtected
            || step != checkpoint.global_step
            || self.latest_protected_checkpoint_step.is_some_and(|previous| step < previous)
        {
            return Err(DomainError::Validation("Protected checkpoint must be committed, match its step and not regress".into()));
        }
        self.latest_local_checkpoint_step = Some(self.latest_local_checkpoint_step.unwrap_or(0).max(step));
        self.latest_observed_step = Some(self.latest_observed_step.unwrap_or(0).max(step));
        self.latest_protected_checkpoint_step = Some(step);
        self.last_protected_at = Some(Utc::now());
        self.checkpoint = Some(checkpoint);
        Ok(())
    }

    /// Rejects contradictory persisted progress and lineage declarations.
    pub fn validate_compute_state(&self) -> Result<(), DomainError> {
        if self.id.trim().is_empty() || self.recipe_id.trim().is_empty() {
            return Err(DomainError::Validation("Execution and recipe IDs are required".into()));
        }
        if let Some(checkpoint) = &self.checkpoint { checkpoint.validate()?; }
        if let (Some(observed), Some(local)) = (self.latest_observed_step, self.latest_local_checkpoint_step) {
            if local > observed {
                return Err(DomainError::Validation("Local checkpoint progress cannot exceed observed training progress".into()));
            }
        }
        if let Some(step) = self.latest_protected_checkpoint_step {
            if self.checkpoint.as_ref().is_none_or(|checkpoint|
                checkpoint.commit_status != CheckpointCommitStatus::CommittedProtected
                    || checkpoint.global_step != step)
                || self.last_protected_at.is_none()
                || self.latest_local_checkpoint_step.is_none_or(|local| local < step)
            {
                return Err(DomainError::Validation("Protected progress requires matching committed checkpoint metadata".into()));
            }
        } else if self.last_protected_at.is_some() {
            return Err(DomainError::Validation("Protection timestamp requires a protected checkpoint step".into()));
        }
        if self.resume_from_attempt_id.is_some() != self.resumed_from_checkpoint_id.is_some()
            || self.resume_from_attempt_id.as_ref().is_some_and(|parent| parent == &self.id || parent.trim().is_empty())
            || self.resumed_from_checkpoint_id.as_ref().is_some_and(|id| id.trim().is_empty())
        {
            return Err(DomainError::Validation("Continuation requires a distinct parent attempt and checkpoint identity".into()));
        }
        Ok(())
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
    fn resume_assessment_never_grants_execution_from_metadata() {
        let record = ExecutionRecord::new("recipe");
        let result = record.assess_checkpoint_resume();
        assert!(result.metadata_valid);
        assert!(!result.state_inventory_declared);
        assert!(!result.can_start);
        assert_eq!(result.blockers.len(), 1);
        assert_eq!(result.unverified_checks.len(), 3);
        let mut manifest = crate::TrainingCheckpointManifest::new("c", 400, 1.0, "s3://bucket/c", "a".repeat(64))
            .with_full_state("model@rev", "engine@rev", "cursor400")
            .with_commit_status(CheckpointCommitStatus::CommittedProtected);
        for path in ["adapter_model.safetensors", "optimizer.pt", "scheduler.pt", "rng_state.pth", "trainer_state.json"] {
            manifest.files.push(crate::CheckpointFileEntry { path: path.into(), size_bytes: 1,
                digest: "a".repeat(64), required_for_state_resume: true });
        }
        let declared = record.clone().with_checkpoint(manifest).assess_checkpoint_resume();
        assert!(declared.state_inventory_declared);
        assert!(declared.blockers.is_empty());
        assert!(!declared.can_start, "Metadata must never authorize training");
        assert!(!declared.unverified_checks.is_empty());
        let inconsistent = record.with_resume_from("parent");
        assert!(!inconsistent.assess_checkpoint_resume().metadata_valid);
    }

    #[test]
    fn checkpoint_milestones_cannot_fabricate_protection_or_mutate_on_failure() {
        let mut record = ExecutionRecord::new("recipe");
        let checkpoint = crate::TrainingCheckpointManifest::new("c", 400, 1.0, "s3://b/c", "a".repeat(64));
        assert!(record.commit_protected_checkpoint(400, checkpoint).is_err());
        assert!(record.checkpoint.is_none());
        assert!(record.latest_protected_checkpoint_step.is_none());
        let forged = record.clone().with_milestones(450, Some(440), Some(400));
        assert!(forged.validate_compute_state().is_err());
        let orphan = record.with_resume_from("parent");
        assert!(orphan.validate_compute_state().is_err());
    }

    #[test]
    fn test_execution_record_durability_milestones() {
        let record = ExecutionRecord::new("recipe_finetune_llama3")
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
