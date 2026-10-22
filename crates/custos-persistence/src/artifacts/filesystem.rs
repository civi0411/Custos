use super::traits::ArtifactStore;
use async_trait::async_trait;
use custos_domain::{digest, new_id, ArtifactRef, DomainError};
use std::path::PathBuf;

const MAX_SAFE_CAS_READ: u64 = 256 * 1024 * 1024; // 256 MiB

pub struct FsArtifactStore {
    base_dir: PathBuf,
}

impl FsArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn validate_hash(hash: &str) -> Result<(), DomainError> {
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainError::Validation(format!(
                "invalid CAS hash '{}': must be 64 hex characters",
                hash
            )));
        }
        if hash.contains('/') || hash.contains('\\') || hash.contains("..") {
            return Err(DomainError::Validation(format!(
                "path traversal detected in CAS key '{}'",
                hash
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl ArtifactStore for FsArtifactStore {
    async fn put(&self, data: &[u8], mime: Option<String>) -> Result<ArtifactRef, DomainError> {
        let hash = digest(data);
        Self::validate_hash(&hash)?;

        let id = new_id("art");
        let path = self.base_dir.join(&hash);
        let temp_path = self.base_dir.join(format!("{}.tmp.{}", hash, new_id("tmp")));
        let size_bytes = data.len() as u64;

        tokio::fs::create_dir_all(&self.base_dir)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        // 1. Write to temporary file first (Crash and partial write safety)
        tokio::fs::write(&temp_path, data)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        // 2. Atomic rename to canonical hash path
        if let Err(e) = tokio::fs::rename(&temp_path, &path).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(DomainError::Validation(format!(
                "failed to atomically commit artifact: {}",
                e
            )));
        }

        // 3. Verify digest matches hash
        let written_hash = digest(data);
        if written_hash != hash {
            let _ = tokio::fs::remove_file(&path).await;
            return Err(DomainError::Validation(format!(
                "CAS integrity check failed after write: expected {}, got {}",
                hash, written_hash
            )));
        }

        Ok(ArtifactRef {
            id,
            kind: custos_domain::ArtifactKind::Other,
            hash,
            path: path.to_string_lossy().to_string(),
            size_bytes,
            mime,
        })
    }

    async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, DomainError> {
        Self::validate_hash(hash)?;

        let path = self.base_dir.join(hash);
        if !path.exists() {
            return Ok(None);
        }

        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if metadata.len() > MAX_SAFE_CAS_READ {
            return Err(DomainError::Validation(format!(
                "CAS artifact exceeds maximum read limit of 256 MiB (size: {} bytes)",
                metadata.len()
            )));
        }

        let data = tokio::fs::read(&path)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let actual_hash = digest(&data);
        if actual_hash != hash {
            return Err(DomainError::Validation(format!(
                "CAS digest mismatch for artifact: expected {}, found {}",
                hash, actual_hash
            )));
        }

        Ok(Some(data))
    }

    async fn exists(&self, hash: &str) -> Result<bool, DomainError> {
        Self::validate_hash(hash)?;
        Ok(self.base_dir.join(hash).exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cas_put_and_get_success() {
        let dir = tempdir().unwrap();
        let store = FsArtifactStore::new(dir.path().to_path_buf());

        let payload = b"trusted cas payload content";
        let art_ref = store.put(payload, Some("text/plain".into())).await.unwrap();

        assert_eq!(art_ref.size_bytes, payload.len() as u64);
        assert_eq!(art_ref.hash, digest(payload));

        let retrieved = store.get(&art_ref.hash).await.unwrap();
        assert_eq!(retrieved, Some(payload.to_vec()));
        assert!(store.exists(&art_ref.hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_cas_path_traversal_rejected() {
        let dir = tempdir().unwrap();
        let store = FsArtifactStore::new(dir.path().to_path_buf());

        let traversal_keys = [
            "../../etc/passwd",
            "../secret",
            "/absolute/path/file",
            "64charsbuthasdots..................................................",
        ];

        for key in traversal_keys {
            let res = store.get(key).await;
            assert!(res.is_err(), "Key '{}' should be rejected as invalid/traversal", key);
        }
    }

    #[tokio::test]
    async fn test_cas_digest_mismatch_detected_and_rejected() {
        let dir = tempdir().unwrap();
        let store = FsArtifactStore::new(dir.path().to_path_buf());

        let payload = b"original content";
        let art_ref = store.put(payload, None).await.unwrap();

        // Corrupt file on disk
        let file_path = dir.path().join(&art_ref.hash);
        tokio::fs::write(&file_path, b"corrupted tampered content").await.unwrap();

        // Reading must fail with digest mismatch, never serving wrong bytes
        let read_res = store.get(&art_ref.hash).await;
        assert!(read_res.is_err());
        assert!(
            matches!(read_res.unwrap_err(), DomainError::Validation(ref msg) if msg.contains("CAS digest mismatch"))
        );
    }
}
