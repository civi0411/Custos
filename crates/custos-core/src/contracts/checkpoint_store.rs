//! Checkpoint Storage Port & Contracts (F10 / P3).
//!
//! Provides durable storage abstractions for checkpoint blob uploads, manifests,
//! conditional promotion, and namespace/prefix fencing across remote backends (S3, R2, HF).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use custos_domain::{DomainError, TrainingCheckpointManifest};

/// Storage backend capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreCapabilities {
    pub supports_conditional_put: bool,
    pub supports_prefix_fencing: bool,
    pub supports_multipart_checksum: bool,
}

/// Metadata for an individual stored blob
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobMetadata {
    pub size_bytes: u64,
    pub digest: String,
    pub created_at_epoch_ms: u64,
}

/// Result of an attempt to promote a checkpoint as canonical latest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PromotionOutcome {
    /// Successfully promoted
    Promoted { canonical_step: u64 },
    /// Rejected because storage generation does not match expected_generation (stale writer fencing)
    StaleGeneration { actual_generation: u64 },
    /// Quarantined due to integrity or missing file anomaly
    Quarantined { reason: String },
}

/// Durable checkpoint store port
#[async_trait]
pub trait CheckpointStorePort: Send + Sync {
    /// Capabilities probe
    async fn capabilities(&self) -> StoreCapabilities;

    /// Uploads an immutable blob into an attempt prefix
    async fn put_blob(
        &self,
        prefix: &str,
        name: &str,
        data: &[u8],
        expected_digest: &str,
    ) -> Result<(), DomainError>;

    /// Retrieves an immutable blob from an attempt prefix
    async fn get_blob(&self, prefix: &str, name: &str) -> Result<Option<Vec<u8>>, DomainError>;

    /// Probes metadata for a blob in an attempt prefix
    async fn head_blob(&self, prefix: &str, name: &str) -> Result<Option<BlobMetadata>, DomainError>;

    /// Publishes an immutable manifest for an attempt checkpoint
    async fn publish_manifest(
        &self,
        prefix: &str,
        manifest: &TrainingCheckpointManifest,
    ) -> Result<(), DomainError>;

    /// Conditional promotion with fencing generation CAS
    async fn promote(
        &self,
        experiment_id: &str,
        checkpoint_id: &str,
        expected_generation: u64,
    ) -> Result<PromotionOutcome, DomainError>;

    /// Lists verified checkpoints for an experiment
    async fn list_checkpoints(
        &self,
        experiment_id: &str,
    ) -> Result<Vec<TrainingCheckpointManifest>, DomainError>;

    /// Cleans up temporary or aborted attempt files
    async fn delete_attempt_prefix(&self, prefix: &str) -> Result<(), DomainError>;
}
