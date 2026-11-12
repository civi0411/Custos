use super::*;
use custos_domain::ComputeWorkloadKind;

#[tokio::test]
async fn test_device_discovery_and_exclusive_mutual_exclusion() {
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-1".into()));
    let coord = WorkloadAdmissionCoordinator::with_custom_devices(vec![cuda.clone()]);

    let devices = coord.discover_devices().await.unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_id, "dev_cuda_0");

    // 1. Training workload requests exclusive lease
    let req_training = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "train_job_1".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: true,
        required_memory_mb: Some(16000),
        lane: None,
    };
    let verdict1 = coord.admit_workload(&req_training).await.unwrap();
    let lease1 = match verdict1 {
        WorkloadAdmissionVerdict::Admitted { lease } => lease,
        _ => panic!("Expected admitted verdict"),
    };
    assert_eq!(lease1.workload_kind, ComputeWorkloadKind::TrainingRun);
    assert_eq!(lease1.lane, ExecutionLane::Batch);

    // 2. Second exclusive workload attempts to claim the same device -> must be queued (no double launch!)
    let req_training_2 = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "train_job_2".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: true,
        required_memory_mb: Some(16000),
        lane: None,
    };
    let verdict2 = coord.admit_workload(&req_training_2).await.unwrap();
    match verdict2 {
        WorkloadAdmissionVerdict::Queued {
            queue_position,
            reason,
        } => {
            assert_eq!(queue_position, 1);
            assert!(reason.contains("exclusively held"));
        }
        _ => panic!("Expected queued verdict due to exclusive lock"),
    }

    // 3. Notebook workload arrives while training holds GPU -> must be queued/rejected, no dual launch!
    let req_nb = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::NotebookKernel,
        owner_id: "notebook_session_1".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: false,
        required_memory_mb: None,
        lane: None,
    };
    let verdict3 = coord.admit_workload(&req_nb).await.unwrap();
    assert!(matches!(verdict3, WorkloadAdmissionVerdict::Queued { .. }));

    // 4. Release training lease -> device is cleanly freed
    coord.release_lease(&lease1.lease_id).await.unwrap();
    let active = coord.list_active_leases(Some("dev_cuda_0")).await.unwrap();
    assert_eq!(active.len(), 0);
}

#[tokio::test]
async fn test_shared_workloads_coexistence() {
    let cpu = ComputeDeviceRef::cpu();
    let coord = WorkloadAdmissionCoordinator::with_custom_devices(vec![cpu.clone()]);

    // Two shared notebook kernels can coexist
    let req1 = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::NotebookKernel,
        owner_id: "session_a".into(),
        preferred_device_id: Some("dev_cpu_0".into()),
        require_exclusive: false,
        required_memory_mb: None,
        lane: None,
    };
    let req2 = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::NotebookKernel,
        owner_id: "session_b".into(),
        preferred_device_id: Some("dev_cpu_0".into()),
        require_exclusive: false,
        required_memory_mb: None,
        lane: None,
    };

    let v1 = coord.admit_workload(&req1).await.unwrap();
    let v2 = coord.admit_workload(&req2).await.unwrap();

    assert!(matches!(v1, WorkloadAdmissionVerdict::Admitted { .. }));
    assert!(matches!(v2, WorkloadAdmissionVerdict::Admitted { .. }));

    let active = coord.list_active_leases(Some("dev_cpu_0")).await.unwrap();
    assert_eq!(active.len(), 2);
    assert_eq!(coord.active_lane_count(ExecutionLane::Interactive).await, 2);

    // Orphan sweep for session_a cleanly releases its lease
    let swept = coord.sweep_orphaned_leases("session_a").await;
    assert_eq!(swept.len(), 1);

    let active_remaining = coord.list_active_leases(Some("dev_cpu_0")).await.unwrap();
    assert_eq!(active_remaining.len(), 1);
    assert_eq!(active_remaining[0].owner_id, "session_b");
}

#[tokio::test]
async fn test_lane_quota_enforcement() {
    let cpu = ComputeDeviceRef::cpu();
    // Policy: max 2 agent workloads
    let mut policy = LanePolicy::coding_fanout(2);
    policy.allow_preemption = false;
    let coord = WorkloadAdmissionCoordinator::with_custom_devices(vec![cpu.clone()])
        .with_policy(policy);

    let req1 = WorkloadAdmissionRequest::new(ComputeWorkloadKind::NotebookKernel, "agent_1")
        .with_lane(ExecutionLane::Agent);
    let req2 = WorkloadAdmissionRequest::new(ComputeWorkloadKind::NotebookKernel, "agent_2")
        .with_lane(ExecutionLane::Agent);
    let req3 = WorkloadAdmissionRequest::new(ComputeWorkloadKind::NotebookKernel, "agent_3")
        .with_lane(ExecutionLane::Agent);

    let v1 = coord.admit_workload(&req1).await.unwrap();
    let v2 = coord.admit_workload(&req2).await.unwrap();
    let v3 = coord.admit_workload(&req3).await.unwrap();

    assert!(matches!(v1, WorkloadAdmissionVerdict::Admitted { .. }));
    assert!(matches!(v2, WorkloadAdmissionVerdict::Admitted { .. }));
    // 3rd agent request must be queued because quota (2) is reached
    match v3 {
        WorkloadAdmissionVerdict::Queued { queue_position, reason } => {
            assert_eq!(queue_position, 1);
            assert!(reason.contains("quota"));
        }
        other => panic!("Expected Queued, got {:?}", other),
    }
    assert_eq!(coord.active_lane_count(ExecutionLane::Agent).await, 2);
}

#[tokio::test]
async fn test_priority_preemption_interactive_over_ambient() {
    let cuda = ComputeDeviceRef::cuda(0, "RTX 4090", 24576, Some("GPU-1".into()));
    let coord = WorkloadAdmissionCoordinator::with_custom_devices(vec![cuda.clone()]);

    // 1. Ambient background embedding running on GPU
    let req_ambient = WorkloadAdmissionRequest::new(
        ComputeWorkloadKind::BackgroundEmbedding,
        "ocr_ambient_task",
    )
    .with_exclusive(true);

    let v_ambient = coord.admit_workload(&req_ambient).await.unwrap();
    let lease_ambient = match v_ambient {
        WorkloadAdmissionVerdict::Admitted { lease } => lease,
        other => panic!("Expected ambient admitted, got {:?}", other),
    };
    assert_eq!(lease_ambient.lane, ExecutionLane::Ambient);
    assert_eq!(coord.active_lane_count(ExecutionLane::Ambient).await, 1);

    // 2. High-priority interactive chat request arrives requiring exclusive device access
    let req_interactive = WorkloadAdmissionRequest::new(
        ComputeWorkloadKind::LocalInference,
        "interactive_chat_user",
    )
    .with_lane(ExecutionLane::Interactive)
    .with_exclusive(true);

    let v_interactive = coord.admit_workload(&req_interactive).await.unwrap();
    let lease_interactive = match v_interactive {
        WorkloadAdmissionVerdict::Admitted { lease } => lease,
        other => panic!("Expected interactive admitted via preemption, got {:?}", other),
    };
    assert_eq!(lease_interactive.lane, ExecutionLane::Interactive);

    // 3. Verify ambient lease was marked Preempted and interactive lease is active
    let active = coord.list_active_leases(Some("dev_cuda_0")).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].owner_id, "interactive_chat_user");
    assert_eq!(coord.active_lane_count(ExecutionLane::Ambient).await, 0);
    assert_eq!(coord.active_lane_count(ExecutionLane::Interactive).await, 1);
}
