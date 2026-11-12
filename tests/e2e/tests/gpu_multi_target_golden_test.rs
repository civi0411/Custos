//! Golden Acceptance Test — Multi-target Profiles & Optional Consumers (GPU-G)
//!
//! Validates:
//! 1. Google Colab execution lifecycle: interactive control, graceful stop vs. release decoupling.
//! 2. Colab compute units preservation: stopped kernel retains held state; explicit release unassigns VM.
//! 3. Kaggle Kernels batch lifecycle: non-interactive batch mode, version pinning (strictly never 'latest').
//! 4. Kaggle artifact collection and SHA-256 digest attestation.
//! 5. Multi-target admission coordination via RemoteExecutionCoordinator (Colab & Kaggle).
//! 6. Optional consumer placement & cooperative CPU fallback: active training run forces background embedding/OCR to CPU.
//! 7. Data privacy / cloud egress guardrail & cooperative priority preemption.

use std::sync::Arc;
use tempfile::tempdir;

use custos_adapters::compute::{ColabTargetAdapter, KaggleTargetAdapter};
use custos_core::contracts::compute::{ExecutionTargetPort, WorkloadAdmissionPort};
use custos_domain::{
    ComputeBackendKind, ComputeDeviceRef, ComputeWorkloadKind, ExecState, ExecutionAttempt,
    ExecutionLane, ExternalJobRef, ResourceState, WorkloadAdmissionRequest, WorkloadAdmissionVerdict,
    WorkloadLease, WorkloadLeaseStatus,
};
use custos_runtime::compute::{
    OptionalConsumerPlacementEngine, RemoteExecutionCoordinator, WorkloadAdmissionCoordinator,
};

#[tokio::test]
async fn test_g_multi_01_colab_interactive_lifecycle_and_capabilities() {
    let tmp = tempdir().unwrap();
    let adapter = ColabTargetAdapter::new(tmp.path()).with_mock_mode();

    // 1. Verify capabilities conform to Colab interactive runtime
    let caps = adapter.capabilities().await;
    assert!(caps.supports_interactive_control);
    assert!(caps.supports_checkpoint_request);
    assert!(caps.supports_graceful_stop);

    // 2. Preflight verification
    let report = adapter.preflight("recipe_colab_bert").await.unwrap();
    assert!(report.allowed);
    assert!(report.blockers.is_empty());

    // 3. Submit attempt
    let attempt = ExecutionAttempt::new(
        "exp_colab_01",
        "recipe_colab_bert",
        ComputeBackendKind::Colab,
        None,
    );
    let job_ref = adapter.submit(&attempt, "idem_colab_01").await.unwrap();
    assert_eq!(job_ref.job_id, "colab-exp-colab-01");
    assert_eq!(job_ref.version_id, "v1");

    // 4. Staged entrypoint check in live mode (unstaged script fails closed)
    let live_adapter = ColabTargetAdapter::new(tmp.path());
    let attempt_unstaged = ExecutionAttempt::new(
        "exp_colab_unstaged",
        "recipe_colab_bert",
        ComputeBackendKind::Colab,
        None,
    );
    let res = live_adapter
        .submit(&attempt_unstaged, "idem_colab_02")
        .await;
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("does not exist; stage your script"));
}

#[tokio::test]
async fn test_g_multi_02_colab_decoupled_stop_and_release_billing_protection() {
    let tmp = tempdir().unwrap();
    let adapter = ColabTargetAdapter::new(tmp.path()).with_mock_mode();
    let job_ref = ExternalJobRef::new("colab_user", "custos-exp-colab-billing", "1").unwrap();

    // 1. Initially Running with ResourceState::Held
    let obs = adapter.observe(&job_ref).await.unwrap();
    assert_eq!(obs.exec_state, ExecState::Running);
    assert_eq!(obs.resource_state, ResourceState::Held);
    assert!(!adapter.is_resource_released().await);

    // 2. Stop job terminates execution but retains VM allocation for log inspection
    adapter.stop_job(&job_ref).await.unwrap();
    let obs_stopped = adapter.observe(&job_ref).await.unwrap();
    assert_eq!(obs_stopped.exec_state, ExecState::Stopped);
    assert_eq!(obs_stopped.resource_state, ResourceState::Held);
    assert!(!adapter.is_resource_released().await);

    // 3. Explicit release deallocates VM/session, stopping compute unit burn
    adapter.release_compute(&job_ref).await.unwrap();
    assert!(adapter.is_resource_released().await);
    let obs_released = adapter.observe(&job_ref).await.unwrap();
    assert_eq!(obs_released.resource_state, ResourceState::ReleaseConfirmed);
}

#[tokio::test]
async fn test_g_multi_03_kaggle_batch_execution_and_version_pinning() {
    let tmp = tempdir().unwrap();
    let adapter = KaggleTargetAdapter::new(tmp.path()).with_mock_mode();

    // 1. Verify capabilities conform to Kaggle non-interactive batch mode
    let caps = adapter.capabilities().await;
    assert!(!caps.supports_interactive_control);
    assert!(!caps.supports_checkpoint_request);
    assert!(caps.supports_live_telemetry_streaming);

    // 2. Submit attempt and verify pinned version reference (never 'latest')
    let attempt = ExecutionAttempt::new(
        "exp_kaggle_01",
        "recipe_kaggle_resnet",
        ComputeBackendKind::Kaggle,
        None,
    );
    let job_ref = adapter.submit(&attempt, "idem_kaggle_01").await.unwrap();
    assert_eq!(job_ref.account_ref, "testuser");
    assert_eq!(job_ref.job_id, "custos-exp-kaggle-01");
    assert_eq!(job_ref.version_id, "1");
    assert_ne!(job_ref.version_id, "latest");

    // 3. Observe status mapping
    adapter.set_mock_status("running").await;
    let obs_running = adapter.observe(&job_ref).await.unwrap();
    assert_eq!(obs_running.exec_state, ExecState::Running);
    assert_eq!(obs_running.resource_state, ResourceState::Held);

    adapter.set_mock_status("complete").await;
    let obs_complete = adapter.observe(&job_ref).await.unwrap();
    assert_eq!(obs_complete.exec_state, ExecState::Completed);
    assert_eq!(obs_complete.resource_state, ResourceState::ReleaseConfirmed);

    adapter.set_mock_status("error").await;
    let obs_failed = adapter.observe(&job_ref).await.unwrap();
    assert_eq!(obs_failed.exec_state, ExecState::Failed);
    assert_eq!(obs_failed.resource_state, ResourceState::ReleaseConfirmed);
}

#[tokio::test]
async fn test_g_multi_04_kaggle_artifact_collection_and_cas_attestation() {
    let tmp = tempdir().unwrap();
    let adapter = KaggleTargetAdapter::new(tmp.path()).with_mock_mode();
    let job_ref = ExternalJobRef::new("testuser", "custos-exp-kaggle-artifacts", "1").unwrap();

    let artifacts = adapter.collect_artifacts(&job_ref).await.unwrap();
    assert!(!artifacts.is_empty());
    let art = &artifacts[0];
    assert!(art.path.ends_with("output.txt"));
    assert_eq!(art.content_hash.len(), 64);
    assert!(art.size_bytes > 0);
}

#[tokio::test]
async fn test_g_multi_05_remote_coordinator_multi_target_admission() {
    let tmp = tempdir().unwrap();
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let colab_adapter = Arc::new(ColabTargetAdapter::new(tmp.path()).with_mock_mode());
    let kaggle_adapter = Arc::new(KaggleTargetAdapter::new(tmp.path()).with_mock_mode());

    // Register virtual devices for Colab and Kaggle
    let colab_dev = ComputeDeviceRef::cuda(0, "Colab A100 GPU", 40960, Some("colab_dev_0".into()));
    let kaggle_dev =
        ComputeDeviceRef::cuda(1, "Kaggle P100 GPU", 16384, Some("kaggle_dev_0".into()));
    admission.register_device(colab_dev.clone()).await;
    admission.register_device(kaggle_dev.clone()).await;

    let colab_coord = RemoteExecutionCoordinator::new(
        admission.clone(),
        colab_adapter as Arc<dyn ExecutionTargetPort>,
    );
    let kaggle_coord = RemoteExecutionCoordinator::new(
        admission.clone(),
        kaggle_adapter as Arc<dyn ExecutionTargetPort>,
    );

    // 1. Submit Colab run
    let mut attempt_colab = ExecutionAttempt::new(
        "exp_colab_admit",
        "rec_1",
        ComputeBackendKind::Colab,
        Some(colab_dev.device_id.clone()),
    );
    let colab_job = colab_coord
        .admit_and_submit(
            &mut attempt_colab,
            Some(colab_dev.device_id.clone()),
            "idem_colab_admit",
        )
        .await
        .unwrap();
    assert_eq!(colab_job.job_id, "colab-exp-colab-admit");
    assert_eq!(attempt_colab.exec_state, ExecState::Submitted);

    // 2. Submit Kaggle run on separate target concurrently succeeds
    let mut attempt_kaggle = ExecutionAttempt::new(
        "exp_kaggle_admit",
        "rec_2",
        ComputeBackendKind::Kaggle,
        Some(kaggle_dev.device_id.clone()),
    );
    let kaggle_job = kaggle_coord
        .admit_and_submit(
            &mut attempt_kaggle,
            Some(kaggle_dev.device_id.clone()),
            "idem_kaggle_admit",
        )
        .await
        .unwrap();
    assert_eq!(kaggle_job.job_id, "custos-exp-kaggle-admit");
    assert_eq!(attempt_kaggle.exec_state, ExecState::Submitted);

    // 3. Attempting to dual-launch another exclusive job on Colab device is queued (anti-dual-launch)
    let req_duplicate = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "competing_run".into(),
        preferred_device_id: Some(colab_dev.device_id.clone()),
        require_exclusive: true,
        required_memory_mb: Some(4096),
        lane: None,
    };
    let verdict = admission.admit_workload(&req_duplicate).await.unwrap();
    assert!(matches!(verdict, WorkloadAdmissionVerdict::Queued { .. }));
}

#[tokio::test]
async fn test_g_multi_06_optional_consumer_cooperative_cpu_fallback() {
    let engine = OptionalConsumerPlacementEngine::new();

    let cpu = ComputeDeviceRef::cpu();
    let gpu = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-1".into()));
    let devices = vec![cpu.clone(), gpu.clone()];

    // Case A: GPU is free (no leases) -> background embedding admitted on GPU with batch 64
    let free_leases: Vec<WorkloadLease> = vec![];
    let placement_free = engine
        .evaluate_placement(
            ComputeWorkloadKind::BackgroundEmbedding,
            Some(&gpu.device_id),
            &free_leases,
            &devices,
            false,
            5000,
        )
        .unwrap();

    assert_eq!(placement_free.target_device_id, gpu.device_id);
    assert!(placement_free.is_accelerated);
    assert_eq!(placement_free.batch_size, 64);
    assert!(placement_free
        .reason
        .contains("admitted for cooperative background indexing"));

    // Case B: GPU is held by an exclusive training run -> background OCR routed to CPU fallback
    let training_lease = WorkloadLease {
        lease_id: "lease_train_active".into(),
        device_id: gpu.device_id.clone(),
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "exp_active_finetuning".into(),
        is_exclusive: true,
        status: WorkloadLeaseStatus::Active,
        acquired_at: 100,
        released_at: None,
        lane: ExecutionLane::Batch,
    };
    let active_leases = vec![training_lease];

    let placement_busy = engine
        .evaluate_placement(
            ComputeWorkloadKind::DocumentOcr,
            Some(&gpu.device_id),
            &active_leases,
            &devices,
            false,
            200,
        )
        .unwrap();

    assert_eq!(placement_busy.target_device_id, cpu.device_id);
    assert!(!placement_busy.is_accelerated);
    assert_eq!(placement_busy.batch_size, 16);
    assert!(placement_busy
        .reason
        .contains("held exclusively by active training run; routed to CPU fallback"));
}

#[tokio::test]
async fn test_g_multi_07_cloud_egress_guardrail_and_priority_preemption() {
    let engine = OptionalConsumerPlacementEngine::new();
    let cpu = ComputeDeviceRef::cpu();
    let devices = vec![cpu.clone()];
    let empty_leases: Vec<WorkloadLease> = vec![];

    // 1. Cloud egress guardrail: requesting a cloud target without allow_cloud_egress fails closed
    let egress_err = engine.evaluate_placement(
        ComputeWorkloadKind::BackgroundEmbedding,
        Some("colab_remote_target"),
        &empty_leases,
        &devices,
        false, // prohibited
        1000,
    );
    assert!(egress_err.is_err());
    assert!(egress_err
        .unwrap_err()
        .to_string()
        .contains("Cloud egress prohibited for local dataset indexing"));

    // 2. Non-background workload passed to placement engine fails closed
    let non_bg_err = engine.evaluate_placement(
        ComputeWorkloadKind::TrainingRun,
        None,
        &empty_leases,
        &devices,
        false,
        1,
    );
    assert!(non_bg_err.is_err());
    assert!(non_bg_err
        .unwrap_err()
        .to_string()
        .contains("Only cooperative background workloads"));

    // 3. Priority preemption evaluation:
    // Background lease can be preempted by interactive notebook or training run, but not by another background task
    let bg_lease = WorkloadLease {
        lease_id: "lease_bg".into(),
        device_id: "dev_gpu_0".into(),
        workload_kind: ComputeWorkloadKind::BackgroundEmbedding,
        owner_id: "doc_indexer".into(),
        is_exclusive: false,
        status: WorkloadLeaseStatus::Active,
        acquired_at: 100,
        released_at: None,
        lane: ExecutionLane::Ambient,
    };

    assert!(engine
        .can_preempt_background_for_interactive(&bg_lease, ComputeWorkloadKind::NotebookKernel));
    assert!(
        engine.can_preempt_background_for_interactive(&bg_lease, ComputeWorkloadKind::TrainingRun)
    );
    assert!(engine
        .can_preempt_background_for_interactive(&bg_lease, ComputeWorkloadKind::LocalInference));
    assert!(
        !engine.can_preempt_background_for_interactive(&bg_lease, ComputeWorkloadKind::DocumentOcr)
    );
    assert!(!engine.can_preempt_background_for_interactive(
        &bg_lease,
        ComputeWorkloadKind::BackgroundEmbedding
    ));
}
