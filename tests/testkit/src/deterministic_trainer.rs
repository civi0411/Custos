//! Deterministic CPU Fixture Trainer (F10 / P2).
//!
//! Provides a bit-exact deterministic linear regression trainer on CPU for
//! verifying checkpoint save/load/relay, data cursor continuity, optimizer state
//! preservation (including empty state for SGD without momentum), and canary evaluation.
//!
//! Zero external dependencies. Zero GPU requirement. Zero cloud credentials.

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use custos_domain::{
    digest, CheckpointCommitStatus, CheckpointFileEntry, DomainError,
    TrainingCheckpointManifest,
};

/// Deterministic Linear Congruential Generator (LCG)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicPrng {
    pub state: u64,
}

impl DeterministicPrng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    /// Returns a float in [-1.0, 1.0)
    pub fn next_f64(&mut self) -> f64 {
        let val = (self.next_u64() >> 11) as f64;
        (val / (1u64 << 53) as f64) * 2.0 - 1.0
    }
}

/// Data cursor tracking precise dataset positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataCursor {
    pub epoch: u64,
    pub index: u64,
    pub dataset_len: u64,
    pub shuffle_seed: u64,
}

impl DataCursor {
    pub fn new(dataset_len: u64, shuffle_seed: u64) -> Self {
        Self {
            epoch: 0,
            index: 0,
            dataset_len,
            shuffle_seed,
        }
    }

    pub fn advance(&mut self) {
        self.index += 1;
        if self.index >= self.dataset_len {
            self.index = 0;
            self.epoch += 1;
        }
    }
}

/// Supported optimizer configurations for deterministic training
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizerKind {
    /// SGD with no momentum: optimizer state contains zero tensor parameters (D5)
    SgdNoMomentum { lr: f64 },
    /// SGD with momentum: stores velocity
    SgdWithMomentum { lr: f64, momentum: f64 },
    /// Adam: stores first and second moments and time step
    Adam { lr: f64, beta1: f64, beta2: f64, eps: f64 },
}

/// Persisted optimizer state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizerState {
    pub kind: OptimizerKind,
    pub v_w: Vec<f64>,
    pub v_b: f64,
    pub m_w: Vec<f64>,
    pub m_b: f64,
    pub step: u64,
}

impl OptimizerState {
    pub fn new(kind: OptimizerKind, num_features: usize) -> Self {
        Self {
            kind,
            v_w: vec![0.0; num_features],
            v_b: 0.0,
            m_w: vec![0.0; num_features],
            m_b: 0.0,
            step: 0,
        }
    }

    pub fn is_empty_tensor_state(&self) -> bool {
        matches!(self.kind, OptimizerKind::SgdNoMomentum { .. })
    }
}

/// Model parameters: y = w[0]*x[0] + w[1]*x[1] + b
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelWeights {
    pub w: Vec<f64>,
    pub b: f64,
}

/// Trainer state metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainerStateMetadata {
    pub global_step: u64,
    pub epoch: f64,
    pub cursor: DataCursor,
    pub last_loss: f64,
}

/// Complete deterministic linear regression trainer
#[derive(Debug, Clone, PartialEq)]
pub struct DeterministicTrainer {
    pub weights: ModelWeights,
    pub optimizer_state: OptimizerState,
    pub cursor: DataCursor,
    pub prng: DeterministicPrng,
    pub global_step: u64,
    pub last_loss: f64,
    pub true_w: Vec<f64>,
    pub true_b: f64,
}

impl DeterministicTrainer {
    pub fn new(optimizer: OptimizerKind, seed: u64, dataset_len: u64) -> Self {
        let num_features = 2;
        Self {
            weights: ModelWeights {
                w: vec![0.0; num_features],
                b: 0.0,
            },
            optimizer_state: OptimizerState::new(optimizer, num_features),
            cursor: DataCursor::new(dataset_len, seed),
            prng: DeterministicPrng::new(seed),
            global_step: 0,
            last_loss: 0.0,
            true_w: vec![2.5, -1.5],
            true_b: 0.5,
        }
    }

    /// Generates sample (x, y) deterministically based on cursor and seed
    pub fn get_sample(&self, cursor: &DataCursor) -> (Vec<f64>, f64) {
        let sample_seed = cursor
            .shuffle_seed
            .wrapping_add(cursor.epoch.wrapping_mul(1000003))
            .wrapping_add(cursor.index.wrapping_mul(999983));
        let mut sample_prng = DeterministicPrng::new(sample_seed);
        let x0 = sample_prng.next_f64();
        let x1 = sample_prng.next_f64();
        let noise = sample_prng.next_f64() * 0.01;
        let y = self.true_w[0] * x0 + self.true_w[1] * x1 + self.true_b + noise;
        (vec![x0, x1], y)
    }

    /// Performs one deterministic training step
    pub fn step(&mut self) -> f64 {
        let (x, y) = self.get_sample(&self.cursor);
        let y_pred = self.weights.w[0] * x[0] + self.weights.w[1] * x[1] + self.weights.b;
        let diff = y_pred - y;
        let loss = 0.5 * diff * diff;
        self.last_loss = loss;

        // Gradients
        let grad_w = vec![diff * x[0], diff * x[1]];
        let grad_b = diff;

        // Optimizer update
        match &mut self.optimizer_state.kind {
            OptimizerKind::SgdNoMomentum { lr } => {
                for i in 0..2 {
                    self.weights.w[i] -= *lr * grad_w[i];
                }
                self.weights.b -= *lr * grad_b;
            }
            OptimizerKind::SgdWithMomentum { lr, momentum } => {
                for i in 0..2 {
                    self.optimizer_state.v_w[i] =
                        *momentum * self.optimizer_state.v_w[i] + *lr * grad_w[i];
                    self.weights.w[i] -= self.optimizer_state.v_w[i];
                }
                self.optimizer_state.v_b =
                    *momentum * self.optimizer_state.v_b + *lr * grad_b;
                self.weights.b -= self.optimizer_state.v_b;
            }
            OptimizerKind::Adam {
                lr,
                beta1,
                beta2,
                eps,
            } => {
                let (lr, beta1, beta2, eps) = (*lr, *beta1, *beta2, *eps);
                self.optimizer_state.step += 1;
                let t = self.optimizer_state.step as f64;
                let correction1 = 1.0 - beta1.powi(t as i32);
                let correction2 = 1.0 - beta2.powi(t as i32);

                for i in 0..2 {
                    self.optimizer_state.m_w[i] =
                        beta1 * self.optimizer_state.m_w[i] + (1.0 - beta1) * grad_w[i];
                    self.optimizer_state.v_w[i] =
                        beta2 * self.optimizer_state.v_w[i] + (1.0 - beta2) * grad_w[i] * grad_w[i];
                    let m_hat = self.optimizer_state.m_w[i] / correction1;
                    let v_hat = self.optimizer_state.v_w[i] / correction2;
                    self.weights.w[i] -= lr * m_hat / (v_hat.sqrt() + eps);
                }

                self.optimizer_state.m_b =
                    beta1 * self.optimizer_state.m_b + (1.0 - beta1) * grad_b;
                self.optimizer_state.v_b =
                    beta2 * self.optimizer_state.v_b + (1.0 - beta2) * grad_b * grad_b;
                let m_b_hat = self.optimizer_state.m_b / correction1;
                let v_b_hat = self.optimizer_state.v_b / correction2;
                self.weights.b -= lr * m_b_hat / (v_b_hat.sqrt() + eps);
            }
        }

        self.global_step += 1;
        self.cursor.advance();
        loss
    }

    /// Evaluates a canary step on a deep clone of self without mutating state (D7)
    pub fn evaluate_canary(&self) -> Result<f64, DomainError> {
        let mut canary_clone = self.clone();
        let loss = canary_clone.step();
        if !loss.is_finite() || loss.is_nan() {
            return Err(DomainError::Validation("Canary step produced non-finite loss".into()));
        }
        Ok(loss)
    }

    /// Saves complete checkpoint bundle to a local directory with manifest
    pub fn save_checkpoint(
        &self,
        dir: &Path,
        checkpoint_id: &str,
        storage_uri: &str,
    ) -> Result<TrainingCheckpointManifest, DomainError> {
        fs::create_dir_all(dir)
            .map_err(|e| DomainError::Validation(format!("Failed to create dir: {e}")))?;

        let mut files = Vec::new();

        // 1. Model weights
        let weights_json = serde_json::to_string(&self.weights)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let weights_path = dir.join("model.safetensors");
        fs::write(&weights_path, &weights_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        files.push(CheckpointFileEntry {
            path: "model.safetensors".into(),
            size_bytes: weights_json.len() as u64,
            digest: digest(weights_json.as_bytes()),
            required_for_state_resume: true,
        });

        // 2. Optimizer state (empty if SgdNoMomentum)
        let opt_json = if self.optimizer_state.is_empty_tensor_state() {
            "{}".to_string()
        } else {
            serde_json::to_string(&self.optimizer_state)
                .map_err(|e| DomainError::Validation(e.to_string()))?
        };
        let opt_path = dir.join("optimizer.pt");
        fs::write(&opt_path, &opt_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        files.push(CheckpointFileEntry {
            path: "optimizer.pt".into(),
            size_bytes: opt_json.len() as u64,
            digest: digest(opt_json.as_bytes()),
            required_for_state_resume: true,
        });

        // 3. Scheduler state
        let sched_json = serde_json::json!({
            "global_step": self.global_step,
            "lr_multiplier": 1.0
        })
        .to_string();
        let sched_path = dir.join("scheduler.pt");
        fs::write(&sched_path, &sched_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        files.push(CheckpointFileEntry {
            path: "scheduler.pt".into(),
            size_bytes: sched_json.len() as u64,
            digest: digest(sched_json.as_bytes()),
            required_for_state_resume: true,
        });

        // 4. RNG state
        let rng_json = serde_json::to_string(&self.prng)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rng_path = dir.join("rng_state.pth");
        fs::write(&rng_path, &rng_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        files.push(CheckpointFileEntry {
            path: "rng_state.pth".into(),
            size_bytes: rng_json.len() as u64,
            digest: digest(rng_json.as_bytes()),
            required_for_state_resume: true,
        });

        // 5. Trainer state & cursor
        let trainer_meta = TrainerStateMetadata {
            global_step: self.global_step,
            epoch: self.cursor.epoch as f64
                + (self.cursor.index as f64 / self.cursor.dataset_len as f64),
            cursor: self.cursor,
            last_loss: self.last_loss,
        };
        let meta_json = serde_json::to_string(&trainer_meta)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let meta_path = dir.join("trainer_state.json");
        fs::write(&meta_path, &meta_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        files.push(CheckpointFileEntry {
            path: "trainer_state.json".into(),
            size_bytes: meta_json.len() as u64,
            digest: digest(meta_json.as_bytes()),
            required_for_state_resume: true,
        });

        // Manifest digest: hash of combined sorted file digests
        let mut digests_combined = String::new();
        for f in &files {
            digests_combined.push_str(&f.digest);
        }
        let storage_digest = digest(digests_combined.as_bytes());

        let mut manifest = TrainingCheckpointManifest::new(
            checkpoint_id,
            self.global_step,
            trainer_meta.epoch,
            storage_uri,
            storage_digest,
        )
        .with_loss(self.last_loss as f32)
        .with_commit_status(CheckpointCommitStatus::LocalStaged)
        .with_full_state(
            "synthetic-linear-v1",
            "deterministic-cpu-1.0",
            serde_json::to_string(&self.cursor).unwrap(),
        );

        for f in files {
            manifest = manifest.add_file(f);
        }

        manifest.validate()?;
        Ok(manifest)
    }

    /// Restores trainer from a checkpoint directory and verified manifest
    pub fn restore_from_checkpoint(
        dir: &Path,
        manifest: &TrainingCheckpointManifest,
    ) -> Result<Self, DomainError> {
        manifest.validate_for_state_resume()?;

        // Verify each file's presence, size, and digest
        for file in &manifest.files {
            let file_path = dir.join(&file.path);
            let bytes = fs::read(&file_path).map_err(|e| {
                DomainError::IntegrityCheckFailed {
                    expected: file.path.clone(),
                    actual: format!("Missing file: {e}"),
                }
            })?;

            if bytes.len() as u64 != file.size_bytes {
                return Err(DomainError::IntegrityCheckFailed {
                    expected: format!("size={}", file.size_bytes),
                    actual: format!("size={}", bytes.len()),
                });
            }

            let computed_digest = digest(&bytes);
            if computed_digest != file.digest {
                return Err(DomainError::IntegrityCheckFailed {
                    expected: file.digest.clone(),
                    actual: computed_digest,
                });
            }
        }

        // 1. Model weights
        let weights_json = fs::read_to_string(dir.join("model.safetensors"))
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let weights: ModelWeights = serde_json::from_str(&weights_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        // 2. Optimizer state
        let opt_json = fs::read_to_string(dir.join("optimizer.pt"))
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let optimizer_state: OptimizerState = if opt_json == "{}" {
            OptimizerState::new(OptimizerKind::SgdNoMomentum { lr: 0.05 }, weights.w.len())
        } else {
            serde_json::from_str(&opt_json).map_err(|e| DomainError::Validation(e.to_string()))?
        };

        // 3. RNG state
        let rng_json = fs::read_to_string(dir.join("rng_state.pth"))
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let prng: DeterministicPrng = serde_json::from_str(&rng_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        // 4. Trainer state & cursor
        let meta_json = fs::read_to_string(dir.join("trainer_state.json"))
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let meta: TrainerStateMetadata = serde_json::from_str(&meta_json)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(Self {
            weights,
            optimizer_state,
            cursor: meta.cursor,
            prng,
            global_step: meta.global_step,
            last_loss: meta.last_loss,
            true_w: vec![2.5, -1.5],
            true_b: 0.5,
        })
    }
}
