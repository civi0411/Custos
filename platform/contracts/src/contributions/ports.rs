//! Contribution, Package Distribution & Installation SPI Trait Ports

use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StagedArtifact {
    pub staging_path: PathBuf,
    pub actual_digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledPath(pub PathBuf);

/// Distribution metadata envelope for an installable workbench package.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageDistributionEnvelope {
    pub package_id: String,
    pub component_id: String,
    pub version: String,
    pub publisher: String,
    pub channel: String,
    pub platform_target: String,
    pub artifact_digest: String,
    pub signature: Option<String>,
    pub min_host_version: String,
    pub max_host_version: Option<String>,
    pub dependencies: Vec<String>,
    pub contributions: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("digest mismatch: expected {expected}, got {actual}")]
    DigestMismatch { expected: String, actual: String },
    #[error("signature verification failed: {0}")]
    SignatureInvalid(String),
    #[error("path traversal detected in archive: {0}")]
    PathTraversal(String),
    #[error("symlink to outside package dir rejected: {0}")]
    ForbiddenSymlink(String),
    #[error("download size exceeded limit: {actual_bytes} > {limit_bytes}")]
    SizeExceeded { actual_bytes: u64, limit_bytes: u64 },
    #[error("disk full or quota exceeded")]
    DiskFull,
    #[error("transfer cancelled")]
    Cancelled,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait PackageTransferPort: Send + Sync {
    async fn download_to_staging(
        &self,
        source_url: &str,
        component_id: &str,
        expected_size_bytes: Option<u64>,
    ) -> Result<StagedArtifact, PackageError>;

    async fn verify_staged(
        &self,
        staged: &StagedArtifact,
        expected_digest: &str,
    ) -> Result<(), PackageError>;

    async fn promote_staged(
        &self,
        staged: StagedArtifact,
        component_id: &str,
        version: &str,
    ) -> Result<InstalledPath, PackageError>;

    async fn remove_version(&self, component_id: &str, version: &str) -> Result<(), PackageError>;
}

