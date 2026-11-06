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

/// A package detected on the local filesystem during auto-probe discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPackage {
    pub component_id: String,
    pub version: String,
    pub platform: String,
    pub digest: String,
    pub installed_path: PathBuf,
}

/// Scans the given packages directory for locally installed or pre-packaged workbenches.
///
/// Directory layout expected: `<packages_dir>/<component_id>/<version>/...`
/// where a valid version directory contains at least one of `package.bundle`, `custos-worker`, or `manifest.json`.
pub fn scan_local_packages(packages_dir: &Path) -> Result<Vec<DiscoveredPackage>, PackageError> {
    if !packages_dir.exists() || !packages_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut discovered = Vec::new();
    let entries = std::fs::read_dir(packages_dir)?;

    for entry in entries.flatten() {
        let comp_path = entry.path();
        if !comp_path.is_dir() {
            continue;
        }

        let comp_name = match comp_path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if comp_name.starts_with('.')
            || LocalPackageTransferAdapter::sanitize_name(&comp_name).is_err()
        {
            continue;
        }

        let version_entries = match std::fs::read_dir(&comp_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for v_entry in version_entries.flatten() {
            let v_path = v_entry.path();
            if !v_path.is_dir() {
                continue;
            }

            let version = match v_path.file_name().and_then(|n| n.to_str()) {
                Some(ver) => ver.to_string(),
                None => continue,
            };

            if version.starts_with('.')
                || LocalPackageTransferAdapter::sanitize_name(&version).is_err()
            {
                continue;
            }

            let bundle_path = v_path.join("package.bundle");
            let worker_path = v_path.join("custos-worker");
            let manifest_path = v_path.join("manifest.json");

            let digest_opt = if bundle_path.exists() {
                std::fs::read(&bundle_path).ok().map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{:x}", hasher.finalize())
                })
            } else if worker_path.exists() {
                std::fs::read(&worker_path).ok().map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{:x}", hasher.finalize())
                })
            } else if manifest_path.exists() {
                std::fs::read(&manifest_path).ok().map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{:x}", hasher.finalize())
                })
            } else {
                None
            };

            if let Some(digest) = digest_opt {
                discovered.push(DiscoveredPackage {
                    component_id: comp_name.clone(),
                    version,
                    platform: "universal".to_string(),
                    digest,
                    installed_path: v_path,
                });
            }
        }
    }

    Ok(discovered)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_local_package_transfer_lifecycle() {
        let root = tempdir().expect("tempdir");
        let adapter = LocalPackageTransferAdapter::new(root.path());

        // 1. Prepare fake local source payload
        let payload = b"custos-quant-worker-mock-binary";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let expected_digest = format!("{:x}", hasher.finalize());

        let source_file = root.path().join("quant.pkg");
        fs::write(&source_file, payload)
            .await
            .expect("write source");

        let source_url = format!("file://{}", source_file.display());

        // 2. Download to staging
        let staged = adapter
            .download_to_staging(&source_url, "quant", Some(payload.len() as u64))
            .await
            .expect("download_to_staging");

        assert_eq!(staged.actual_digest, expected_digest);
        assert_eq!(staged.size_bytes, payload.len() as u64);
        assert!(staged.staging_path.exists());

        // 3. Verify digest
        adapter
            .verify_staged(&staged, &expected_digest)
            .await
            .expect("verify_staged");

        // 4. Promote to installed directory
        let installed = adapter
            .promote_staged(staged, "quant", "0.1.0")
            .await
            .expect("promote_staged");

        assert!(installed.0.join("package.bundle").exists());

        // 5. Remove version
        adapter
            .remove_version("quant", "0.1.0")
            .await
            .expect("remove_version");
        assert!(!installed.0.exists());
    }

    #[tokio::test]
    async fn test_local_package_transfer_rejects_path_traversal() {
        let root = tempdir().expect("tempdir");
        let adapter = LocalPackageTransferAdapter::new(root.path());

        let err = adapter
            .download_to_staging("file:///etc/hosts", "../evil", None)
            .await
            .unwrap_err();

        match err {
            PackageError::PathTraversal(path) => assert_eq!(path, "../evil"),
            other => panic!("Expected PathTraversal, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_local_package_transfer_rejects_digest_mismatch() {
        let root = tempdir().expect("tempdir");
        let adapter = LocalPackageTransferAdapter::new(root.path());

        let source_file = root.path().join("corrupted.pkg");
        fs::write(&source_file, b"content").await.expect("write");
        let source_url = format!("file://{}", source_file.display());

        let staged = adapter
            .download_to_staging(&source_url, "research", None)
            .await
            .expect("download");

        let err = adapter
            .verify_staged(
                &staged,
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .await
            .unwrap_err();

        match err {
            PackageError::DigestMismatch { .. } => {}
            other => panic!("Expected DigestMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn test_scan_local_packages_auto_probe() {
        let root = tempdir().expect("tempdir");
        let packages_dir = root.path().join("packages");

        // Create pre-existing local workbench directory structure
        let quant_dir = packages_dir.join("quant").join("0.3.0");
        std::fs::create_dir_all(&quant_dir).expect("create dir");
        std::fs::write(quant_dir.join("package.bundle"), b"mock-quant-bundle")
            .expect("write bundle");

        let research_dir = packages_dir.join("research").join("1.2.0");
        std::fs::create_dir_all(&research_dir).expect("create dir");
        std::fs::write(research_dir.join("manifest.json"), b"{\"id\":\"research\"}")
            .expect("write manifest");

        let discovered = scan_local_packages(&packages_dir).expect("scan_local_packages");
        assert_eq!(discovered.len(), 2);

        let quant = discovered
            .iter()
            .find(|d| d.component_id == "quant")
            .expect("quant found");
        assert_eq!(quant.version, "0.3.0");
        assert!(quant.digest.starts_with("sha256:"));

        let research = discovered
            .iter()
            .find(|d| d.component_id == "research")
            .expect("research found");
        assert_eq!(research.version, "1.2.0");
    }
}
