//! E2E Test Suite: Training Checkpoint Relay & Recovery (F10 / P2 & Golden Scenario G11).
//!
//! Validates:
//! 1. Bit-exact parity between uninterrupted execution (0->100) and interrupted/resumed execution (0->50, restore, 51->100).
//! 2. D5: Valid empty optimizer state (SGD without momentum) resumes correctly.
//! 3. D6: Data cursor desynchronization is detected by fixture baseline comparison.
//! 4. D7: Canary evaluation runs on isolated state and never mutates live training weights.
//! 5. Tampering/corruption with checkpoint files fails closed with IntegrityCheckFailed.
//! 6. Three-tier progress triplet guarantees (observed >= local >= protected).

use tempfile::tempdir;
use custos_domain::{
    digest, CheckpointCommitStatus, DomainError, ProgressTriplet, ProtectionPolicy,
    ProtectionStatus,
};
use custos_testkit::{
    DataCursor, DeterministicTrainer, OptimizerKind,
};

#[test]
fn test_uninterrupted_vs_interrupted_resumed_parity() {
    let seed = 42;
    let dataset_len = 1000;
    let opt = OptimizerKind::Adam {
        lr: 0.01,
        beta1: 0.9,
        beta2: 0.999,
        eps: 1e-8,
    };

    // ─── Branch A: Uninterrupted run (step 0 to 100) ───
    let mut trainer_a = DeterministicTrainer::new(opt.clone(), seed, dataset_len);
    let mut losses_a = Vec::new();
    for _ in 0..100 {
        losses_a.push(trainer_a.step());
    }
    assert_eq!(trainer_a.global_step, 100);

    // ─── Branch B: Run 0 to 50 -> Checkpoint -> Restore -> Run 51 to 100 ───
    let tmp = tempdir().unwrap();
    let ckpt_dir = tmp.path().join("ckpt_step_50");

    let mut trainer_b = DeterministicTrainer::new(opt.clone(), seed, dataset_len);
    let mut losses_b = Vec::new();
    for _ in 0..50 {
        losses_b.push(trainer_b.step());
    }
    assert_eq!(trainer_b.global_step, 50);

    // Verify first 50 steps match exactly
    assert_eq!(&losses_a[..50], &losses_b[..50]);

    // Save checkpoint at step 50
    let mut manifest = trainer_b
        .save_checkpoint(&ckpt_dir, "ckpt_50", "file:///remote/storage/ckpt_50")
        .expect("Checkpoint save must succeed");

    assert_eq!(manifest.global_step, 50);
    assert_eq!(manifest.commit_status, CheckpointCommitStatus::LocalStaged);

    // Simulate commit to protected storage
    manifest.commit_status = CheckpointCommitStatus::CommittedProtected;
    manifest
        .validate_for_state_resume()
        .expect("Protected manifest must validate for state resume");

    // Restore into a fresh Attempt 2 trainer instance
    let mut restored_trainer =
        DeterministicTrainer::restore_from_checkpoint(&ckpt_dir, &manifest)
            .expect("Checkpoint restore must succeed");

    assert_eq!(restored_trainer.global_step, 50);
    assert_eq!(restored_trainer.weights, trainer_b.weights);
    assert_eq!(restored_trainer.optimizer_state, trainer_b.optimizer_state);
    assert_eq!(restored_trainer.cursor, trainer_b.cursor);

    // Continue training from step 51 to 100
    for _ in 50..100 {
        losses_b.push(restored_trainer.step());
    }
    assert_eq!(restored_trainer.global_step, 100);

    // ─── Verify Bit-Exact Parity at Step 100 ───
    assert_eq!(
        trainer_a.weights.w, restored_trainer.weights.w,
        "Model weights W must match bit-exact"
    );
    assert_eq!(
        trainer_a.weights.b, restored_trainer.weights.b,
        "Model bias b must match bit-exact"
    );
    assert_eq!(
        trainer_a.last_loss, restored_trainer.last_loss,
        "Final loss must match bit-exact"
    );
    assert_eq!(
        trainer_a.cursor, restored_trainer.cursor,
        "Data cursor must match bit-exact"
    );
    assert_eq!(
        losses_a, losses_b,
        "Full loss trajectory must match bit-exact between uninterrupted and resumed"
    );
}

#[test]
fn test_d5_sgd_no_momentum_empty_state_resumes_bit_exact() {
    let seed = 12345;
    let dataset_len = 500;
    // SGD with no momentum: optimizer state contains NO tensors!
    let opt = OptimizerKind::SgdNoMomentum { lr: 0.05 };

    // Baseline
    let mut trainer_base = DeterministicTrainer::new(opt.clone(), seed, dataset_len);
    for _ in 0..60 {
        trainer_base.step();
    }

    // Interrupted run
    let tmp = tempdir().unwrap();
    let ckpt_dir = tmp.path().join("ckpt_sgd_30");

    let mut trainer = DeterministicTrainer::new(opt.clone(), seed, dataset_len);
    for _ in 0..30 {
        trainer.step();
    }

    let mut manifest = trainer
        .save_checkpoint(&ckpt_dir, "sgd_ckpt", "file:///storage/sgd_ckpt")
        .expect("Save must succeed");

    // D5 verification: verify optimizer.pt on disk is literally empty JSON "{}"
    let opt_content = std::fs::read_to_string(ckpt_dir.join("optimizer.pt")).unwrap();
    assert_eq!(opt_content, "{}", "SGD without momentum must store empty tensor state");

    manifest.commit_status = CheckpointCommitStatus::CommittedProtected;
    manifest.validate_for_state_resume().unwrap();

    let mut resumed = DeterministicTrainer::restore_from_checkpoint(&ckpt_dir, &manifest)
        .expect("Restore with empty optimizer state must succeed");

    for _ in 30..60 {
        resumed.step();
    }

    assert_eq!(trainer_base.weights, resumed.weights);
    assert_eq!(trainer_base.last_loss, resumed.last_loss);
}

#[test]
fn test_d6_cursor_mutation_or_desync_detected_by_oracle() {
    let seed = 999;
    let dataset_len = 1000;
    let opt = OptimizerKind::SgdWithMomentum { lr: 0.01, momentum: 0.9 };

    let mut baseline = DeterministicTrainer::new(opt.clone(), seed, dataset_len);
    for _ in 0..100 {
        baseline.step();
    }

    let tmp = tempdir().unwrap();
    let ckpt_dir = tmp.path().join("ckpt_d6");

    let mut trainer = DeterministicTrainer::new(opt.clone(), seed, dataset_len);
    for _ in 0..50 {
        trainer.step();
    }

    let mut manifest = trainer
        .save_checkpoint(&ckpt_dir, "d6_ckpt", "file:///storage/d6")
        .unwrap();
    manifest.commit_status = CheckpointCommitStatus::CommittedProtected;

    // Simulate faulty cursor in checkpoint (e.g., reset index to 0)
    let faulty_cursor = DataCursor { epoch: 0, index: 0, dataset_len, shuffle_seed: seed };
    let mut meta: custos_testkit::TrainerStateMetadata =
        serde_json::from_str(&std::fs::read_to_string(ckpt_dir.join("trainer_state.json")).unwrap()).unwrap();
    meta.cursor = faulty_cursor;
    let meta_json = serde_json::to_string(&meta).unwrap();
    std::fs::write(ckpt_dir.join("trainer_state.json"), &meta_json).unwrap();

    // Update manifest file entry to match new file content so integrity passes but cursor is wrong
    for f in &mut manifest.files {
        if f.path == "trainer_state.json" {
            f.size_bytes = meta_json.len() as u64;
            f.digest = digest(meta_json.as_bytes());
        }
    }

    let mut resumed = DeterministicTrainer::restore_from_checkpoint(&ckpt_dir, &manifest).unwrap();
    for _ in 50..100 {
        resumed.step();
    }

    // Oracle baseline detects cursor desync: weights diverge because samples were replayed
    assert_ne!(
        baseline.weights, resumed.weights,
        "Faulty cursor must produce divergent weights detected by baseline oracle"
    );
}

#[test]
fn test_d7_canary_evaluation_does_not_mutate_state() {
    let opt = OptimizerKind::Adam { lr: 0.01, beta1: 0.9, beta2: 0.999, eps: 1e-8 };
    let mut trainer = DeterministicTrainer::new(opt, 777, 500);

    for _ in 0..25 {
        trainer.step();
    }

    let weights_before = trainer.weights.clone();
    let opt_before = trainer.optimizer_state.clone();
    let cursor_before = trainer.cursor;
    let step_before = trainer.global_step;
    let loss_before = trainer.last_loss;

    // Run canary step
    let canary_loss = trainer.evaluate_canary().expect("Canary step must succeed");
    assert!(canary_loss.is_finite());

    // Assert live trainer was NOT mutated by canary (D7)
    assert_eq!(trainer.weights, weights_before);
    assert_eq!(trainer.optimizer_state, opt_before);
    assert_eq!(trainer.cursor, cursor_before);
    assert_eq!(trainer.global_step, step_before);
    assert_eq!(trainer.last_loss, loss_before);
}

#[test]
fn test_checkpoint_integrity_tampering_fails_closed() {
    let opt = OptimizerKind::SgdNoMomentum { lr: 0.01 };
    let mut trainer = DeterministicTrainer::new(opt, 111, 200);
    trainer.step();

    let tmp = tempdir().unwrap();
    let ckpt_dir = tmp.path().join("tampered_ckpt");
    let mut manifest = trainer
        .save_checkpoint(&ckpt_dir, "tamper_test", "file:///storage/tamper")
        .unwrap();
    manifest.commit_status = CheckpointCommitStatus::CommittedProtected;

    // Tamper with weights file by appending junk
    let weights_path = ckpt_dir.join("model.safetensors");
    let mut raw_bytes = std::fs::read(&weights_path).unwrap();
    raw_bytes.push(0xFF);
    std::fs::write(&weights_path, raw_bytes).unwrap();

    // Restoring must fail closed with IntegrityCheckFailed
    let err = DeterministicTrainer::restore_from_checkpoint(&ckpt_dir, &manifest)
        .expect_err("Tampered weights must fail integrity check");

    assert!(
        matches!(err, DomainError::IntegrityCheckFailed { .. }),
        "Expected IntegrityCheckFailed, got: {:?}",
        err
    );
}

#[test]
fn test_progress_triplet_reporting_and_policy() {
    // Spec example: observed 920, local 900, protected 850
    let progress = ProgressTriplet {
        observed: Some(920),
        local: Some(900),
        protected: Some(850),
    };

    assert!(progress.validate().is_ok());
    // Only protected step is safe recovery point
    assert_eq!(progress.recovery_point(), Some(850));

    let policy = ProtectionPolicy {
        max_unprotected_steps: 100, // 920 - 850 = 70 <= 100 -> Protected
        max_unprotected_secs: 300,
        max_inflight_uploads: 2,
    };

    let status = custos_domain::protection_status(&progress, &policy, 60, 1);
    assert_eq!(status, ProtectionStatus::Protected);

    // If observed step jumps to 960 (960 - 850 = 110 > 100) -> AtRisk
    let risky_progress = ProgressTriplet {
        observed: Some(960),
        local: Some(900),
        protected: Some(850),
    };
    assert_eq!(
        custos_domain::protection_status(&risky_progress, &policy, 60, 1),
        ProtectionStatus::AtRisk
    );

    // If inflight uploads exceed max (e.g. 3 > 2) -> Degraded
    assert_eq!(
        custos_domain::protection_status(&progress, &policy, 60, 3),
        ProtectionStatus::Degraded
    );
}
