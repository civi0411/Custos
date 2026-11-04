//! CAS Checkpoint Store Adapter (F10 / P3 / GPU-F).
//!
//! Provides durable content-addressed storage for checkpoint blobs, manifest publishing,
//! generation-CAS promotion fencing (stopping zombie writers), and corrupt checkpoint quarantining.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use custos_core::contracts::checkpoint_store::{
    BlobMetadata, CheckpointStorePort, PromotionOutcome, StoreCapabilities,
};
use custos_domain::{DomainError, TrainingCheckpointManifest};

type BlobMap = HashMap<String, (Vec<u8>, BlobMetadata)>;

/// In-memory and local disk Content-Addressed Checkpoint Store Adapter
#[derive(Clone)]
pub struct CasCheckpointStoreAdapter {
    root_dir: PathBuf,
    blobs: Arc<Mutex<BlobMap>>, // key -> (data, metadata)
    manifests: Arc<Mutex<HashMap<String, TrainingCheckpointManifest>>>, // checkpoint_id -> manifest
    experiment_manifests: Arc<Mutex<HashMap<String, Vec<String>>>>, // experiment_id -> Vec<checkpoint_id>
    promotions: Arc<Mutex<HashMap<String, (u64, String)>>>, // experiment_id -> (generation, checkpoint_id)
}

impl CasCheckpointStoreAdapter {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
            blobs: Arc::new(Mutex::new(HashMap::new())),
            manifests: Arc::new(Mutex::new(HashMap::new())),
            experiment_manifests: Arc::new(Mutex::new(HashMap::new())),
            promotions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn blob_key(prefix: &str, name: &str) -> String {
        format!("{}/{}", prefix.trim_end_matches('/'), name.trim_start_matches('/'))
    }
}

#[async_trait]
impl CheckpointStorePort for CasCheckpointStoreAdapter {
    async fn capabilities(&self) -> StoreCapabilities {
        StoreCapabilities {
            supports_conditional_put: true,
            supports_prefix_fencing: true,
            supports_multipart_checksum: true,
        }
    }

    async fn put_blob(
        &self,
        prefix: &str,
        name: &str,
        data: &[u8],
        expected_digest: &str,
    ) -> Result<(), DomainError> {
        // 1. Verify content SHA-256 digest
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual_digest = format!("{:x}", hasher.finalize());

        if !expected_digest.is_empty() && actual_digest != expected_digest.to_ascii_lowercase() {
            return Err(DomainError::Validation(format!(
                "Blob digest mismatch for '{}/{}': expected {}, got {}",
                prefix, name, expected_digest, actual_digest
            )));
        }

        let key = Self::blob_key(prefix, name);
        let metadata = BlobMetadata {
            size_bytes: data.len() as u64,
            digest: actual_digest,
            created_at_epoch_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        // 2. Persist to in-memory map
        let mut blobs = self.blobs.lock().await;
        blobs.insert(key, (data.to_vec(), metadata));

        // 3. Write to local staging filesystem if root_dir exists
        let disk_target = self.root_dir.join(prefix).join(name);
        if let Some(parent) = disk_target.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&disk_target, data).await;

        Ok(())
    }

    async fn get_blob(&self, prefix: &str, name: &str) -> Result<Option<Vec<u8>>, DomainError> {
        let key = Self::blob_key(prefix, name);
        let blobs = self.blobs.lock().await;
        if let Some((data, _)) = blobs.get(&key) {
            return Ok(Some(data.clone()));
        }

        let disk_target = self.root_dir.join(prefix).join(name);
        if disk_target.exists() {
            if let Ok(bytes) = tokio::fs::read(&disk_target).await {
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }

    async fn head_blob(&self, prefix: &str, name: &str) -> Result<Option<BlobMetadata>, DomainError> {
        let key = Self::blob_key(prefix, name);
        let blobs = self.blobs.lock().await;
        if let Some((_, meta)) = blobs.get(&key) {
            return Ok(Some(meta.clone()));
        }

        let disk_target = self.root_dir.join(prefix).join(name);
        if disk_target.exists() {
            if let Ok(meta) = tokio::fs::metadata(&disk_target).await {
                return Ok(Some(BlobMetadata {
                    size_bytes: meta.len(),
                    digest: String::new(),
                    created_at_epoch_ms: 0,
                }));
            }
        }
        Ok(None)
    }

    async fn publish_manifest(
        &self,
        prefix: &str,
        manifest: &TrainingCheckpointManifest,
    ) -> Result<(), DomainError> {
        // 1. Validate manifest structural invariants
        manifest.validate()?;

        // 2. Verify all inventoried files are present and match digests
        let blobs = self.blobs.lock().await;
        for file in &manifest.files {
            let key = Self::blob_key(prefix, &file.path);
            if let Some((data, meta)) = blobs.get(&key) {
                if data.len() as u64 != file.size_bytes || meta.digest != file.digest.to_ascii_lowercase() {
                    return Err(DomainError::Validation(format!(
                        "Inventoried file '{}' digest/size mismatch in store",
                        file.path
                    )));
                }
            } else {
                let disk_target = self.root_dir.join(prefix).join(&file.path);
                if !disk_target.exists() {
                    return Err(DomainError::Validation(format!(
                        "Inventoried file '{}' missing from store prefix '{}'",
                        file.path, prefix
                    )));
                }
            }
        }
        drop(blobs);

        // 3. Register manifest
        let mut manifests = self.manifests.lock().await;
        manifests.insert(manifest.checkpoint_id.clone(), manifest.clone());

        let mut exp_map = self.experiment_manifests.lock().await;
        // Prefix is usually experiments/{exp_id}/attempts/{attempt_id}/
        let exp_id = prefix
            .strip_prefix("experiments/")
            .and_then(|p| p.split('/').next())
            .unwrap_or("default");

        exp_map
            .entry(exp_id.to_string())
            .or_default()
            .push(manifest.checkpoint_id.clone());

        Ok(())
    }

    async fn promote(
        &self,
        experiment_id: &str,
        checkpoint_id: &str,
        expected_generation: u64,
    ) -> Result<PromotionOutcome, DomainError> {
        // 1. Fetch manifest
        let manifests = self.manifests.lock().await;
        let manifest = match manifests.get(checkpoint_id) {
            Some(m) => m.clone(),
            None => {
                return Err(DomainError::NotFound {
                    kind: "checkpoint_manifest".into(),
                    id: checkpoint_id.to_string(),
                })
            }
        };
        drop(manifests);

        // 2. Pre-resume verification: must have full training state bundle and be CommittedProtected
        if let Err(e) = manifest.validate_for_state_resume() {
            return Ok(PromotionOutcome::Quarantined {
                reason: format!("Checkpoint failed state resume gate: {}", e),
            });
        }

        // 3. Generation CAS fencing: reject stale writers
        let mut promotions = self.promotions.lock().await;
        if let Some((cur_gen, cur_ckpt)) = promotions.get(experiment_id) {
            if expected_generation < *cur_gen {
                return Ok(PromotionOutcome::StaleGeneration {
                    actual_generation: *cur_gen,
                });
            }
            if expected_generation == *cur_gen && cur_ckpt != checkpoint_id {
                return Ok(PromotionOutcome::StaleGeneration {
                    actual_generation: *cur_gen,
                });
            }
        }

        // 4. Promote
        promotions.insert(
            experiment_id.to_string(),
            (expected_generation, checkpoint_id.to_string()),
        );

        Ok(PromotionOutcome::Promoted {
            canonical_step: manifest.global_step,
        })
    }

    async fn list_checkpoints(
        &self,
        experiment_id: &str,
    ) -> Result<Vec<TrainingCheckpointManifest>, DomainError> {
        let exp_map = self.experiment_manifests.lock().await;
        let manifests = self.manifests.lock().await;

        let mut result = Vec::new();
        if let Some(ids) = exp_map.get(experiment_id) {
            for id in ids {
                if let Some(m) = manifests.get(id) {
                    result.push(m.clone());
                }
            }
        }
        result.sort_by_key(|m| m.global_step);
        Ok(result)
    }

    async fn delete_attempt_prefix(&self, prefix: &str) -> Result<(), DomainError> {
        let mut blobs = self.blobs.lock().await;
        let pfx = prefix.trim_end_matches('/');
        blobs.retain(|k, _| !k.starts_with(pfx));

        let disk_dir = self.root_dir.join(pfx);
        if disk_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&disk_dir).await;
        }
        Ok(())
    }
}
