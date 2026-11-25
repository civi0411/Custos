use super::*;

#[tokio::test]
async fn rented_gpu_capabilities_and_preflight() {
    let tmp = tempfile::tempdir().unwrap();
    let adapter = RentedGpuTargetAdapter::new(
        "192.168.1.100",
        22,
        "root",
        ComputeBackendKind::DedicatedSsh,
        tmp.path(),
    )
    .with_mock_mode();

    let caps = adapter.capabilities().await;
    assert!(caps.supports_interactive_control);
    assert!(caps.supports_checkpoint_request);
    assert!(caps.supports_live_telemetry_streaming);

    let report = adapter.preflight("rec_gpu").await.unwrap();
    assert!(report.allowed);
    assert!(report.warnings[0].contains("A100"));
}

#[tokio::test]
async fn rented_gpu_submit_and_observe_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let adapter = RentedGpuTargetAdapter::new(
        "runpod-gpu.cloud",
        12345,
        "root",
        ComputeBackendKind::RunPod,
        tmp.path(),
    )
    .with_pod_lease("pod_abc", Some("runpod_key".into()))
    .with_mock_mode();

    let attempt = ExecutionAttempt::new("exp_gpu", "rec_gpu", ComputeBackendKind::RunPod, None);
    let job = adapter.submit(&attempt, "idem_1").await.unwrap();
    assert_eq!(job.account_ref, "runpod-gpu.cloud");
    assert_eq!(job.job_id, attempt.id);
    assert_eq!(job.version_id, "pid_41920");

    // Initially Running, Held
    let obs = adapter.observe(&job).await.unwrap();
    assert_eq!(obs.exec_state, ExecState::Running);
    assert_eq!(obs.resource_state, ResourceState::Held);

    // Stopped, still Held (billing keeps going until pod is released!)
    adapter.stop_job(&job).await.unwrap();
    let obs = adapter.observe(&job).await.unwrap();
    assert_eq!(obs.exec_state, ExecState::Stopped);
    assert_eq!(obs.resource_state, ResourceState::Held);

    // Released confirmed: pod stopped to save cost
    adapter.release_compute(&job).await.unwrap();
    assert!(adapter.is_released().await);
    let obs = adapter.observe(&job).await.unwrap();
    assert_eq!(obs.resource_state, ResourceState::ReleaseConfirmed);
}

#[tokio::test]
async fn rented_gpu_submit_fails_in_live_mode_without_recipe() {
    let tmp = tempfile::tempdir().unwrap();
    let adapter = RentedGpuTargetAdapter::new(
        "192.168.1.50",
        22,
        "root",
        ComputeBackendKind::DedicatedSsh,
        tmp.path(),
    );
    let attempt = ExecutionAttempt::new(
        "exp_gpu_fail",
        "default",
        ComputeBackendKind::DedicatedSsh,
        None,
    );
    let res = adapter.submit(&attempt, "idem").await;
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("Cannot submit real GPU job without a valid recipe_id"));
}
