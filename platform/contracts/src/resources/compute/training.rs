//! Supervised Training and Evaluation Pure Domain Models (GPU-C / GPU-D)
//!
//! Zero I/O. Defines frozen reproducible training specs, step telemetry,
//! execution receipts, and model evaluation reports.

use crate::evidence::experiment_attempt::ExecState;
use crate::ids::new_id;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Frozen, immutable training launch specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenTrainingRecipe {
    pub recipe_id: String,
    pub model_id: String,
    pub dataset_uri: String,
    pub split: String,
    pub max_steps: u64,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub target_device_id: Option<String>,
    pub checkpoint_interval_steps: u64,
    pub seed: u64,
}

impl FrozenTrainingRecipe {
    pub fn new(
        model_id: impl Into<String>,
        dataset_uri: impl Into<String>,
        max_steps: u64,
    ) -> Self {
        let max_steps = max_steps.max(1);
        Self {
            recipe_id: new_id("recipe"),
            model_id: model_id.into(),
            dataset_uri: dataset_uri.into(),
            split: "train".into(),
            max_steps,
            batch_size: 4,
            learning_rate: 2e-4,
            target_device_id: None,
            checkpoint_interval_steps: (max_steps / 2).max(1),
            seed: 42,
        }
    }

    pub fn with_device(mut self, device_id: impl Into<String>) -> Self {
        self.target_device_id = Some(device_id.into());
        self
    }

    pub fn with_batch_size(mut self, batch_size: u32) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_learning_rate(mut self, lr: f64) -> Self {
        self.learning_rate = lr;
        self
    }

    pub fn with_checkpoint_interval(mut self, interval: u64) -> Self {
        self.checkpoint_interval_steps = interval.max(1);
        self
    }

    /// Computes deterministic content digest (SHA-256) of the frozen recipe
    pub fn content_digest(&self) -> String {
        let serialized = serde_json::to_string(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        let digest = hasher.finalize();
        format!(
            "sha256:{}",
            digest.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    }
}

/// Structured per-step or interval training telemetry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingStepTelemetry {
    pub step: u64,
    pub epoch: f64,
    pub loss: f64,
    pub learning_rate: f64,
    pub elapsed_ms: u64,
    pub gpu_memory_used_mb: Option<u64>,
}

/// Immutable terminal receipt of a completed or stopped training run
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingExecutionReceipt {
    pub attempt_id: String,
    pub recipe_digest: String,
    pub device_id: String,
    pub lease_id: String,
    pub total_steps_completed: u64,
    pub final_loss: f64,
    pub checkpoints_created: Vec<String>,
    pub status: ExecState,
}

/// Specification for evaluating a candidate checkpoint on a held-out dataset
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEvaluationSpec {
    pub eval_id: String,
    pub candidate_checkpoint_id: String,
    pub dataset_uri: String,
    pub rubric_version: String,
    pub target_device_id: Option<String>,
    pub sample_limit: usize,
}

impl ModelEvaluationSpec {
    pub fn new(candidate_checkpoint_id: impl Into<String>, dataset_uri: impl Into<String>) -> Self {
        Self {
            eval_id: new_id("eval"),
            candidate_checkpoint_id: candidate_checkpoint_id.into(),
            dataset_uri: dataset_uri.into(),
            rubric_version: "v1.0.0".into(),
            target_device_id: None,
            sample_limit: 100,
        }
    }

    pub fn with_device(mut self, device_id: impl Into<String>) -> Self {
        self.target_device_id = Some(device_id.into());
        self
    }

    pub fn with_sample_limit(mut self, limit: usize) -> Self {
        self.sample_limit = limit;
        self
    }
}

/// Scored evaluation report on held-out data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelEvaluationReport {
    pub eval_id: String,
    pub candidate_checkpoint_id: String,
    pub device_id: String,
    pub sample_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub accuracy: f64,
    pub average_loss: f64,
    pub latency_ms: u64,
    pub passed_hurdle: bool,
    pub artifact_refs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frozen_recipe_digest_determinism() {
        let r1 = FrozenTrainingRecipe::new("meta-llama/Llama-3-8B", "s3://datasets/alpaca", 100)
            .with_device("dev_cuda_0");
        let r2 = r1.clone();
        assert_eq!(r1.content_digest(), r2.content_digest());
        assert!(r1.content_digest().starts_with("sha256:"));
    }

    #[test]
    fn test_evaluation_spec_defaults() {
        let spec = ModelEvaluationSpec::new("ckpt_step_100", "s3://datasets/eval_set")
            .with_sample_limit(50);
        assert_eq!(spec.candidate_checkpoint_id, "ckpt_step_100");
        assert_eq!(spec.sample_limit, 50);
        assert_eq!(spec.rubric_version, "v1.0.0");
    }
}

