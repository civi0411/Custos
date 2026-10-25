//! Sovereign Compute Tier & Checkpoint Relay Models (RFC 005)
//!
//! Provides strongly-typed domain definitions for heterogeneous compute backends,
//! accelerator specifications, safe checkpoint manifests, cloud pod leases,
//! and hardware adaptation planning for cross-platform failover.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeBackendKind {
    Local,
    Colab,
    Kaggle,
    DedicatedSsh,
    RunPod,
    VastAi,
    LambdaCloud,
}

impl std::fmt::Display for ComputeBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Colab => write!(f, "colab"),
            Self::Kaggle => write!(f, "kaggle"),
            Self::DedicatedSsh => write!(f, "dedicated_ssh"),
            Self::RunPod => write!(f, "runpod"),
            Self::VastAi => write!(f, "vast_ai"),
            Self::LambdaCloud => write!(f, "lambda_cloud"),
        }
    }
}

/// Hardware accelerator profile observed or requested
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceleratorSpec {
    pub name: String,
    pub memory_mb: u64,
    pub compute_capability: Option<String>,
    pub supports_bfloat16: bool,
    pub driver_version: Option<String>,
    pub cuda_version: Option<String>,
}

impl AcceleratorSpec {
    pub fn new(name: impl Into<String>, memory_mb: u64, supports_bfloat16: bool) -> Self {
        Self {
            name: name.into(),
            memory_mb,
            compute_capability: None,
            supports_bfloat16,
            driver_version: None,
            cuda_version: None,
        }
    }

    pub fn rtx_4090() -> Self {
        Self {
            name: "NVIDIA GeForce RTX 4090".into(),
            memory_mb: 24576,
            compute_capability: Some("8.9".into()),
            supports_bfloat16: true,
            driver_version: None,
            cuda_version: Some("12.2".into()),
        }
    }

    pub fn a100_40gb() -> Self {
        Self {
            name: "NVIDIA A100-SXM4-40GB".into(),
            memory_mb: 40960,
            compute_capability: Some("8.0".into()),
            supports_bfloat16: true,
            driver_version: None,
            cuda_version: Some("12.2".into()),
        }
    }

    pub fn tesla_t4() -> Self {
        Self {
            name: "Tesla T4".into(),
            memory_mb: 15360,
            compute_capability: Some("7.5".into()),
            supports_bfloat16: false,
            driver_version: None,
            cuda_version: Some("12.0".into()),
        }
    }

    pub fn tesla_p100() -> Self {
        Self {
            name: "Tesla P100-PCIE-16GB".into(),
            memory_mb: 16280,
            compute_capability: Some("6.0".into()),
            supports_bfloat16: false,
            driver_version: None,
            cuda_version: Some("12.0".into()),
        }
    }
}

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
            || self.storage_uri.split("://").nth(1)
                .and_then(|rest| rest.split('/').next()).is_some_and(|authority| authority.contains('@'))
            || !self.epoch.is_finite() || self.epoch < 0.0
            || self.loss.is_some_and(|v| !v.is_finite())
            || self.learning_rate.is_some_and(|v| !v.is_finite() || v < 0.0)
            || !matches!(self.precision.as_str(), "fp16" | "bf16" | "fp32" | "fp64")
        {
            return Err(invalid("Invalid checkpoint identity, metrics or precision"));
        }
        let digest_valid = |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        let mut paths = std::collections::HashSet::new();
        for file in &self.files {
            if file.path.is_empty() || file.path.contains(['\\', ':'])
                || file.path.split('/').any(|part| part.is_empty() || part == "." || part == "..")
                || !paths.insert(&file.path) || file.size_bytes == 0 || !digest_valid(&file.digest)
            {
                return Err(invalid("Checkpoint contains invalid, duplicate or unsafe file entries"));
            }
        }
        if self.commit_status == CheckpointCommitStatus::CommittedProtected
            && (self.files.is_empty() || !digest_valid(&self.storage_digest))
        {
            return Err(invalid("Protected checkpoint requires a file inventory and manifest digest"));
        }
        if self.resumability_level.allows_state_continuation()
            && (!self.optimizer_state_preserved || !self.scheduler_state_preserved
                || !self.rng_state_preserved
                || [&self.base_model_revision, &self.engine_version, &self.dataset_cursor]
                    .iter().any(|v| v.as_ref().is_none_or(|s| s.trim().is_empty()))
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
        let required = |name: &str| self.files.iter().any(|file| file.path == name && file.required_for_state_resume);
        if !(required("model.safetensors") || required("adapter_model.safetensors") || required("pytorch_model.bin"))
            || !["optimizer.pt", "scheduler.pt", "rng_state.pth", "trainer_state.json"].iter().all(|name| required(name))
        {
            return Err(DomainError::Validation("State resume currently requires an inventoried single-GPU Trainer model/optimizer/scheduler/RNG/trainer-state bundle".into()));
        }
        Ok(())
    }
}

/// Verdict from the hardware compatibility and safety gate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityVerdict {
    /// Fully compatible: exact precision and equivalent memory capacity
    Compatible,
    /// Compatible with adaptation: microbatch or precision adjustment needed; warnings must be acknowledged
    RequiresAdaptation { warnings: Vec<String> },
    /// Incompatible: target cannot fit model, lacks hardware feature without approved fallback, etc.
    Incompatible { reasons: Vec<String> },
}

/// Evaluated gate result determining whether cross-platform failover is valid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityGateEvaluation {
    pub source_accelerator: String,
    pub target_accelerator: String,
    pub source_precision: String,
    pub target_precision: String,
    pub verdict: CompatibilityVerdict,
    pub plan: Option<HardwareAdaptationPlan>,
    pub notes: Vec<String>,
}

/// Adaptive training parameters computed during cross-hardware failover
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareAdaptationPlan {
    pub source_accelerator: String,
    pub target_accelerator: String,
    pub source_precision: String,
    pub target_precision: String,
    pub precision_downgraded: bool,
    pub per_device_batch_size: u32,
    pub gradient_accumulation_steps: u32,
    pub effective_batch_size: u32,
    pub resume_checkpoint_uri: String,
    pub resume_step: u64,
    pub adaptation_warnings: Vec<String>,
}

impl HardwareAdaptationPlan {
    /// Evaluates hardware compatibility gate without naive auto-downgrade.
    /// Explicitly surfaces precision mismatch, VRAM constraints, and scaling implications.
    pub fn evaluate_gate(
        source: &AcceleratorSpec,
        target: &AcceleratorSpec,
        source_batch: u32,
        source_grad_accum: u32,
        checkpoint: &TrainingCheckpointManifest,
    ) -> CompatibilityGateEvaluation {
        let mut warnings = Vec::new();
        let mut reasons = Vec::new();
        let mut notes = Vec::new();

        let effective_batch = source_batch.checked_mul(source_grad_accum);
        if source_batch == 0 || source_grad_accum == 0 || effective_batch.is_none()
            || source.memory_mb == 0 || target.memory_mb == 0
        {
            return CompatibilityGateEvaluation {
                source_accelerator: source.name.clone(), target_accelerator: target.name.clone(),
                source_precision: checkpoint.precision.clone(), target_precision: checkpoint.precision.clone(),
                verdict: CompatibilityVerdict::Incompatible { reasons: vec!["Invalid hardware memory or batch dimensions".into()] },
                plan: None, notes,
            };
        }

        // 1. VRAM validation
        if target.memory_mb < 4096 {
            reasons.push(format!(
                "Target accelerator memory ({} MB) is insufficient for LLM training",
                target.memory_mb
            ));
        }

        // 2. Precision & Architecture Gate
        let source_precision = checkpoint.precision.clone();
        let mut target_precision = source_precision.clone();
        let mut precision_downgraded = false;

        if source_precision == "bf16" && !target.supports_bfloat16 {
            target_precision = "fp16".to_string();
            precision_downgraded = true;
            warnings.push(format!(
                "Target accelerator '{}' does not support native bf16. Downgrading to fp16 may cause numerical divergence or overflow; trajectory equivalence is not guaranteed.",
                target.name
            ));
        }

        // 3. Batch size scaling
        let effective_batch = effective_batch.unwrap_or(0);
        let target_batch = if target.memory_mb >= source.memory_mb {
            source_batch
        } else {
            let ratio = target.memory_mb as f64 / source.memory_mb as f64;
            let scaled = ((source_batch as f64) * ratio).floor() as u32;
            scaled.max(1)
        };

        if target_batch != source_batch {
            notes.push(format!(
                "Scaled per-device batch from {} to {} based on memory ratio",
                source_batch, target_batch
            ));
        }

        let target_grad_accum = if target_batch == 0 {
            1
        } else {
            ((effective_batch as f64) / (target_batch as f64)).ceil() as u32
        };

        let resulting_effective_batch = match target_batch.checked_mul(target_grad_accum) {
            Some(value) => value,
            None => {
                reasons.push("Adapted effective batch size exceeds supported range".into());
                0
            }
        };
        if target_batch != source_batch {
            warnings.push("Batch adaptation requires explicit recipe approval and engine validation".into());
        }
        if resulting_effective_batch != effective_batch {
            warnings.push(format!(
                "Effective batch size changed from {} to {} due to accumulation rounding",
                effective_batch, resulting_effective_batch
            ));
        }

        // 4. Checkpoint Resumability Check
        if !checkpoint.resumability_level.allows_state_continuation() {
            warnings.push(format!(
                "Checkpoint {} has level '{:?}': state continuation will reset optimizer/scheduler.",
                checkpoint.checkpoint_id, checkpoint.resumability_level
            ));
        }

        let verdict = if !reasons.is_empty() {
            CompatibilityVerdict::Incompatible { reasons }
        } else if !warnings.is_empty() {
            CompatibilityVerdict::RequiresAdaptation {
                warnings: warnings.clone(),
            }
        } else {
            CompatibilityVerdict::Compatible
        };

        let plan = if !matches!(verdict, CompatibilityVerdict::Incompatible { .. }) {
            Some(HardwareAdaptationPlan {
                source_accelerator: source.name.clone(),
                target_accelerator: target.name.clone(),
                source_precision: source_precision.clone(),
                target_precision: target_precision.clone(),
                precision_downgraded,
                per_device_batch_size: target_batch,
                gradient_accumulation_steps: target_grad_accum,
                effective_batch_size: resulting_effective_batch,
                resume_checkpoint_uri: checkpoint.storage_uri.clone(),
                resume_step: checkpoint.global_step,
                adaptation_warnings: warnings,
            })
        } else {
            None
        };

        CompatibilityGateEvaluation {
            source_accelerator: source.name.clone(),
            target_accelerator: target.name.clone(),
            source_precision,
            target_precision,
            verdict,
            plan,
            notes,
        }
    }

    /// Returns a proposal without fabricating a fallback for rejected hardware.
    pub fn compute(
        source: &AcceleratorSpec,
        target: &AcceleratorSpec,
        source_batch: u32,
        source_grad_accum: u32,
        checkpoint: &TrainingCheckpointManifest,
    ) -> Result<Self, DomainError> {
        let eval = Self::evaluate_gate(source, target, source_batch, source_grad_accum, checkpoint);
        eval.plan.ok_or_else(|| DomainError::Validation("No compatible hardware adaptation proposal".into()))
    }
}

/// Paid on-demand cloud GPU pod lease record (RunPod / Vast.ai)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudPodLease {
    pub lease_id: String,
    pub provider: ComputeBackendKind,
    pub external_pod_id: String,
    pub host_address: Option<String>,
    pub ssh_port: Option<u16>,
    pub cost_per_hour_usd: f32,
    pub max_budget_usd: Option<f32>,
    pub auto_stop_minutes: u32,
    pub started_at: DateTime<Utc>,
    pub status: CloudPodStatus,
}

impl CloudPodLease {
    pub fn new(
        lease_id: impl Into<String>,
        provider: ComputeBackendKind,
        external_pod_id: impl Into<String>,
        cost_per_hour: f32,
    ) -> Self {
        Self {
            lease_id: lease_id.into(),
            provider,
            external_pod_id: external_pod_id.into(),
            host_address: None,
            ssh_port: None,
            cost_per_hour_usd: cost_per_hour,
            max_budget_usd: None,
            auto_stop_minutes: 30,
            started_at: Utc::now(),
            status: CloudPodStatus::Provisioning,
        }
    }

    pub fn with_budget(mut self, max_budget_usd: f32) -> Self {
        self.max_budget_usd = Some(max_budget_usd);
        self
    }

    pub fn with_auto_stop(mut self, minutes: u32) -> Self {
        self.auto_stop_minutes = minutes;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudPodStatus {
    Provisioning,
    Running,
    Computing,
    Stopping,
    Stopped,
    Terminated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_admission_is_fail_closed() {
        let mut manifest = TrainingCheckpointManifest::new("c", 400, 1.0, "s3://bucket/c", "a".repeat(64));
        assert_eq!(manifest.commit_status, CheckpointCommitStatus::Observed);
        assert!(manifest.validate_for_state_resume().is_err());
        manifest = manifest.with_full_state("model@rev", "unsloth@rev", "cursor400")
            .with_commit_status(CheckpointCommitStatus::CommittedProtected);
        for path in ["adapter_model.safetensors", "optimizer.pt", "scheduler.pt", "rng_state.pth", "trainer_state.json"] {
            manifest.files.push(CheckpointFileEntry { path: path.into(), size_bytes: 1,
                digest: "a".repeat(64), required_for_state_resume: true });
        }
        assert!(manifest.validate_for_state_resume().is_ok());
        let mut secret_uri = manifest.clone();
        secret_uri.storage_uri = "https://store/checkpoint?token=secret".into();
        assert!(secret_uri.validate().is_err());
        manifest.rng_state_preserved = false;
        assert!(manifest.validate_for_state_resume().is_err());
        manifest.rng_state_preserved = true;
        manifest.files.retain(|file| file.path != "optimizer.pt");
        assert!(manifest.validate_for_state_resume().is_err());
        manifest.files[0].path = "../weights".into();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn invalid_hardware_dimensions_do_not_panic_or_fabricate_a_plan() {
        let gpu = AcceleratorSpec::tesla_t4();
        let checkpoint = TrainingCheckpointManifest::new("c", 0, 0.0, "s3://b/c", "");
        for (batch, accum) in [(0, 1), (1, 0), (u32::MAX, 2)] {
            assert!(HardwareAdaptationPlan::evaluate_gate(&gpu, &gpu, batch, accum, &checkpoint).plan.is_none());
        }
        let tiny = AcceleratorSpec::new("tiny", 1, false);
        assert!(HardwareAdaptationPlan::compute(&gpu, &tiny, 1, 1, &checkpoint).is_err());
    }

    #[test]
    fn test_hardware_adaptation_a100_to_t4() {
        let source_a100 = AcceleratorSpec::a100_40gb();
        let target_t4 = AcceleratorSpec::tesla_t4();

        let checkpoint = TrainingCheckpointManifest::new(
            "ckpt_001",
            450,
            1.5,
            "hf://user/repo/checkpoint-450",
            "sha256_mock_hash",
        )
        .with_precision("bf16");

        // Evaluate via compatibility gate
        let eval = HardwareAdaptationPlan::evaluate_gate(
            &source_a100,
            &target_t4,
            8,
            2,
            &checkpoint,
        );

        // Target lacks bf16 -> Requires adaptation with warning
        assert!(matches!(eval.verdict, CompatibilityVerdict::RequiresAdaptation { .. }));
        assert_eq!(eval.target_precision, "fp16");

        let plan = eval.plan.expect("plan should be available");
        assert!(plan.precision_downgraded);
        assert_eq!(plan.target_precision, "fp16");
        // VRAM drops from 40GB to 15GB (ratio 0.375) -> batch drops from 8 to 3
        assert_eq!(plan.per_device_batch_size, 3);
        // Grad accum increases to preserve effective batch size (ceil(16/3) = 6)
        assert_eq!(plan.gradient_accumulation_steps, 6);
        assert_eq!(plan.resume_step, 450);
        assert_eq!(plan.resume_checkpoint_uri, "hf://user/repo/checkpoint-450");
    }

    #[test]
    fn test_hardware_adaptation_t4_to_4090() {
        let source_t4 = AcceleratorSpec::tesla_t4();
        let target_4090 = AcceleratorSpec::rtx_4090();

        let checkpoint = TrainingCheckpointManifest::new(
            "ckpt_002",
            100,
            0.5,
            "s3://bucket/checkpoints/step-100",
            "sha256_hash",
        )
        .with_full_state("meta-llama/Llama-3-8B", "unsloth-2024.8", "train:100/1000");

        let eval = HardwareAdaptationPlan::evaluate_gate(
            &source_t4,
            &target_4090,
            4,
            4,
            &checkpoint,
        );

        // Fully compatible
        assert_eq!(eval.verdict, CompatibilityVerdict::Compatible);

        let plan = eval.plan.expect("plan must exist");
        assert!(!plan.precision_downgraded);
        assert_eq!(plan.per_device_batch_size, 4);
        assert_eq!(plan.gradient_accumulation_steps, 4);
        assert_eq!(plan.effective_batch_size, 16);
    }

    #[test]
    fn test_compatibility_gate_insufficient_vram() {
        let source = AcceleratorSpec::rtx_4090();
        let tiny_gpu = AcceleratorSpec::new("Tiny Embedded GPU", 2048, false);

        let checkpoint = TrainingCheckpointManifest::new(
            "ckpt_tiny",
            50,
            0.1,
            "s3://bucket/test",
            "hash",
        );

        let eval = HardwareAdaptationPlan::evaluate_gate(&source, &tiny_gpu, 2, 2, &checkpoint);
        assert!(matches!(eval.verdict, CompatibilityVerdict::Incompatible { .. }));
        assert!(eval.plan.is_none());
    }

    #[test]
    fn test_checkpoint_manifest_resumability_and_commit_stages() {
        let mut manifest = TrainingCheckpointManifest::new(
            "ckpt_durable",
            400,
            2.0,
            "r2://bucket/models/attempt-a/ckpt_400",
            "blake3_hash",
        )
        .with_commit_status(CheckpointCommitStatus::LocalStaged)
        .with_resumability(ResumabilityLevel::WarmStart);

        assert_eq!(manifest.commit_status, CheckpointCommitStatus::LocalStaged);
        assert_eq!(manifest.resumability_level, ResumabilityLevel::WarmStart);
        assert!(!manifest.resumability_level.allows_state_continuation());

        // Promote to full state and committed
        manifest = manifest
            .with_full_state("base-model-v1", "torch-2.4", "batch_400")
            .with_commit_status(CheckpointCommitStatus::CommittedProtected)
            .add_file(CheckpointFileEntry {
                path: "optimizer.pt".into(),
                size_bytes: 1024 * 1024 * 50,
                digest: "sha256_opt".into(),
                required_for_state_resume: true,
            });

        assert_eq!(manifest.resumability_level, ResumabilityLevel::TrainingStateResume);
        assert!(manifest.resumability_level.allows_state_continuation());
        assert_eq!(manifest.commit_status, CheckpointCommitStatus::CommittedProtected);
        assert_eq!(manifest.files.len(), 1);
        assert!(manifest.optimizer_state_preserved);
    }
}
