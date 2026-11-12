//! Pillar 3: Workbench Package Installer
//!
//! Handles tarball extraction, SHA-256 digest verification, bounded download,
//! and staging promotion to `~/.custos/packages/{id}/{version}`.

use async_trait::async_trait;
use custos_core::contracts::packages::{
    InstalledPath, PackageError, PackageTransferPort, StagedArtifact,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub const DEFAULT_MAX_PACKAGE_SIZE_BYTES: u64 = 150 * 1024 * 1024; // 150 MiB bounded limit

pub struct LocalPackageTransferAdapter {
    packages_dir: PathBuf,
    staging_dir: PathBuf,
    client: reqwest::Client,
    max_size_bytes: u64,
}

impl LocalPackageTransferAdapter {
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        let root = root_dir.as_ref().to_path_buf();
        let packages_dir = root.join("packages");
        let staging_dir = packages_dir.join(".staging");
        Self {
            packages_dir,
            staging_dir,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            max_size_bytes: DEFAULT_MAX_PACKAGE_SIZE_BYTES,
        }
    }

    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.max_size_bytes = max_size_bytes;
        self
    }

    pub fn sanitize_name(name: &str) -> Result<(), PackageError> {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains("..")
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(PackageError::PathTraversal(name.to_string()));
        }
        Ok(())
    }
}

#[async_trait]
impl PackageTransferPort for LocalPackageTransferAdapter {
    async fn download_to_staging(
        &self,
        source_url: &str,
        component_id: &str,
        expected_size_bytes: Option<u64>,
    ) -> Result<StagedArtifact, PackageError> {
        Self::sanitize_name(component_id)?;
        fs::create_dir_all(&self.staging_dir).await?;

        let part_file = self.staging_dir.join(format!("{}.part", component_id));
        let mut hasher = Sha256::new();
        let mut total_bytes: u64 = 0;

        if let Some(file_path_str) = source_url.strip_prefix("file://") {
            let path = Path::new(file_path_str);
            if !path.exists() {
                return Err(PackageError::Validation(format!(
                    "Local source file does not exist: {}",
                    file_path_str
                )));
            }
            let bytes = fs::read(path).await?;
            if bytes.len() as u64 > self.max_size_bytes {
                return Err(PackageError::SizeExceeded {
                    actual_bytes: bytes.len() as u64,
                    limit_bytes: self.max_size_bytes,
                });
            }
            hasher.update(&bytes);
            total_bytes = bytes.len() as u64;
            let mut file = fs::File::create(&part_file).await?;
            file.write_all(&bytes).await?;
            file.flush().await?;
        } else if source_url.starts_with("http://") || source_url.starts_with("https://") {
            let mut response = self
                .client
                .get(source_url)
                .send()
                .await
                .map_err(|e| PackageError::Validation(e.to_string()))?;

            if !response.status().is_success() {
                return Err(PackageError::Validation(format!(
                    "HTTP download failed with status: {}",
                    response.status()
                )));
            }

            let mut file = fs::File::create(&part_file).await?;

            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| PackageError::Validation(e.to_string()))?
            {
                total_bytes += chunk.len() as u64;
                if total_bytes > self.max_size_bytes {
                    let _ = fs::remove_file(&part_file).await;
                    return Err(PackageError::SizeExceeded {
                        actual_bytes: total_bytes,
                        limit_bytes: self.max_size_bytes,
                    });
                }
                hasher.update(&chunk);
                file.write_all(&chunk).await?;
            }
            file.flush().await?;
        } else {
            return Err(PackageError::Validation(format!(
                "Unsupported source URL scheme: {}",
                source_url
            )));
        }

        if let Some(expected_size) = expected_size_bytes {
            if total_bytes != expected_size {
                let _ = fs::remove_file(&part_file).await;
                return Err(PackageError::Validation(format!(
                    "Payload size mismatch: expected {}, got {}",
                    expected_size, total_bytes
                )));
            }
        }

        let actual_digest = format!("{:x}", hasher.finalize());
        let staged_file = self
            .staging_dir
            .join(format!("{}-{}.staged", component_id, actual_digest));

        fs::rename(&part_file, &staged_file).await?;

        Ok(StagedArtifact {
            staging_path: staged_file,
            actual_digest,
            size_bytes: total_bytes,
        })
    }

    async fn verify_staged(
        &self,
        staged: &StagedArtifact,
        expected_digest: &str,
    ) -> Result<(), PackageError> {
        let expected = expected_digest.trim().to_lowercase();
        let actual = staged.actual_digest.trim().to_lowercase();
        if expected != actual {
            if staged.staging_path.exists() {
                let _ = fs::remove_file(&staged.staging_path).await;
            }
            return Err(PackageError::DigestMismatch { expected, actual });
        }
        Ok(())
    }

    async fn promote_staged(
        &self,
        staged: StagedArtifact,
        component_id: &str,
        version: &str,
    ) -> Result<InstalledPath, PackageError> {
        Self::sanitize_name(component_id)?;
        Self::sanitize_name(version)?;

        let dest_dir = self.packages_dir.join(component_id).join(version);
        fs::create_dir_all(&dest_dir).await?;

        let dest_file = dest_dir.join("package.bundle");
        fs::rename(&staged.staging_path, &dest_file).await?;

        Ok(InstalledPath(dest_dir))
    }

    async fn remove_version(&self, component_id: &str, version: &str) -> Result<(), PackageError> {
        Self::sanitize_name(component_id)?;
        Self::sanitize_name(version)?;

        let target = self.packages_dir.join(component_id).join(version);
        if target.exists() {
            fs::remove_dir_all(&target).await?;
        }
        Ok(())
    }
}
