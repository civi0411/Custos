//! Source Acquisition Adapter
//!
//! Provides concrete implementation of SourceAcquisitionPort for fetching
//! complete UTF-8 text from local files beneath an explicitly configured root.

use async_trait::async_trait;
use custos_core::contracts::source_acquisition::SourceAcquisitionPort;
use custos_domain::DomainError;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

pub struct SourceFetchAdapter {
    default_base_dir: Option<PathBuf>,
}

impl SourceFetchAdapter {
    pub fn new() -> Self {
        Self {
            default_base_dir: None,
        }
    }

    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            default_base_dir: Some(base_dir.into()),
        }
    }
}

impl Default for SourceFetchAdapter {
    fn default() -> Self {
        Self::new()
    }
}

const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024; // 4 MiB bound

#[async_trait]
impl SourceAcquisitionPort for SourceFetchAdapter {
    async fn fetch_source_text(
        &self,
        path_or_uri: &str,
        base_path: Option<&str>,
    ) -> Result<String, DomainError> {
        if path_or_uri.trim().is_empty() {
            return Err(DomainError::Validation("Empty source path".into()));
        }

        if path_or_uri.contains("://") {
            return Err(DomainError::Validation(
                "SourceFetchAdapter supports local files only".into(),
            ));
        }
        let trusted_root = self.default_base_dir.as_ref().ok_or_else(|| {
            DomainError::Unauthorized("Source acquisition requires an explicit trusted root".into())
        })?;
        let canonical_root = tokio::fs::canonicalize(trusted_root)
            .await
            .map_err(|error| {
                DomainError::Validation(format!("Invalid trusted source root: {error}"))
            })?;

        // 2. Base path must be within trusted root if supplied
        let effective_base = if let Some(bp) = base_path {
            let bp_path = Path::new(bp);
            let combined = if bp_path.is_absolute() {
                bp_path.to_path_buf()
            } else {
                canonical_root.join(bp_path)
            };
            let canonical_bp = tokio::fs::canonicalize(combined).await.map_err(|error| {
                DomainError::Validation(format!("Invalid source base path: {error}"))
            })?;
            if !canonical_bp.starts_with(&canonical_root) {
                return Err(DomainError::Unauthorized(format!(
                    "Base path '{}' escapes trusted root '{}'",
                    bp,
                    canonical_root.display()
                )));
            }
            canonical_bp
        } else {
            canonical_root.clone()
        };

        // 3. Resolve target path within effective base
        let target = Path::new(path_or_uri);
        let combined = if target.is_absolute() {
            target.to_path_buf()
        } else {
            effective_base.join(target)
        };

        let canonical_target = tokio::fs::canonicalize(&combined).await.map_err(|e| {
            DomainError::Validation(format!(
                "Source file '{}' does not exist or cannot be resolved: {e}",
                combined.display()
            ))
        })?;

        if !canonical_target.starts_with(&canonical_root) {
            return Err(DomainError::Unauthorized(format!(
                "Path traversal violation: target '{}' escapes trusted root '{}'",
                path_or_uri,
                canonical_root.display()
            )));
        }

        let file = tokio::fs::File::open(&canonical_target)
            .await
            .map_err(|e| {
                DomainError::Validation(format!(
                    "Failed to open source '{}': {e}",
                    canonical_target.display()
                ))
            })?;
        let metadata = file
            .metadata()
            .await
            .map_err(|error| DomainError::Validation(format!("Cannot inspect source: {error}")))?;
        if !metadata.is_file() {
            return Err(DomainError::Validation(
                "Source target is not a regular file".into(),
            ));
        }
        // Bound the read itself, including files that grow after metadata is observed.
        let mut content = Vec::new();
        file.take(MAX_SOURCE_BYTES + 1)
            .read_to_end(&mut content)
            .await
            .map_err(|error| DomainError::Validation(format!("Cannot read source: {error}")))?;
        if content.len() as u64 > MAX_SOURCE_BYTES {
            return Err(DomainError::Validation(
                "Source exceeds the 4 MiB acquisition limit".into(),
            ));
        }
        String::from_utf8(content)
            .map_err(|error| DomainError::Validation(format!("Source is not valid UTF-8: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_source_fetch_reads_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "custos_fetch_test_{}",
            custos_domain::new_id("test")
        ));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "Source content line 1\nline 2")
            .await
            .unwrap();

        let adapter = SourceFetchAdapter::with_base_dir(&temp_dir);
        let content = adapter.fetch_source_text("sample.txt", None).await.unwrap();
        assert_eq!(content, "Source content line 1\nline 2");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_source_fetch_rejects_traversal() {
        let temp_dir = std::env::temp_dir().join(format!(
            "custos_fetch_trav_{}",
            custos_domain::new_id("test")
        ));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();

        let adapter = SourceFetchAdapter::with_base_dir(&temp_dir);
        let res = adapter.fetch_source_text("../../etc/passwd", None).await;
        assert!(res.is_err(), "Must reject traversal out of base dir");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn explicit_root_is_required_and_external_paths_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        tokio::fs::create_dir(&root).await.unwrap();
        let outside = dir.path().join("outside.txt");
        tokio::fs::write(&outside, "outside").await.unwrap();
        let outside = outside.to_str().unwrap();
        assert!(matches!(
            SourceFetchAdapter::new()
                .fetch_source_text(outside, None)
                .await,
            Err(DomainError::Unauthorized(_))
        ));
        let adapter = SourceFetchAdapter::with_base_dir(&root);
        for (path, base) in [
            ("../outside.txt", None),
            (outside, None),
            (outside, Some(dir.path().to_str().unwrap())),
        ] {
            assert!(matches!(
                adapter.fetch_source_text(path, base).await,
                Err(DomainError::Unauthorized(_))
            ));
        }
    }

    #[tokio::test]
    async fn missing_bases_and_incomplete_or_non_utf8_sources_are_not_evidence() {
        let root = tempfile::tempdir().unwrap();
        let adapter = SourceFetchAdapter::with_base_dir(root.path());
        tokio::fs::write(root.path().join("small.txt"), "complete")
            .await
            .unwrap();
        tokio::fs::write(
            root.path().join("large.txt"),
            vec![b'x'; MAX_SOURCE_BYTES as usize + 1],
        )
        .await
        .unwrap();
        tokio::fs::write(root.path().join("invalid.txt"), [0xff])
            .await
            .unwrap();
        assert_eq!(
            adapter
                .fetch_source_text("small.txt", Some("."))
                .await
                .unwrap(),
            "complete"
        );
        assert!(adapter
            .fetch_source_text("small.txt", Some("missing"))
            .await
            .is_err());
        assert!(adapter.fetch_source_text("large.txt", None).await.is_err());
        assert!(adapter
            .fetch_source_text("invalid.txt", None)
            .await
            .is_err());
        assert!(adapter
            .fetch_source_text("https://example.com", None)
            .await
            .is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_outside_the_root_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        tokio::fs::create_dir(&root).await.unwrap();
        let outside = dir.path().join("outside.txt");
        tokio::fs::write(&outside, "outside").await.unwrap();
        std::os::unix::fs::symlink(outside, root.join("link.txt")).unwrap();
        assert!(matches!(
            SourceFetchAdapter::with_base_dir(root)
                .fetch_source_text("link.txt", None)
                .await,
            Err(DomainError::Unauthorized(_))
        ));
    }
}
