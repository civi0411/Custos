use crate::traits::ArtifactStore;
use async_trait::async_trait;
use custos_core_domain::{digest, new_id, ArtifactRef, DomainError};
use std::path::PathBuf;

pub struct FsArtifactStore {
    base_dir: PathBuf,
}

impl FsArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[async_trait]
impl ArtifactStore for FsArtifactStore {
    async fn put(&self, data: &[u8], mime: Option<String>) -> Result<ArtifactRef, DomainError> {
        let hash = digest(data);
        let id = new_id("art");
        let path = self.base_dir.join(&hash);
        tokio::fs::create_dir_all(&self.base_dir)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(ArtifactRef {
            id,
            hash,
            path: path.to_string_lossy().to_string(),
            mime,
        })
    }

    async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>, DomainError> {
        let path = self.base_dir.join(hash);
        if !path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read(&path)
            .await
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Some(data))
    }

    async fn exists(&self, hash: &str) -> Result<bool, DomainError> {
        Ok(self.base_dir.join(hash).exists())
    }
}
