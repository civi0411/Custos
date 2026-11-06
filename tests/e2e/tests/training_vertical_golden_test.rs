//! Golden Acceptance Test G11 — Training Studio, Remote Targets & Checkpoint Continuity (F10)
//!
//! Validates:
//! 1. Normal Path & Cross-Target Lineage: Attempt 1 on RunPod saves CommittedProtected checkpoint, promotes via CAS.
//!    Continuation Attempt 2 resumes on DedicatedSsh with parent_checkpoint_id linkage and generation increment.
//! 2. Target Death, Crash Reconciliation & Zombie Writer Fencing: In-flight running attempt recovered to Uncertain
//!    on restart; stale generation promotion by zombie worker is rejected with StaleGeneration.
//! 3. Incomplete Checkpoint Rejection: LocalStaged status, missing optimizer/scheduler state files, or path traversal
//!    attempts fail validation fail-closed before allocating expensive compute.
//! 4. Idempotent Invocations: Remote compute operations carry idempotency keys; duplicate executions conflict cleanly.

use custos_core::contracts::checkpoint_store::PromotionOutcome;
use custos_domain::{
    new_id, CheckpointCommitStatus, CheckpointFileEntry, ComputeBackendKind, DomainError,
    ExecEvent, ExecState, ExecutionAttempt, ExternalJobRef, Invocation, InvocationOutcome,
    ResourceState, TrainingCheckpointManifest,
};
use custos_persistence::SqliteTaskStore;
use std::sync::Arc;

fn make_valid_manifest(
    checkpoint_id: &str,
    step: u64,
    storage_uri: &str,
) -> TrainingCheckpointManifest {
    let dummy_digest = "a".repeat(64);
    let mut manifest = TrainingCheckpointManifest::new(
        checkpoint_id,
        step,
        (step as f64) / 100.0,
        storage_uri,
        dummy_digest.clone(),
    )
    .with_full_state(
        "meta-llama/Llama-3-8B@rev1",
        "torch2.3+unsloth",
        "epoch:1,idx:500",
    )
    .with_commit_status(CheckpointCommitStatus::CommittedProtected);

    for name in &[
        "model.safetensors",
        "optimizer.pt",
        "scheduler.pt",
        "rng_state.pth",
        "trainer_state.json",
    ] {
        manifest = manifest.add_file(CheckpointFileEntry {
            path: (*name).to_string(),
            size_bytes: 1024,
            digest: dummy_digest.clone(),
            required_for_state_resume: true,
        });
    }

    manifest
}

#[test]
fn test_g11_normal_path_checkpoint_continuity_and_cross_target_lineage() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let experiment_id = "exp_llama3_lora_01";
    let recipe_id = "recipe_unsloth_sft";

    // 1. Attempt 1 launched on RunPod
    let mut attempt1 = ExecutionAttempt::new(
        experiment_id,
        recipe_id,
        ComputeBackendKind::RunPod,
        None, // Root attempt: no parent checkpoint
    );
    attempt1.generation = 1;
    attempt1.external_ref = Some(
        ExternalJobRef::new("runpod_acc_01", "pod_job_9981", "v1.2.0").expect("Valid external ref"),
    );

    // Apply lifecycle events: Planned -> Preflighted -> Submitting -> Submitted -> Running
    attempt1.apply(ExecEvent::PreflightOk).unwrap();
    attempt1.apply(ExecEvent::SubmitStarted).unwrap();
    attempt1
        .apply(ExecEvent::SubmitAcked(
            attempt1.external_ref.clone().unwrap(),
        ))
        .unwrap();
    attempt1.apply(ExecEvent::ObservedRunning).unwrap();
    assert_eq!(attempt1.exec_state, ExecState::Running);

    store.experiment_attempts().save_attempt(&attempt1).unwrap();

    // Attempt 1 completes Step 100 and uploads full checkpoint
    let ckpt1_id = "ckpt_step_100";
    let storage_uri1 = "https://r2.cloudflarestorage.com/checkpoints/exp_01/ckpt_100";
    let manifest1 = make_valid_manifest(ckpt1_id, 100, storage_uri1);
    manifest1
        .validate_for_state_resume()
        .expect("Manifest 1 must be valid for state resume");

    // Promote checkpoint at Generation 1
    let promo1 = store
        .experiment_attempts()
        .promote_checkpoint(experiment_id, 1, ckpt1_id, 100)
        .unwrap();
    assert_eq!(
        promo1,
        PromotionOutcome::Promoted {
            canonical_step: 100
        },
        "Generation 1 checkpoint promotion must succeed"
    );

    // Attempt 1 reaches clean completion
    attempt1.apply(ExecEvent::RemoteCompleted).unwrap();
    attempt1.apply(ExecEvent::Collected).unwrap();
    store.experiment_attempts().save_attempt(&attempt1).unwrap();

    // 2. Continuation Attempt 2 launched on a DIFFERENT target (DedicatedSsh)
    // Cross-target handoff: references parent_checkpoint_id with generation increment (2)
    let mut attempt2 = ExecutionAttempt::new(
        experiment_id,
        recipe_id,
        ComputeBackendKind::DedicatedSsh,
        Some(ckpt1_id.into()),
    );
    attempt2.generation = 2;
    attempt2.external_ref = Some(
        ExternalJobRef::new("ssh_cluster_box", "pid_40912", "commit_88ab")
            .expect("Valid external ref"),
    );

    attempt2.apply(ExecEvent::PreflightOk).unwrap();
    attempt2.apply(ExecEvent::SubmitStarted).unwrap();
    attempt2
        .apply(ExecEvent::SubmitAcked(
            attempt2.external_ref.clone().unwrap(),
        ))
        .unwrap();
    attempt2.apply(ExecEvent::ObservedRunning).unwrap();
    store.experiment_attempts().save_attempt(&attempt2).unwrap();

    // Attempt 2 verifies parent checkpoint readiness before starting training
    assert!(manifest1.validate_for_state_resume().is_ok());

    // Attempt 2 trains to Step 200 and promotes checkpoint at Generation 2
    let ckpt2_id = "ckpt_step_200";
    let storage_uri2 = "https://r2.cloudflarestorage.com/checkpoints/exp_01/ckpt_200";
    let manifest2 = make_valid_manifest(ckpt2_id, 200, storage_uri2);
    manifest2
        .validate_for_state_resume()
        .expect("Manifest 2 must be valid for state resume");

    let promo2 = store
        .experiment_attempts()
        .promote_checkpoint(experiment_id, 2, ckpt2_id, 200)
        .unwrap();
    assert_eq!(
        promo2,
        PromotionOutcome::Promoted {
            canonical_step: 200
        },
        "Generation 2 checkpoint promotion must succeed"
    );

    // 3. Verify Complete Attempt Lineage & DAG Continuity
    let all_attempts = store
        .experiment_attempts()
        .list_attempts_for_experiment(experiment_id)
        .unwrap();
    assert_eq!(all_attempts.len(), 2);

    assert_eq!(all_attempts[0].id, attempt1.id);
    assert_eq!(all_attempts[0].parent_checkpoint_id, None);
    assert_eq!(all_attempts[0].generation, 1);
    assert_eq!(all_attempts[0].backend, ComputeBackendKind::RunPod);

    assert_eq!(all_attempts[1].id, attempt2.id);
    assert_eq!(
        all_attempts[1].parent_checkpoint_id,
        Some(ckpt1_id.into()),
        "Attempt 2 must link back to Attempt 1's checkpoint"
    );
    assert_eq!(all_attempts[1].generation, 2);
    assert_eq!(all_attempts[1].backend, ComputeBackendKind::DedicatedSsh);
}

#[test]
fn test_g11_target_death_crash_recovery_and_zombie_writer_fencing() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let experiment_id = "exp_crash_recovery";
    let recipe_id = "recipe_lora_finetune";

    // 1. Attempt 1 is Running on Remote Pod at Generation 1
    let mut attempt1 =
        ExecutionAttempt::new(experiment_id, recipe_id, ComputeBackendKind::RunPod, None);
    attempt1.generation = 1;
    attempt1.exec_state = ExecState::Running;
    attempt1.resource_state = ResourceState::ReleaseRequested;
    store.experiment_attempts().save_attempt(&attempt1).unwrap();

    // 2. REMOTE TARGET CRASH / DAEMON RESTART PROTOCOL
    // reconcile_attempts_on_startup() must recover in-flight states to Uncertain fail-closed
    let reconciled_count = store
        .experiment_attempts()
        .reconcile_attempts_on_startup()
        .unwrap();
    assert_eq!(
        reconciled_count, 1,
        "Startup reconciliation must recover the crashed in-flight attempt"
    );

    let recovered1 = store
        .experiment_attempts()
        .get_attempt(&attempt1.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        recovered1.exec_state,
        ExecState::Uncertain,
        "Running attempt during crash must be safely marked Uncertain"
    );
    assert_eq!(
        recovered1.resource_state,
        ResourceState::Unknown,
        "ReleaseRequested resource during crash must be marked Unknown"
    );

    // 3. New Attempt 2 is launched at Generation 2 to take over
    let attempt2 =
        ExecutionAttempt::new(experiment_id, recipe_id, ComputeBackendKind::VastAi, None);
    store.experiment_attempts().save_attempt(&attempt2).unwrap();

    // Attempt 2 completes and promotes checkpoint at Generation 2
    let promo_att2 = store
        .experiment_attempts()
        .promote_checkpoint(experiment_id, 2, "ckpt_gen2", 150)
        .unwrap();
    assert_eq!(
        promo_att2,
        PromotionOutcome::Promoted {
            canonical_step: 150
        }
    );

    // 4. ZOMBIE WRITER FENCING (G11 Invariant):
    // Stale Attempt 1 (delayed network packet or rogue worker) attempts to promote its checkpoint at Generation 1
    let stale_promo = store
        .experiment_attempts()
        .promote_checkpoint(experiment_id, 1, "ckpt_stale_zombie", 80)
        .unwrap();

    assert_eq!(
        stale_promo,
        PromotionOutcome::StaleGeneration {
            actual_generation: 2
        },
        "Stale writer promotion must be fenced and rejected with StaleGeneration"
    );
}

#[test]
fn test_g11_incomplete_or_uncommitted_checkpoint_blocks_unsafe_resume() {
    let dummy_digest = "b".repeat(64);

    // Case A: Checkpoint is LocalStaged (not yet committed to protected storage)
    let uncommitted_manifest = TrainingCheckpointManifest::new(
        "ckpt_staged",
        50,
        0.5,
        "https://r2.cloudflarestorage.com/checkpoints/staged",
        dummy_digest.clone(),
    )
    .with_full_state(
        "meta-llama/Llama-3-8B@rev1",
        "torch2.3+unsloth",
        "epoch:0,idx:50",
    )
    .with_commit_status(CheckpointCommitStatus::LocalStaged) // Staged, NOT CommittedProtected
    .add_file(CheckpointFileEntry {
        path: "model.safetensors".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    .add_file(CheckpointFileEntry {
        path: "optimizer.pt".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    .add_file(CheckpointFileEntry {
        path: "scheduler.pt".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    .add_file(CheckpointFileEntry {
        path: "rng_state.pth".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    .add_file(CheckpointFileEntry {
        path: "trainer_state.json".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    });

    let res_staged = uncommitted_manifest.validate_for_state_resume();
    assert!(
        res_staged.is_err(),
        "Uncommitted LocalStaged checkpoint must not be admitted for state resume"
    );
    let err_msg = res_staged.unwrap_err().to_string();
    assert!(err_msg.contains("State resume requires a protected full-state checkpoint"));

    // Case B: Missing required trainer state file (missing optimizer.pt)
    let missing_opt_manifest = TrainingCheckpointManifest::new(
        "ckpt_no_opt",
        50,
        0.5,
        "https://r2.cloudflarestorage.com/checkpoints/no_opt",
        dummy_digest.clone(),
    )
    .with_full_state(
        "meta-llama/Llama-3-8B@rev1",
        "torch2.3+unsloth",
        "epoch:0,idx:50",
    )
    .with_commit_status(CheckpointCommitStatus::CommittedProtected)
    .add_file(CheckpointFileEntry {
        path: "model.safetensors".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    // optimizer.pt deliberately omitted!
    .add_file(CheckpointFileEntry {
        path: "scheduler.pt".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    .add_file(CheckpointFileEntry {
        path: "rng_state.pth".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    })
    .add_file(CheckpointFileEntry {
        path: "trainer_state.json".into(),
        size_bytes: 500,
        digest: dummy_digest.clone(),
        required_for_state_resume: true,
    });

    let res_missing = missing_opt_manifest.validate_for_state_resume();
    assert!(
        res_missing.is_err(),
        "Checkpoint missing optimizer.pt must fail state resume validation"
    );
    let err_msg = res_missing.unwrap_err().to_string();
    assert!(err_msg.contains("State resume currently requires an inventoried single-GPU Trainer"));

    // Case C: Directory traversal attempt in checkpoint file inventory
    let unsafe_path_manifest = TrainingCheckpointManifest::new(
        "ckpt_traversal",
        50,
        0.5,
        "https://r2.cloudflarestorage.com/checkpoints/traversal",
        dummy_digest.clone(),
    )
    .with_commit_status(CheckpointCommitStatus::CommittedProtected)
    .add_file(CheckpointFileEntry {
        path: "../../etc/passwd".into(),
        size_bytes: 100,
        digest: dummy_digest,
        required_for_state_resume: true,
    });

    let res_traversal = unsafe_path_manifest.validate();
    assert!(
        res_traversal.is_err(),
        "Path traversal entry in checkpoint manifest must fail validation"
    );
    let err_msg = res_traversal.unwrap_err().to_string();
    assert!(err_msg.contains("invalid, duplicate or unsafe file entries"));
}

#[test]
fn test_g11_idempotent_invocation_recording_fences_duplicate_ops() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));

    let attempt =
        ExecutionAttempt::new("exp_idemp", "recipe_sft", ComputeBackendKind::RunPod, None);
    store.experiment_attempts().save_attempt(&attempt).unwrap();

    let inv = Invocation {
        id: new_id("inv"),
        attempt_id: attempt.id.clone(),
        op: "runpod.create_pod".into(),
        idempotency_key: "idemp_key_unique_123".into(),
        argv_fingerprint: "sha256:fingerprint_abc".into(),
        outcome: InvocationOutcome::Ok,
    };

    // First recording succeeds
    store
        .experiment_attempts()
        .record_invocation(&inv)
        .expect("First invocation recording must succeed");

    // Second recording with identical idempotency_key must return Conflict
    let dup_res = store.experiment_attempts().record_invocation(&inv);
    assert!(
        matches!(dup_res, Err(DomainError::Conflict(_))),
        "Duplicate invocation key must return DomainError::Conflict"
    );
}
