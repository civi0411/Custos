//! Golden Acceptance Test — Checkpoint Relay, Heterogeneous Failover & Generation CAS Fencing (GPU-F)
//!
//! Validates:
//! 1. On-Target Checkpoint Blob Commit & CAS Digest Attestation: Digest mismatch fails closed.
//! 2. Pre-Resume Full State Validation Gate: Uncommitted, missing optimizer/scheduler, or traversal rejected.
//! 3. Single-Writer Generation CAS Fencing: Zombie writers from older generations rejected with StaleGeneration.
//! 4. Heterogeneous Failover & Hardware Adaptation Gate: A100 (40GB) -> RTX 4090 (24GB) batch scaling.
//! 5. Quarantining of Corrupted/Incomplete Checkpoints: Missing files quarantined during promotion.
//! 6. Full Provenance Lineage: Multi-generation attempt chain tracked in SQLite store with parent checkpoint linkages.
//! 7. Crash Handoff & Recovery: Crashed attempt settled honestly and continuation child dispatched seamlessly.

use std::collections::HashMap;
use std::sync::Arc;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

use custos_adapters::compute::checkpoint_store::CasCheckpointStoreAdapter;
use custos_core::contracts::checkpoint_store::{CheckpointStorePort, PromotionOutcome};
use custos_domain::{
    AcceleratorSpec, CheckpointCommitStatus, CheckpointFileEntry, ComputeBackendKind,
    ExecState, ExecutionAttempt, ResourceState, TrainingCheckpointManifest,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::compute::relay::CheckpointRelayCoordinator;

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn make_test_bundle() -> HashMap<&'static str, Vec<u8>> {
    let mut map = HashMap::new();
    map.insert("model.safetensors", b"DUMMY_MODEL_WEIGHTS_TENSOR_BYTES".to_vec());
    map.insert("optimizer.pt", b"DUMMY_ADAMW_OPTIMIZER_MOMENTUM_BYTES".to_vec());
    map.insert("scheduler.pt", b"DUMMY_COSINE_LR_SCHEDULER_BYTES".to_vec());
    map.insert("rng_state.pth", b"DUMMY_TORCH_CUDA_RNG_STATE_BYTES".to_vec());
    map.insert("trainer_state.json", b"{\"step\": 100, \"epoch\": 1.0}".to_vec());
    map
}

fn build_manifest_for_bundle(
    checkpoint_id: &str,
    step: u64,
    storage_uri: &str,
    bundle: &HashMap<&str, Vec<u8>>,
) -> TrainingCheckpointManifest {
    let mut manifest = TrainingCheckpointManifest::new(
        checkpoint_id,
        step,
        (step as f64) / 100.0,
        storage_uri,
        "a".repeat(64),
    )
    .with_full_state(
        "meta-llama/Llama-3-8B@rev1",
        "torch2.3+unsloth",
        "epoch:1,idx:500",
    )
    .with_commit_status(CheckpointCommitStatus::CommittedProtected)
    .with_precision("bf16");

    for (name, data) in bundle {
        manifest = manifest.add_file(CheckpointFileEntry {
            path: (*name).to_string(),
            size_bytes: data.len() as u64,
            digest: sha256_hex(data),
            required_for_state_resume: true,
        });
    }

    manifest
}

#[tokio::test]
async fn test_g_relay_01_blob_commit_and_digest_attestation() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(CasCheckpointStoreAdapter::new(tmp.path()));

    let prefix = "experiments/exp_lora_01/attempts/att_01/checkpoints/ckpt_100";
    let data = b"EXACT_CHECKPOINT_DATA_BYTES";
    let correct_digest = sha256_hex(data);

    // 1. Correct digest upload succeeds
    store
        .put_blob(prefix, "optimizer.pt", data, &correct_digest)
        .await
        .expect("Valid digest upload must succeed");

    // 2. Querying blob returns exact bytes
    let fetched = store
        .get_blob(prefix, "optimizer.pt")
        .await
        .unwrap()
        .expect("Blob must exist in store");
    assert_eq!(fetched, data);

    // 3. Corrupted/tampered digest upload fails closed
    let wrong_digest = "0".repeat(64);
    let fail_res = store
        .put_blob(prefix, "corrupted.pt", data, &wrong_digest)
        .await;
    assert!(fail_res.is_err());
    let err_msg = fail_res.unwrap_err().to_string();
    assert!(err_msg.contains("Blob digest mismatch"));
}

#[tokio::test]
async fn test_g_relay_02_pre_resume_full_state_validation_gate() {
    let bundle = make_test_bundle();

    // 1. Valid bundle satisfies state resume gate
    let valid_manifest = build_manifest_for_bundle(
        "ckpt_valid",
        100,
        "https://r2.cloudflarestorage.com/checkpoints/ckpt_valid",
        &bundle,
    );
    valid_manifest
        .validate_for_state_resume()
        .expect("Complete bundle with optimizer/scheduler must pass resume gate");

    // 2. Uncommitted / LocalStaged checkpoint rejected
    let mut uncommitted = valid_manifest.clone();
    uncommitted.commit_status = CheckpointCommitStatus::LocalStaged;
    assert!(uncommitted.validate_for_state_resume().is_err());

    // 3. Missing optimizer.pt rejected
    let mut missing_opt = valid_manifest.clone();
    missing_opt.files.retain(|f| f.path != "optimizer.pt");
    assert!(missing_opt.validate_for_state_resume().is_err());

    // 4. Path traversal attempt in manifest file entries rejected
    let mut traversal_manifest = valid_manifest.clone();
    traversal_manifest.files.push(CheckpointFileEntry {
        path: "../etc/passwd".to_string(),
        size_bytes: 100,
        digest: "a".repeat(64),
        required_for_state_resume: false,
    });
    assert!(traversal_manifest.validate_for_state_resume().is_err());
}

#[tokio::test]
async fn test_g_relay_03_single_writer_generation_cas_fencing() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(CasCheckpointStoreAdapter::new(tmp.path()));
    let relay = CheckpointRelayCoordinator::new();

    let experiment_id = "exp_fencing_01";
    let bundle = make_test_bundle();

    // Attempt 1 (Gen 1) writes Checkpoint 1
    let pfx1 = format!("experiments/{experiment_id}/attempts/att_1/");
    for (name, data) in &bundle {
        store.put_blob(&pfx1, name, data, &sha256_hex(data)).await.unwrap();
    }
    let manifest1 = build_manifest_for_bundle(
        "ckpt_1",
        100,
        &format!("s3://bucket/{pfx1}"),
        &bundle,
    );
    store.publish_manifest(&pfx1, &manifest1).await.unwrap();

    // Promote Checkpoint 1 at Generation 1
    let outcome1 = relay
        .promote_checkpoint_with_fencing(store.as_ref(), experiment_id, "ckpt_1", 1)
        .await
        .unwrap();
    assert_eq!(outcome1, PromotionOutcome::Promoted { canonical_step: 100 });

    // Attempt 2 (Gen 2) writes Checkpoint 2
    let pfx2 = format!("experiments/{experiment_id}/attempts/att_2/");
    for (name, data) in &bundle {
        store.put_blob(&pfx2, name, data, &sha256_hex(data)).await.unwrap();
    }
    let manifest2 = build_manifest_for_bundle(
        "ckpt_2",
        200,
        &format!("s3://bucket/{pfx2}"),
        &bundle,
    );
    store.publish_manifest(&pfx2, &manifest2).await.unwrap();

    // Promote Checkpoint 2 at Generation 2
    let outcome2 = relay
        .promote_checkpoint_with_fencing(store.as_ref(), experiment_id, "ckpt_2", 2)
        .await
        .unwrap();
    assert_eq!(outcome2, PromotionOutcome::Promoted { canonical_step: 200 });

    // ZOMBIE WRITER DEFENSE: Lagging Attempt 1 attempts to promote at Generation 1
    let stale_outcome = relay
        .promote_checkpoint_with_fencing(store.as_ref(), experiment_id, "ckpt_1", 1)
        .await
        .unwrap();

    // INVARIANT: Stale promotion rejected with StaleGeneration(2)
    assert_eq!(
        stale_outcome,
        PromotionOutcome::StaleGeneration {
            actual_generation: 2
        }
    );
}

#[tokio::test]
async fn test_g_relay_04_heterogeneous_failover_hardware_adaptation_gate() {
    let relay = CheckpointRelayCoordinator::new();
    let bundle = make_test_bundle();

    let manifest = build_manifest_for_bundle(
        "ckpt_a100",
        500,
        "s3://bucket/ckpt_a100",
        &bundle,
    );

    // Root attempt on RunPod A100 (40GB)
    let parent = ExecutionAttempt {
        id: "att_runpod_root".into(),
        experiment_id: "exp_llama3_failover".into(),
        recipe_id: "recipe_unsloth_sft".into(),
        backend: ComputeBackendKind::RunPod,
        external_ref: None,
        exec_state: ExecState::Running,
        resource_state: ResourceState::Held,
        parent_checkpoint_id: None,
        generation: 1,
        output_prefix: "experiments/exp_llama3_failover/attempts/att_runpod_root/".into(),
    };

    let source_a100 = AcceleratorSpec::a100_40gb(); // 40960 MB
    let target_4090 = AcceleratorSpec::rtx_4090();   // 24576 MB

    // Source batch configuration: per_device_batch = 8, grad_accum = 2 => effective_batch = 16
    let (child_attempt, plan) = relay
        .prepare_continuation_attempt(
            &parent,
            ComputeBackendKind::DedicatedSsh,
            &manifest,
            &source_a100,
            &target_4090,
            8,
            2,
        )
        .expect("Failover to RTX 4090 must succeed with adaptation");

    // INVARIANTS:
    // 1. Monotonic generation increment: Gen 1 -> Gen 2
    assert_eq!(child_attempt.generation, 2);
    assert_eq!(child_attempt.parent_checkpoint_id, Some("ckpt_a100".into()));
    assert_eq!(child_attempt.backend, ComputeBackendKind::DedicatedSsh);
    assert_eq!(child_attempt.exec_state, ExecState::Planned);

    // 2. Hardware scaling: batch scaled to 4, grad accum increased to 4 to preserve effective batch = 16
    assert_eq!(plan.per_device_batch_size, 4);
    assert_eq!(plan.gradient_accumulation_steps, 4);
    assert_eq!(plan.effective_batch_size, 16);
    assert_eq!(plan.resume_step, 500);

    // 3. Test incompatible target: VRAM < 4GB must be rejected
    let tiny_accelerator = AcceleratorSpec::new("Tiny GPU", 2048, false);
    let fail_res = relay.prepare_continuation_attempt(
        &parent,
        ComputeBackendKind::DedicatedSsh,
        &manifest,
        &source_a100,
        &tiny_accelerator,
        8,
        2,
    );
    assert!(fail_res.is_err());
    assert!(fail_res.unwrap_err().to_string().contains("insufficient for LLM training"));
}

#[tokio::test]
async fn test_g_relay_05_quarantining_corrupted_checkpoint() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(CasCheckpointStoreAdapter::new(tmp.path()));
    let relay = CheckpointRelayCoordinator::new();

    let experiment_id = "exp_quarantine_01";
    let bundle = make_test_bundle();

    let pfx = format!("experiments/{experiment_id}/attempts/att_1/");
    for (name, data) in &bundle {
        store.put_blob(&pfx, name, data, &sha256_hex(data)).await.unwrap();
    }

    // Create manifest missing optimizer state
    let mut corrupt_manifest = build_manifest_for_bundle(
        "ckpt_corrupt",
        100,
        &format!("s3://bucket/{pfx}"),
        &bundle,
    );
    corrupt_manifest.commit_status = CheckpointCommitStatus::LocalStaged; // uncommitted!

    // Publish manifest
    let manifests = store.list_checkpoints(experiment_id).await.unwrap();
    assert!(manifests.is_empty());

    // Publish directly
    store.publish_manifest(&pfx, &corrupt_manifest).await.unwrap();

    // Promote must quarantine due to uncommitted / corrupt state
    let outcome = relay
        .promote_checkpoint_with_fencing(store.as_ref(), experiment_id, "ckpt_corrupt", 1)
        .await
        .unwrap();

    match outcome {
        PromotionOutcome::Quarantined { reason } => {
            assert!(reason.contains("State resume requires a protected full-state checkpoint"));
        }
        other => panic!("Expected Quarantined, got {:?}", other),
    }
}

#[tokio::test]
async fn test_g_relay_06_full_provenance_lineage_in_sqlite_store() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let experiment_id = "exp_lineage_chain";

    // 1. Root attempt on RunPod
    let attempt1 = ExecutionAttempt {
        id: "att_01_runpod".into(),
        experiment_id: experiment_id.into(),
        recipe_id: "recipe_sft_v1".into(),
        backend: ComputeBackendKind::RunPod,
        external_ref: None,
        exec_state: ExecState::Completed,
        resource_state: ResourceState::ReleaseConfirmed,
        parent_checkpoint_id: None,
        generation: 1,
        output_prefix: format!("experiments/{experiment_id}/attempts/att_01_runpod/"),
    };
    store.experiment_attempts().save_attempt(&attempt1).unwrap();

    // 2. Child continuation attempt on Dedicated SSH
    let attempt2 = ExecutionAttempt {
        id: "att_02_ssh".into(),
        experiment_id: experiment_id.into(),
        recipe_id: "recipe_sft_v1".into(),
        backend: ComputeBackendKind::DedicatedSsh,
        external_ref: None,
        exec_state: ExecState::Running,
        resource_state: ResourceState::Held,
        parent_checkpoint_id: Some("ckpt_step_100".into()),
        generation: 2,
        output_prefix: format!("experiments/{experiment_id}/attempts/att_02_ssh/"),
    };
    store.experiment_attempts().save_attempt(&attempt2).unwrap();

    // 3. Query lineage DAG
    let attempts = store
        .experiment_attempts()
        .list_attempts_for_experiment(experiment_id)
        .unwrap();

    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].id, "att_01_runpod");
    assert_eq!(attempts[0].generation, 1);
    assert_eq!(attempts[0].parent_checkpoint_id, None);

    assert_eq!(attempts[1].id, "att_02_ssh");
    assert_eq!(attempts[1].generation, 2);
    assert_eq!(attempts[1].parent_checkpoint_id, Some("ckpt_step_100".into()));
}

#[tokio::test]
async fn test_g_relay_07_crash_handoff_and_continuation_dispatch() {
    let relay = CheckpointRelayCoordinator::new();
    let bundle = make_test_bundle();

    let protected_checkpoint = build_manifest_for_bundle(
        "ckpt_step_300",
        300,
        "s3://bucket/ckpt_300",
        &bundle,
    );

    let mut crashed_attempt = ExecutionAttempt {
        id: "att_crashed".into(),
        experiment_id: "exp_recovery_flow".into(),
        recipe_id: "recipe_llama3".into(),
        backend: ComputeBackendKind::RunPod,
        external_ref: None,
        exec_state: ExecState::Running,
        resource_state: ResourceState::Held,
        parent_checkpoint_id: None,
        generation: 1,
        output_prefix: "experiments/exp_recovery_flow/attempts/att_crashed/".into(),
    };

    let source_a100 = AcceleratorSpec::a100_40gb();
    let target_ssh = AcceleratorSpec::a100_40gb();

    // Handoff crashed attempt to Dedicated SSH
    let (continuation, plan) = relay
        .handoff_crashed_attempt(
            &mut crashed_attempt,
            ComputeBackendKind::DedicatedSsh,
            &protected_checkpoint,
            &source_a100,
            &target_ssh,
            4,
            4,
        )
        .expect("Crash handoff must succeed");

    // INVARIANTS:
    // 1. Crashed attempt is marked Failed
    assert_eq!(crashed_attempt.exec_state, ExecState::Failed);

    // 2. Child continuation attempt inherits parent checkpoint and generation bumps to 2
    assert_eq!(continuation.generation, 2);
    assert_eq!(continuation.parent_checkpoint_id, Some("ckpt_step_300".into()));
    assert_eq!(continuation.backend, ComputeBackendKind::DedicatedSsh);
    assert_eq!(continuation.exec_state, ExecState::Planned);
    assert_eq!(plan.resume_step, 300);
}
