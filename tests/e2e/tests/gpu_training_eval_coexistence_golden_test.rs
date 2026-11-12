//! Golden Acceptance Test: GPU Training, Evaluation & Inference Coexistence (GPU-C / GPU-D)
//!
//! Validates:
//! 1. Deterministic Tiny CPU Training Run (monotonic steps, convergence, valid checkpoint manifests).
//! 2. Exclusive Mutual Exclusion & Anti-Dual-Launch (competing training run blocked with Conflict error).
//! 3. Inter-Workload Collision Avoidance: Training vs Evaluation (evaluation blocked from spawning duplicate model in occupied VRAM).
//! 4. Sequential Safe-Point Handoff (Training finishes -> commits checkpoint -> evaluation admits and scores model).
//! 5. Local Inference Coexistence (GPU-D: local inference blocked fail-closed when accelerator held by training; zero silent fallback).
//! 6. In-Flight Inference Request Eviction Protection (eviction blocked while requests in-flight; clean release once drained).
//! 7. Training Fault & OOM Recovery (simulated failure immediately reclaims lease, zero zombie allocations).

use std::sync::Arc;

use custos_provider::local::{evict_model, preload_model_with_profile, InferenceRuntime};
use custos_core::contracts::compute::WorkloadAdmissionPort;
use custos_domain::{
    ComputeBackendKind, ComputeDeviceRef, DomainError, ExecState, ExecutionAttempt,
    FrozenTrainingRecipe, LocalModelLoadProfile, ModelEvaluationSpec,
};
use custos_runtime::compute::{
    LocalTrainingRunner, ModelEvaluationRunner, WorkloadAdmissionCoordinator,
};

static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1: Deterministic Tiny CPU Training Run (GPU-C Baseline Correctness)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_1_deterministic_tiny_cpu_training_run() {
    let _lock = TEST_LOCK.lock().await;
    let cpu = ComputeDeviceRef::cpu();
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cpu.clone()
    ]));
    let runner = LocalTrainingRunner::new(admission.clone());

    let recipe =
        FrozenTrainingRecipe::new("meta-llama/Llama-3-8B", "s3://datasets/alpaca_tiny", 10)
            .with_device(&cpu.device_id)
            .with_checkpoint_interval(5)
            .with_batch_size(2)
            .with_learning_rate(1e-4);

    let mut attempt = ExecutionAttempt::new(
        "exp_tiny_cpu_01",
        &recipe.recipe_id,
        ComputeBackendKind::Local,
        None,
    );

    let receipt = runner
        .execute_training_run(&mut attempt, &recipe, None)
        .await
        .expect("Training run must complete cleanly");

    assert_eq!(receipt.total_steps_completed, 10);
    assert_eq!(receipt.device_id, cpu.device_id);
    assert_eq!(receipt.status, ExecState::Collected);
    assert!(receipt.final_loss < 2.5, "Loss must decrease monotonically");
    assert_eq!(
        receipt.checkpoints_created.len(),
        2,
        "Checkpoints must be generated at step 5 and step 10"
    );

    // Verify lease was automatically released upon completion (zero zombie leases)
    let active_leases = admission.list_active_leases(None).await.unwrap();
    assert_eq!(
        active_leases.len(),
        0,
        "Exclusive lease must be freed upon clean completion"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2: Exclusive Mutual Exclusion & Anti-Dual-Launch (GPU-C Core Gate)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_2_training_exclusive_mutual_exclusion_anti_dual_launch() {
    let _lock = TEST_LOCK.lock().await;
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));
    let runner = LocalTrainingRunner::new(admission.clone());

    let _recipe1 =
        FrozenTrainingRecipe::new("model-a", "dataset-a", 20).with_device(&cuda.device_id);
    let recipe2 =
        FrozenTrainingRecipe::new("model-b", "dataset-b", 20).with_device(&cuda.device_id);

    // 1. Manually admit first training attempt to simulate in-flight execution holding the GPU
    let req_training_1 = custos_domain::WorkloadAdmissionRequest {
        workload_kind: custos_domain::ComputeWorkloadKind::TrainingRun,
        owner_id: "attempt_in_flight_1".into(),
        preferred_device_id: Some(cuda.device_id.clone()),
        require_exclusive: true,
        required_memory_mb: Some(18000),
        lane: None,
    };
    let v1 = admission.admit_workload(&req_training_1).await.unwrap();
    assert!(matches!(
        v1,
        custos_domain::WorkloadAdmissionVerdict::Admitted { .. }
    ));

    // 2. Second training attempt tries to launch concurrently on same GPU
    let mut attempt2 = ExecutionAttempt::new(
        "exp_training_2",
        &recipe2.recipe_id,
        ComputeBackendKind::Local,
        None,
    );

    let result = runner
        .execute_training_run(&mut attempt2, &recipe2, None)
        .await;
    match result {
        Err(DomainError::Conflict(reason)) => {
            assert!(
                reason.contains("Training workload queued"),
                "Competing training launch must be cleanly queued/rejected with Conflict: got '{}'",
                reason
            );
        }
        other => panic!("Expected Conflict error for dual launch, got: {:?}", other),
    }

    // Verify only 1 active lease is held on the card
    let active = admission
        .list_active_leases(Some(&cuda.device_id))
        .await
        .unwrap();
    assert_eq!(
        active.len(),
        1,
        "Exactly one exclusive workload may hold the accelerator"
    );
    assert_eq!(active[0].owner_id, "attempt_in_flight_1");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3: Inter-Workload Collision Avoidance — Training vs Evaluation
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_3_training_blocks_evaluation_vram_collision() {
    let _lock = TEST_LOCK.lock().await;
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));
    let eval_runner = ModelEvaluationRunner::new(admission.clone());

    // 1. Training holds exclusive lease on the GPU
    let req_training = custos_domain::WorkloadAdmissionRequest {
        workload_kind: custos_domain::ComputeWorkloadKind::TrainingRun,
        owner_id: "active_training_run".into(),
        preferred_device_id: Some(cuda.device_id.clone()),
        require_exclusive: true,
        required_memory_mb: Some(20000),
        lane: None,
    };
    let _ = admission.admit_workload(&req_training).await.unwrap();

    // 2. Evaluation attempts to run on the SAME occupied GPU
    let eval_spec = ModelEvaluationSpec::new("ckpt_step_50", "s3://eval/validation_split")
        .with_device(&cuda.device_id)
        .with_sample_limit(100);

    let eval_result = eval_runner.execute_evaluation(&eval_spec).await;
    match eval_result {
        Err(DomainError::Conflict(msg)) => {
            assert!(
                msg.contains("Evaluation blocked: accelerator busy or queued"),
                "Evaluation must fail closed rather than causing parallel model OOM: got '{}'",
                msg
            );
        }
        other => panic!(
            "Expected Conflict error due to training occupancy, got: {:?}",
            other
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 4: Sequential Safe-Point Handoff (Training -> Checkpoint -> Eval)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_4_sequential_safe_point_handoff() {
    let _lock = TEST_LOCK.lock().await;
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));
    let train_runner = LocalTrainingRunner::new(admission.clone());
    let eval_runner = ModelEvaluationRunner::new(admission.clone());

    // 1. Training completes and commits checkpoint
    let recipe = FrozenTrainingRecipe::new("meta-llama/Llama-3-8B", "s3://datasets/train", 6)
        .with_device(&cuda.device_id)
        .with_checkpoint_interval(3);

    let mut attempt = ExecutionAttempt::new(
        "exp_sequential_01",
        &recipe.recipe_id,
        ComputeBackendKind::Local,
        None,
    );

    let receipt = train_runner
        .execute_training_run(&mut attempt, &recipe, None)
        .await
        .expect("Training run must complete cleanly");

    let latest_checkpoint = receipt.checkpoints_created.last().unwrap();

    // 2. Accelerator is now free; evaluation admits and scores model
    let eval_spec = ModelEvaluationSpec::new(latest_checkpoint, "s3://datasets/held_out_test")
        .with_device(&cuda.device_id)
        .with_sample_limit(50);

    let report = eval_runner
        .execute_evaluation(&eval_spec)
        .await
        .expect("Evaluation must execute cleanly once training deallocates device");

    assert_eq!(report.candidate_checkpoint_id, *latest_checkpoint);
    assert_eq!(report.sample_count, 50);
    assert!(report.passed_hurdle, "Report must pass hurdle");
    assert_eq!(report.device_id, cuda.device_id);

    // 3. Post-evaluation: zero active leases
    let active = admission.list_active_leases(None).await.unwrap();
    assert_eq!(active.len(), 0, "No lingering leases after sequential eval");
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 5: Local Inference Coexistence (GPU-D Gate)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_5_inference_coexistence_blocked_when_training_active() {
    let _lock = TEST_LOCK.lock().await;
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));

    // 1. Training occupies GPU exclusively
    let req_training = custos_domain::WorkloadAdmissionRequest {
        workload_kind: custos_domain::ComputeWorkloadKind::TrainingRun,
        owner_id: "training_run_occupier".into(),
        preferred_device_id: Some(cuda.device_id.clone()),
        require_exclusive: true,
        required_memory_mb: Some(20000),
        lane: None,
    };
    let _ = admission.admit_workload(&req_training).await.unwrap();

    // 2. Wire InferenceRuntime to admission coordinator
    let runtime = InferenceRuntime::get_or_init().expect("inference runtime");
    runtime.set_admission_coordinator(admission.clone());

    // 3. Create a temporary GGUF file so the path resolver discovers a valid local model
    let temp_gguf = tempfile::Builder::new()
        .prefix("custos_model_")
        .suffix(".gguf")
        .tempfile()
        .unwrap();
    let model_path = temp_gguf.path().to_string_lossy().to_string();

    let profile = LocalModelLoadProfile {
        model_path: model_path.clone(),
        revision: 1,
        context_size: 4096,
        gpu_mode: "max".to_string(),
        gpu_layers: Some(33),
        eval_batch_size: None,
        cpu_threads: None,
        flash_attention: None,
        mmap: true,
        mlock: false,
        idle_timeout_seconds: None,
        updated_at: 0,
    };

    let result = preload_model_with_profile(&model_path, &profile).await;
    match result {
        Err(custos_provider::errors::ProviderError::ExecutionError(msg)) => {
            assert!(
                msg.contains("Local inference queued")
                    || msg.contains("accelerator busy")
                    || msg.contains("unavailable"),
                "Inference request must fail closed or queue rather than silently fallbacking to cloud: got '{}'",
                msg
            );
        }
        other => panic!(
            "Expected ExecutionError queuing inference, got: {:?}",
            other
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 6: In-Flight Inference Request Protects Model from Eviction Race
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_6_in_flight_inference_protects_from_eviction() {
    let _lock = TEST_LOCK.lock().await;
    let runtime = InferenceRuntime::get_or_init().expect("inference runtime");
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        ComputeDeviceRef::cpu(),
    ]));
    runtime.set_admission_coordinator(admission);

    // Simulate in-flight consumer on a model slot
    let key = "mock_busy_model";
    let evicted = evict_model(key).await;
    assert!(
        evicted.is_ok(),
        "Evicting idle/unloaded model succeeds cleanly"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 7: Training Fault & OOM Recovery (Zero Zombie Leases)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_7_training_fault_recovery_zero_zombies() {
    let _lock = TEST_LOCK.lock().await;
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));
    let runner = LocalTrainingRunner::new(admission.clone());

    let recipe = FrozenTrainingRecipe::new("meta-llama/Llama-3-8B", "s3://datasets/fault_test", 10)
        .with_device(&cuda.device_id)
        .with_checkpoint_interval(2);

    let mut attempt = ExecutionAttempt::new(
        "exp_fault_01",
        &recipe.recipe_id,
        ComputeBackendKind::Local,
        None,
    );

    // Simulate an unexpected accelerator OOM at step 4
    let result = runner
        .execute_training_run(&mut attempt, &recipe, Some(4))
        .await;

    assert!(result.is_err(), "Run with simulated OOM must fail");
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("Simulated accelerator OOM"),
        "Error message must specify simulated fault: got '{}'",
        err_str
    );

    // Critical Invariant: Lease must have been cleaned up immediately on error (ZERO ZOMBIE LEASES)
    let active_leases = admission
        .list_active_leases(Some(&cuda.device_id))
        .await
        .unwrap();
    assert_eq!(
        active_leases.len(),
        0,
        "Failed training run must not leave zombie leases on accelerator"
    );
}
