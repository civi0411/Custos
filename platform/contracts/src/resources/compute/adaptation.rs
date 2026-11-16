use serde::{Deserialize, Serialize};

use crate::DomainError;

use super::backend::AcceleratorSpec;
use super::checkpoint::TrainingCheckpointManifest;

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
        if source_batch == 0
            || source_grad_accum == 0
            || effective_batch.is_none()
            || source.memory_mb == 0
            || target.memory_mb == 0
        {
            return CompatibilityGateEvaluation {
                source_accelerator: source.name.clone(),
                target_accelerator: target.name.clone(),
                source_precision: checkpoint.precision.clone(),
                target_precision: checkpoint.precision.clone(),
                verdict: CompatibilityVerdict::Incompatible {
                    reasons: vec!["Invalid hardware memory or batch dimensions".into()],
                },
                plan: None,
                notes,
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
            warnings.push(
                "Batch adaptation requires explicit recipe approval and engine validation".into(),
            );
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
        eval.plan.ok_or_else(|| {
            DomainError::Validation("No compatible hardware adaptation proposal".into())
        })
    }
}

