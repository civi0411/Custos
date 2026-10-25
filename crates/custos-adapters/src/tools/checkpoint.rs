//! Bounded integrity checks for caller-authorized checkpoint directories.

use custos_domain::{DomainError, TrainingCheckpointManifest};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

/// Checks local SHA-256 inventory only; does not attest remote commit or engine compatibility.
/// The caller must authorize the directory and prevent concurrent filesystem mutation.
#[tracing::instrument(skip(manifest), fields(checkpoint_id = %manifest.checkpoint_id))]
pub fn verify_local_checkpoint_files(
    root: &Path,
    manifest: &TrainingCheckpointManifest,
    max_total_bytes: u64,
) -> Result<(), DomainError> {
    manifest.validate()?;
    let fail = |message: String| DomainError::Validation(message);
    if manifest.files.is_empty() || !root.is_dir() {
        return Err(fail(
            "Checkpoint directory and nonempty inventory are required".into(),
        ));
    }
    let total = manifest
        .files
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.size_bytes))
        .ok_or_else(|| fail("Checkpoint size overflow".into()))?;
    if total > max_total_bytes {
        return Err(fail("Checkpoint exceeds authorized byte limit".into()));
    }
    let root = root
        .canonicalize()
        .map_err(|error| fail(error.to_string()))?;
    for entry in &manifest.files {
        let mut path = root.clone();
        for part in entry.path.split('/') {
            path.push(part);
            let metadata = fs::symlink_metadata(&path).map_err(|error| fail(error.to_string()))?;
            if metadata.file_type().is_symlink() {
                return Err(fail(format!(
                    "Symlink in checkpoint inventory: {}",
                    entry.path
                )));
            }
        }
        if !fs::symlink_metadata(&path)
            .map_err(|error| fail(error.to_string()))?
            .is_file()
        {
            return Err(fail(
                "Checkpoint inventory must contain regular files".into(),
            ));
        }
        let mut file = fs::File::open(&path).map_err(|error| fail(error.to_string()))?;
        let metadata = file.metadata().map_err(|error| fail(error.to_string()))?;
        if !metadata.is_file() || metadata.len() != entry.size_bytes {
            return Err(fail(format!(
                "Checkpoint size or file type mismatch: {}",
                entry.path
            )));
        }
        let mut digest = Sha256::new();
        let mut count = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let n = file
                .read(&mut buffer)
                .map_err(|error| fail(error.to_string()))?;
            if n == 0 {
                break;
            }
            count = count
                .checked_add(n as u64)
                .ok_or_else(|| fail("Checkpoint size overflow".into()))?;
            if count > entry.size_bytes {
                return Err(fail("Checkpoint changed during verification".into()));
            }
            digest.update(&buffer[..n]);
        }
        if count != entry.size_bytes
            || format!("{:x}", digest.finalize()) != entry.digest.to_ascii_lowercase()
        {
            return Err(fail(format!(
                "Checkpoint integrity mismatch: {}",
                entry.path
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::CheckpointFileEntry;

    #[test]
    fn verifies_bytes_and_rejects_corruption_limits_and_traversal() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("weights.bin"), b"test").unwrap();
        let mut manifest = TrainingCheckpointManifest::new("c", 5, 0.0, "s3://bucket/c", "");
        manifest.files.push(CheckpointFileEntry {
            path: "weights.bin".into(),
            size_bytes: 4,
            digest: format!("{:x}", Sha256::digest(b"test")),
            required_for_state_resume: true,
        });
        assert!(verify_local_checkpoint_files(dir.path(), &manifest, 4).is_ok());
        assert!(verify_local_checkpoint_files(dir.path(), &manifest, 3).is_err());
        fs::write(dir.path().join("weights.bin"), b"evil").unwrap();
        assert!(verify_local_checkpoint_files(dir.path(), &manifest, 4).is_err());
        manifest.files[0].path = "../weights.bin".into();
        assert!(verify_local_checkpoint_files(dir.path(), &manifest, 4).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real"), b"test").unwrap();
        std::os::unix::fs::symlink("real", dir.path().join("link")).unwrap();
        let manifest = TrainingCheckpointManifest::new("c", 0, 0.0, "local://c", "").add_file(
            CheckpointFileEntry {
                path: "link".into(),
                size_bytes: 4,
                digest: format!("{:x}", Sha256::digest(b"test")),
                required_for_state_resume: true,
            },
        );
        assert!(verify_local_checkpoint_files(dir.path(), &manifest, 4).is_err());
    }
}
