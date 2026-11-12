//! Golden Acceptance Test — Remote Compute Targets, Decoupled Lifecycle & Reconciliation (GPU-E)
//!
//! Validates:
//! 1. Remote Target Preflight & Capability Gate: Blockers reject before allocating compute.
//! 2. Idempotent Submission & Anti-Dual-Launch: Exclusive admission prevents overlapping runs on same remote device.
//! 3. Telemetry & Monotonic Progress Observation: ProgressTriplet strictly validates protected <= local <= observed.
//! 4. Decoupled Stop vs. Release: stop_job halts execution (Held, cost continues), release_compute terminates pod (ReleaseConfirmed).
//! 5. Disconnection & Startup Crash Recovery: Remote probe drop becomes Uncertain; startup sweep reconciles safely.
//! 6. Direct Teardown Safety: Calling release_compute on running job guarantees process termination before pod deletion.
//! 7. Artifact Collection & CAS Verification: Remote outputs downloaded and verified via SHA-256 digests.

use std::sync::Arc;
use tempfile::tempdir;

use custos_adapters::compute::rented_gpu::RentedGpuTargetAdapter;
use custos_domain::{
    ComputeBackendKind, ComputeDeviceRef, DomainError, ExecState, ExecutionAttempt,
    ProgressTriplet, ResourceState,
};
use custos_persistence::SqliteTaskStore;
use custos_runtime::compute::{RemoteExecutionCoordinator, WorkloadAdmissionCoordinator};

#[tokio::test]
async fn test_g_remote_01_preflight_and_capability_gate() {
    let tmp = tempdir().unwrap();
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "runpod-a100.cluster",
            22,
            "root",
            ComputeBackendKind::RunPod,
            tmp.path(),
        )
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission, adapter.clone());

    // 1. Verify target capabilities
    let caps = coordinator.capabilities().await;
    assert!(caps.supports_interactive_control);
    assert!(caps.supports_checkpoint_request);
    assert!(caps.supports_live_telemetry_streaming);
    assert!(caps.supports_graceful_stop);

    // 2. Normal preflight passes
    let report = coordinator.preflight("recipe_qwen2_lora").await.unwrap();
    assert!(report.allowed);
    assert!(report.warnings[0].contains("A100"));

    // 3. Preflight with blockers rejects admission upfront
    adapter
        .set_mock_preflight_blockers(vec![
            "Insufficient remote VRAM: requested 40GB, available 16GB".into(),
        ])
        .await;

    let mut attempt = ExecutionAttempt::new(
        "exp_preflight_fail",
        "recipe_qwen2_lora",
        ComputeBackendKind::RunPod,
        None,
    );

    let submit_res = coordinator
        .admit_and_submit(&mut attempt, None, "idem_preflight_fail")
        .await;
    assert!(submit_res.is_err());
    let err_msg = submit_res.unwrap_err().to_string();
    assert!(err_msg.contains("Remote preflight blocked"));
    assert!(err_msg.contains("Insufficient remote VRAM"));
    assert_eq!(attempt.exec_state, ExecState::Planned);
}

#[tokio::test]
async fn test_g_remote_02_idempotent_submit_and_anti_dual_launch() {
    let tmp = tempdir().unwrap();
    let remote_dev = ComputeDeviceRef::cuda(
        0,
        "NVIDIA A100-SXM4-80GB (RunPod)",
        81920,
        Some("GPU-REMOTE-A100-UUID".into()),
    );
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        remote_dev.clone(),
    ]));
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "runpod-a100-node1",
            22,
            "root",
            ComputeBackendKind::RunPod,
            tmp.path(),
        )
        .with_pod_lease("pod_runpod_exclusive", Some("secret_key".into()))
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission.clone(), adapter.clone());

    // 1. First attempt admitted and submitted
    let mut attempt1 = ExecutionAttempt::new(
        "exp_runpod_01",
        "recipe_llama3_sft",
        ComputeBackendKind::RunPod,
        None,
    );
    let ext_ref1 = coordinator
        .admit_and_submit(
            &mut attempt1,
            Some(remote_dev.device_id.clone()),
            "idem_submit_1",
        )
        .await
        .expect("Attempt 1 must be admitted and submitted");

    assert_eq!(ext_ref1.account_ref, "runpod-a100-node1");
    assert_eq!(ext_ref1.job_id, attempt1.id);
    assert_eq!(attempt1.exec_state, ExecState::Submitted);
    assert_eq!(attempt1.resource_state, ResourceState::Held);

    // 2. Competing attempt requesting exclusive access on same remote accelerator must fail closed
    let mut attempt2 = ExecutionAttempt::new(
        "exp_runpod_02",
        "recipe_llama3_sft",
        ComputeBackendKind::RunPod,
        None,
    );
    let submit2_res = coordinator
        .admit_and_submit(
            &mut attempt2,
            Some(remote_dev.device_id.clone()),
            "idem_submit_2",
        )
        .await;

    assert!(submit2_res.is_err());
    match submit2_res.unwrap_err() {
        DomainError::Conflict(reason) => {
            assert!(reason.contains("queued at position 1"));
        }
        other => panic!("Expected Conflict, got {:?}", other),
    }

    // Clean up attempt 1
    coordinator.release_compute(&mut attempt1).await.unwrap();
    assert_eq!(attempt1.resource_state, ResourceState::ReleaseConfirmed);
}

#[tokio::test]
async fn test_g_remote_03_telemetry_and_monotonic_progress() {
    let tmp = tempdir().unwrap();
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "ssh-vastai.cloud",
            2222,
            "root",
            ComputeBackendKind::VastAi,
            tmp.path(),
        )
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission, adapter.clone());

    let mut attempt = ExecutionAttempt::new(
        "exp_telemetry",
        "recipe_phi3_mini",
        ComputeBackendKind::VastAi,
        None,
    );
    let _ext_ref = coordinator
        .admit_and_submit(&mut attempt, None, "idem_tel_1")
        .await
        .unwrap();

    // 1. Initial observation: running
    adapter.set_mock_status("running").await;
    let triplet = ProgressTriplet {
        observed: Some(100),
        local: Some(80),
        protected: Some(50),
    };
    triplet
        .validate()
        .expect("Protected <= Local <= Observed must hold");
    adapter.set_mock_progress(triplet).await;

    let obs = coordinator
        .observe_and_reconcile(&mut attempt)
        .await
        .unwrap();
    assert_eq!(obs.exec_state, ExecState::Running);
    assert_eq!(attempt.exec_state, ExecState::Running);
    assert_eq!(obs.progress.observed, Some(100));
    assert_eq!(obs.progress.local, Some(80));
    assert_eq!(obs.progress.protected, Some(50));
    assert_eq!(obs.progress.recovery_point(), Some(50));

    // 2. Progression to step 200
    let triplet2 = ProgressTriplet {
        observed: Some(200),
        local: Some(200),
        protected: Some(150),
    };
    triplet2.validate().unwrap();
    adapter.set_mock_progress(triplet2).await;

    let obs2 = coordinator
        .observe_and_reconcile(&mut attempt)
        .await
        .unwrap();
    assert_eq!(obs2.progress.observed, Some(200));
    assert_eq!(obs2.progress.recovery_point(), Some(150));

    // 3. Invalid triplet (producer lie: protected > observed) must fail validation
    let bad_triplet = ProgressTriplet {
        observed: Some(50),
        local: Some(100),
        protected: Some(120),
    };
    assert!(bad_triplet.validate().is_err());

    // Clean up
    coordinator.release_compute(&mut attempt).await.unwrap();
}

#[tokio::test]
async fn test_g_remote_04_decoupled_stop_vs_release_billing_protection() {
    let tmp = tempdir().unwrap();
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "runpod-cloud-inst",
            10022,
            "root",
            ComputeBackendKind::RunPod,
            tmp.path(),
        )
        .with_pod_lease("pod_cost_01", Some("api_key".into()))
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission, adapter.clone());

    let mut attempt = ExecutionAttempt::new(
        "exp_billing_guard",
        "recipe_mistral_instruct",
        ComputeBackendKind::RunPod,
        None,
    );
    coordinator
        .admit_and_submit(&mut attempt, None, "idem_billing")
        .await
        .unwrap();

    assert_eq!(attempt.exec_state, ExecState::Submitted);
    assert_eq!(attempt.resource_state, ResourceState::Held);
    assert!(attempt.may_still_incur_cost());

    // 1. Calling stop_job terminates the training script
    coordinator.stop_job(&mut attempt).await.unwrap();
    assert_eq!(attempt.exec_state, ExecState::Stopped);

    // INVARIANT: ResourceState remains Held! (User may inspect files; pod billing keeps running)
    assert_eq!(attempt.resource_state, ResourceState::Held);
    assert!(attempt.may_still_incur_cost());
    assert!(!adapter.is_released().await);

    // 2. Calling release_compute officially deallocates pod from cloud provider
    coordinator.release_compute(&mut attempt).await.unwrap();
    assert_eq!(attempt.resource_state, ResourceState::ReleaseConfirmed);

    // INVARIANT: Cost is terminated
    assert!(!attempt.may_still_incur_cost());
    assert!(adapter.is_released().await);
}

#[tokio::test]
async fn test_g_remote_05_disconnection_and_startup_crash_reconciliation() {
    let tmp = tempdir().unwrap();
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "ssh-dedicated.company.internal",
            22,
            "ubuntu",
            ComputeBackendKind::DedicatedSsh,
            tmp.path(),
        )
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission, adapter.clone());

    let mut attempt = ExecutionAttempt::new(
        "exp_crash_recon",
        "recipe_deepseek_coder",
        ComputeBackendKind::DedicatedSsh,
        None,
    );
    coordinator
        .admit_and_submit(&mut attempt, None, "idem_recon_1")
        .await
        .unwrap();

    // Mark running and persist
    adapter.set_mock_status("running").await;
    let _ = coordinator
        .observe_and_reconcile(&mut attempt)
        .await
        .unwrap();
    assert_eq!(attempt.exec_state, ExecState::Running);
    store.experiment_attempts().save_attempt(&attempt).unwrap();

    // 1. Simulate network failure / connection lost
    adapter.set_mock_status("unknown_status_network_drop").await;
    let obs_lost = coordinator
        .observe_and_reconcile(&mut attempt)
        .await
        .unwrap();
    assert_eq!(obs_lost.exec_state, ExecState::Uncertain);
    assert_eq!(attempt.exec_state, ExecState::Uncertain);
    store.experiment_attempts().save_attempt(&attempt).unwrap();

    // 2. Simulate daemon restart while attempt was running/submitting:
    // Startup recovery reconciles all active in-flight attempts to Uncertain
    let mut active_attempt = ExecutionAttempt::new(
        "exp_inflight",
        "recipe_phi",
        ComputeBackendKind::DedicatedSsh,
        None,
    );
    active_attempt.exec_state = ExecState::Running;
    active_attempt.resource_state = ResourceState::ReleaseRequested;
    store
        .experiment_attempts()
        .save_attempt(&active_attempt)
        .unwrap();

    let reconciled_count = store
        .experiment_attempts()
        .reconcile_attempts_on_startup()
        .unwrap();
    assert!(reconciled_count >= 1);

    let reloaded = store
        .experiment_attempts()
        .get_attempt(&active_attempt.id)
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.exec_state, ExecState::Uncertain);
    assert_eq!(reloaded.resource_state, ResourceState::Unknown);

    // 3. Network connection recovers and job finishes
    adapter.set_mock_status("completed").await;
    let obs_recovered = coordinator
        .observe_and_reconcile(&mut attempt)
        .await
        .unwrap();
    assert_eq!(obs_recovered.exec_state, ExecState::Completed);
    assert_eq!(attempt.exec_state, ExecState::Completed);

    coordinator.release_compute(&mut attempt).await.unwrap();
}

#[tokio::test]
async fn test_g_remote_06_direct_teardown_without_prior_stop() {
    let tmp = tempdir().unwrap();
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "runpod-direct-release",
            22,
            "root",
            ComputeBackendKind::RunPod,
            tmp.path(),
        )
        .with_pod_lease("pod_direct_01", Some("token".into()))
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission, adapter.clone());

    let mut attempt = ExecutionAttempt::new(
        "exp_direct_teardown",
        "recipe_test",
        ComputeBackendKind::RunPod,
        None,
    );
    coordinator
        .admit_and_submit(&mut attempt, None, "idem_direct")
        .await
        .unwrap();

    // Directly calling release_compute without calling stop_job first:
    // Must safely terminate the remote process and then deallocate the pod
    coordinator.release_compute(&mut attempt).await.unwrap();
    assert_eq!(attempt.resource_state, ResourceState::ReleaseConfirmed);
    assert!(adapter.is_released().await);
    assert!(!attempt.may_still_incur_cost());
}

#[tokio::test]
async fn test_g_remote_07_artifact_collection_and_cas_verification() {
    let tmp = tempdir().unwrap();
    let admission = Arc::new(WorkloadAdmissionCoordinator::new());
    let adapter = Arc::new(
        RentedGpuTargetAdapter::new(
            "ssh-vastai-artifact",
            22,
            "root",
            ComputeBackendKind::VastAi,
            tmp.path(),
        )
        .with_mock_mode(),
    );

    let coordinator = RemoteExecutionCoordinator::new(admission, adapter.clone());

    let mut attempt = ExecutionAttempt::new(
        "exp_artifact_download",
        "recipe_bert_classifier",
        ComputeBackendKind::VastAi,
        None,
    );
    coordinator
        .admit_and_submit(&mut attempt, None, "idem_artifact")
        .await
        .unwrap();

    // 1. Progress to completed state on remote target
    adapter.set_mock_status("completed").await;
    let obs = coordinator
        .observe_and_reconcile(&mut attempt)
        .await
        .unwrap();
    assert_eq!(obs.exec_state, ExecState::Completed);
    assert_eq!(attempt.exec_state, ExecState::Completed);

    // 2. Collect artifacts from completed run
    let artifacts = coordinator.collect_artifacts(&mut attempt).await.unwrap();
    assert!(!artifacts.is_empty());
    assert_eq!(attempt.exec_state, ExecState::Collected);

    // Verify SHA-256 digest format
    for artifact in &artifacts {
        assert_eq!(artifact.content_hash.len(), 64);
        assert!(artifact.size_bytes > 0);
        assert!(artifact.path.contains("train.log"));
    }

    coordinator.release_compute(&mut attempt).await.unwrap();
}
