use crate::fleet::compute::checkpoint::*;

#[test]
fn checkpoint_admission_is_fail_closed() {
    let mut manifest =
        TrainingCheckpointManifest::new("c", 400, 1.0, "s3://bucket/c", "a".repeat(64));
    assert_eq!(manifest.commit_status, CheckpointCommitStatus::Observed);
    assert!(manifest.validate_for_state_resume().is_err());
    manifest = manifest
        .with_full_state("model@rev", "unsloth@rev", "cursor400")
        .with_commit_status(CheckpointCommitStatus::CommittedProtected);
    for path in [
        "adapter_model.safetensors",
        "optimizer.pt",
        "scheduler.pt",
        "rng_state.pth",
        "trainer_state.json",
    ] {
        manifest.files.push(CheckpointFileEntry {
            path: path.into(),
            size_bytes: 1,
            digest: "a".repeat(64),
            required_for_state_resume: true,
        });
    }
    assert!(manifest.validate_for_state_resume().is_ok());
    let mut secret_uri = manifest.clone();
    secret_uri.storage_uri = "https://store/checkpoint?token=secret".into();
    assert!(secret_uri.validate().is_err());
    manifest.rng_state_preserved = false;
    assert!(manifest.validate_for_state_resume().is_err());
    manifest.rng_state_preserved = true;
    manifest.files.retain(|file| file.path != "optimizer.pt");
    assert!(manifest.validate_for_state_resume().is_err());
    manifest.files[0].path = "../weights".into();
    assert!(manifest.validate().is_err());
}

#[test]
fn test_checkpoint_manifest_resumability_and_commit_stages() {
    let mut manifest = TrainingCheckpointManifest::new(
        "ckpt_durable",
        400,
        2.0,
        "r2://bucket/models/attempt-a/ckpt_400",
        "blake3_hash",
    )
    .with_commit_status(CheckpointCommitStatus::LocalStaged)
    .with_resumability(ResumabilityLevel::WarmStart);

    assert_eq!(manifest.commit_status, CheckpointCommitStatus::LocalStaged);
    assert_eq!(manifest.resumability_level, ResumabilityLevel::WarmStart);
    assert!(!manifest.resumability_level.allows_state_continuation());

    // Promote to full state and committed
    manifest = manifest
        .with_full_state("base-model-v1", "torch-2.4", "batch_400")
        .with_commit_status(CheckpointCommitStatus::CommittedProtected)
        .add_file(CheckpointFileEntry {
            path: "optimizer.pt".into(),
            size_bytes: 1024 * 1024 * 50,
            digest: "sha256_opt".into(),
            required_for_state_resume: true,
        });

    assert_eq!(
        manifest.resumability_level,
        ResumabilityLevel::TrainingStateResume
    );
    assert!(manifest.resumability_level.allows_state_continuation());
    assert_eq!(
        manifest.commit_status,
        CheckpointCommitStatus::CommittedProtected
    );
    assert_eq!(manifest.files.len(), 1);
    assert!(manifest.optimizer_state_preserved);
}

