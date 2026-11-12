use crate::fleet::compute::admission::*;
use crate::fleet::compute::backend::*;

#[test]
fn test_compute_device_ref_and_admission_models() {
    let cpu = ComputeDeviceRef::cpu();
    assert_eq!(cpu.device_kind, DeviceKind::Cpu);
    assert_eq!(cpu.device_id, "dev_cpu_0");

    let mps = ComputeDeviceRef::apple_mps(32768);
    assert_eq!(mps.device_kind, DeviceKind::Mps);
    assert_eq!(mps.total_memory_mb, 32768);
    assert_eq!(mps.available_memory_mb, Some(24576));

    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-1".into()));
    assert_eq!(cuda.device_kind, DeviceKind::Cuda);
    assert_eq!(cuda.ordinal, 0);

    let req = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "run_123".into(),
        preferred_device_id: Some(cuda.device_id.clone()),
        require_exclusive: true,
        required_memory_mb: Some(20000),
        lane: None,
    };
    assert!(req.require_exclusive);
    assert_eq!(req.effective_lane(), crate::ExecutionLane::Batch);

    let lease = WorkloadLease {
        lease_id: "lease_abc".into(),
        device_id: cuda.device_id,
        workload_kind: req.workload_kind,
        owner_id: req.owner_id.clone(),
        is_exclusive: true,
        status: WorkloadLeaseStatus::Admitted,
        acquired_at: 1000,
        released_at: None,
        lane: req.effective_lane(),
    };
    assert_eq!(lease.status, WorkloadLeaseStatus::Admitted);
    assert_eq!(lease.lane, crate::ExecutionLane::Batch);
}

