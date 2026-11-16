use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::DomainError;

/// Four explicit tiers of checkpoint resumability and reproducibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumabilityLevel {
    /// Weights/adapters/config only — suitable for inference, evaluation or starting a fresh training run
    ExportOnly,
    /// Weights loaded, but optimizer, scheduler, data cursor, or RNG states are missing or reset
    WarmStart,
    /// Full training state restored (model weights, optimizer, scheduler, engine state, data cursor, RNG)
    TrainingStateResume,
    /// Verified parity against an uninterrupted baseline within a specific hardware and numerical tolerance
    ValidatedResume,
}

impl ResumabilityLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ExportOnly => "Export only",
            Self::WarmStart => "Warm start",
            Self::TrainingStateResume => "Training-state resume",
            Self::ValidatedResume => "Validated parity",
        }
    }

    pub fn allows_state_continuation(&self) -> bool {
        matches!(self, Self::TrainingStateResume | Self::ValidatedResume)
    }
}

/// Physical durability stages of a checkpoint during the commit protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointCommitStatus {
    /// Step observed in logs / live telemetry, not yet saved to local disk
    Observed,
    /// Flushed safely to local compute node storage at consistent boundary
    LocalStaged,
    /// In the process of being uploaded to external durable storage
    Uploading,
    /// Fully uploaded, completeness/hash verified, committed externally
    CommittedProtected,
    /// Verification failed or corrupted files detected
    Corrupted,
}

/// Entry for an individual artifact file within a portable checkpoint
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointFileEntry {
    pub path: String,
    pub size_bytes: u64,
    pub digest: String,
    pub required_for_state_resume: bool,
}

/// Verified checkpoint manifest for stateful relay across heterogeneous hardware
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCheckpointManifest {
    pub checkpoint_id: String,
    pub global_step: u64,
    pub epoch: f64,
    pub loss: Option<f32>,
    pub learning_rate: Option<f64>,
    pub precision: String,
    pub storage_uri: String,
    pub storage_digest: String,
    pub adapter_only: bool,
    pub resumability_level: ResumabilityLevel,
    pub commit_status: CheckpointCommitStatus,
    pub files: Vec<CheckpointFileEntry>,
    pub base_model_revision: Option<String>,
    pub dataset_cursor: Option<String>,
    pub engine_version: Option<String>,
    pub rng_state_preserved: bool,
    pub optimizer_state_preserved: bool,
    pub scheduler_state_preserved: bool,
    pub created_at: DateTime<Utc>,
}

impl TrainingCheckpointManifest {
    pub fn new(
        checkpoint_id: impl Into<String>,
        global_step: u64,
        epoch: f64,
        storage_uri: impl Into<String>,
        storage_digest: impl Into<String>,
    ) -> Self {
        Self {
            checkpoint_id: checkpoint_id.into(),
            global_step,
            epoch,
            loss: None,
            learning_rate: None,
            precision: "fp16".into(),
            storage_uri: storage_uri.into(),
            storage_digest: storage_digest.into(),
            adapter_only: true,
            resumability_level: ResumabilityLevel::ExportOnly,
            commit_status: CheckpointCommitStatus::Observed,
            files: Vec::new(),
            base_model_revision: None,
            dataset_cursor: None,
            engine_version: None,
            rng_state_preserved: false,
            optimizer_state_preserved: false,
            scheduler_state_preserved: false,
            created_at: Utc::now(),
        }
    }

    pub fn with_loss(mut self, loss: f32) -> Self {
        self.loss = Some(loss);
        self
    }

    pub fn with_precision(mut self, prec: impl Into<String>) -> Self {
        self.precision = prec.into();
        self
    }

    pub fn with_resumability(mut self, level: ResumabilityLevel) -> Self {
        self.resumability_level = level;
        self
    }

    pub fn with_commit_status(mut self, status: CheckpointCommitStatus) -> Self {
        self.commit_status = status;
        self
    }

    pub fn with_full_state(
        mut self,
        base_model_revision: impl Into<String>,
        engine_version: impl Into<String>,
        dataset_cursor: impl Into<String>,
    ) -> Self {
        self.adapter_only = false;
        self.resumability_level = ResumabilityLevel::TrainingStateResume;
        self.base_model_revision = Some(base_model_revision.into());
        self.engine_version = Some(engine_version.into());
        self.dataset_cursor = Some(dataset_cursor.into());
        self.optimizer_state_preserved = true;
        self.scheduler_state_preserved = true;
        self.rng_state_preserved = true;
        self
    }

    pub fn add_file(mut self, file: CheckpointFileEntry) -> Self {
        self.files.push(file);
        self
    }

    /// Validates declared metadata, not the integrity or provenance of remote bytes.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = |message: &str| DomainError::Validation(message.into());
        if self.checkpoint_id.trim().is_empty()
            || self.storage_uri.trim().is_empty()
            || self.storage_uri.contains(['?', '#'])
            || self.storage_uri.chars().any(char::is_whitespace)
            || self
                .storage_uri
                .split("://")
                .nth(1)
                .and_then(|rest| rest.split('/').next())
                .is_some_and(|authority| authority.contains('@'))
            || !self.epoch.is_finite()
            || self.epoch < 0.0
            || self.loss.is_some_and(|v| !v.is_finite())
            || self
                .learning_rate
                .is_some_and(|v| !v.is_finite() || v < 0.0)
            || !matches!(self.precision.as_str(), "fp16" | "bf16" | "fp32" | "fp64")
        {
            return Err(invalid("Invalid checkpoint identity, metrics or precision"));
        }
        let digest_valid =
            |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        let mut paths = std::collections::HashSet::new();
        for file in &self.files {
            if file.path.is_empty()
                || file.path.contains(['\\', ':'])
                || file
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
                || !paths.insert(&file.path)
                || file.size_bytes == 0
                || !digest_valid(&file.digest)
            {
                return Err(invalid(
                    "Checkpoint contains invalid, duplicate or unsafe file entries",
                ));
            }
        }
        if self.commit_status == CheckpointCommitStatus::CommittedProtected
            && (self.files.is_empty() || !digest_valid(&self.storage_digest))
        {
            return Err(invalid(
                "Protected checkpoint requires a file inventory and manifest digest",
            ));
        }
        if self.resumability_level.allows_state_continuation()
            && (!self.optimizer_state_preserved
                || !self.scheduler_state_preserved
                || !self.rng_state_preserved
                || [
                    &self.base_model_revision,
                    &self.engine_version,
                    &self.dataset_cursor,
                ]
                .iter()
                .any(|v| v.as_ref().is_none_or(|s| s.trim().is_empty()))
                || !self.files.iter().any(|f| f.required_for_state_resume))
        {
            return Err(invalid("Full-state resume requires engine/model/data references and inventoried training state"));
        }
        Ok(())
    }

    /// Admission precondition; an engine must still validate and load the bundle.
    pub fn validate_for_state_resume(&self) -> Result<(), DomainError> {
        self.validate()?;
        if self.commit_status != CheckpointCommitStatus::CommittedProtected
            || !self.resumability_level.allows_state_continuation()
        {
            return Err(DomainError::Validation("State resume requires a protected full-state checkpoint; warm start is a separate operation".into()));
        }
        let required = |name: &str| {
            self.files
                .iter()
                .any(|file| file.path == name && file.required_for_state_resume)
        };
        if !(required("model.safetensors")
            || required("adapter_model.safetensors")
            || required("pytorch_model.bin"))
            || ![
                "optimizer.pt",
                "scheduler.pt",
                "rng_state.pth",
                "trainer_state.json",
            ]
            .iter()
            .all(|name| required(name))
        {
            return Err(DomainError::Validation("State resume currently requires an inventoried single-GPU Trainer model/optimizer/scheduler/RNG/trainer-state bundle".into()));
        }
        Ok(())
    }
}

