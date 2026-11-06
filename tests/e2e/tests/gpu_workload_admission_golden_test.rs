//! Golden Acceptance Test: GPU Workload Components & Resource Admission (GPU-A / GPU-B)
//!
//! Validates:
//! 1. Hardware Device Inventory Discovery & Classification.
//! 2. Exclusive Mutual Exclusion & Anti-Dual-Launch (Zero duplicate runs on exclusive accelerators).
//! 3. Inter-Workload Collision Avoidance (Training holding exclusive GPU blocks notebook/inference dual launch).
//! 4. Multi-Notebook Shared Coexistence (Concurrent non-exclusive notebook executions receive isolated leases).
//! 5. Crash, Interrupt, and Kernel Reset Resource Recovery (Zero zombie leases, clean teardown).
//! 6. Bounded Diagnostic Probe (Python accelerator environment probe with strict timeout).

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use custos_core::contracts::compute::WorkloadAdmissionPort;
use custos_core::contracts::notebook::{
    NotebookDriverPort, NotebookWorkerResponse, NotebookWorkerSession,
};
use custos_domain::{
    ComputeDeviceRef, ComputeWorkloadKind, DeviceKind, DomainError, ExecuteCellParams,
    WorkloadAdmissionRequest, WorkloadAdmissionVerdict, WorkloadLeaseStatus,
};
use custos_runtime::compute::WorkloadAdmissionCoordinator;
use custos_runtime::notebook::PythonKernelCoordinator;

// ─────────────────────────────────────────────────────────────────────────────
// Mock Notebook Driver for Deterministic Kernel Testing
// ─────────────────────────────────────────────────────────────────────────────
struct MockWorkerSession;

#[async_trait]
impl NotebookWorkerSession for MockWorkerSession {
    async fn execute(
        &self,
        code: String,
        _cwd: Option<String>,
    ) -> Result<NotebookWorkerResponse, DomainError> {
        Ok(NotebookWorkerResponse {
            status: "success".into(),
            stdout: format!("Executed: {}", code.trim()),
            stderr: String::new(),
        })
    }

    fn interrupt(&self) {}
}

struct MockNotebookDriver;

#[async_trait]
impl NotebookDriverPort for MockNotebookDriver {
    async fn spawn_worker(
        &self,
        _python_path: PathBuf,
        _work_dir: PathBuf,
    ) -> Result<Box<dyn NotebookWorkerSession>, DomainError> {
        Ok(Box::new(MockWorkerSession))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 1: Hardware Device Inventory Discovery & Classification
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_1_device_inventory_discovery() {
    let cpu = ComputeDeviceRef::cpu();
    let mps = ComputeDeviceRef::apple_mps(32768);
    let cuda0 = ComputeDeviceRef::cuda(
        0,
        "NVIDIA GeForce RTX 4090",
        24576,
        Some("GPU-UUID-0".into()),
    );
    let cuda1 =
        ComputeDeviceRef::cuda(1, "NVIDIA A100-SXM4-40GB", 40960, Some("GPU-UUID-1".into()));

    let coord = WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cpu.clone(),
        mps.clone(),
        cuda0.clone(),
        cuda1.clone(),
    ]);

    let devices = coord.discover_devices().await.unwrap();
    assert_eq!(devices.len(), 4, "Must discover all 4 registered devices");

    let found_cpu = devices
        .iter()
        .find(|d| d.device_kind == DeviceKind::Cpu)
        .unwrap();
    assert_eq!(found_cpu.device_id, "dev_cpu_0");

    let found_mps = devices
        .iter()
        .find(|d| d.device_kind == DeviceKind::Mps)
        .unwrap();
    assert_eq!(found_mps.total_memory_mb, 32768);

    let found_4090 = devices
        .iter()
        .find(|d| d.device_id == "dev_cuda_0")
        .unwrap();
    assert_eq!(found_4090.device_kind, DeviceKind::Cuda);
    assert_eq!(found_4090.ordinal, 0);
    assert_eq!(found_4090.total_memory_mb, 24576);

    let found_a100 = devices
        .iter()
        .find(|d| d.device_id == "dev_cuda_1")
        .unwrap();
    assert_eq!(found_a100.ordinal, 1);
    assert_eq!(found_a100.total_memory_mb, 40960);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 2: Exclusive Mutual Exclusion & Anti-Dual-Launch (GPU-A Core Gate)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_2_exclusive_mutual_exclusion_anti_dual_launch() {
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let coord = WorkloadAdmissionCoordinator::with_custom_devices(vec![cuda.clone()]);

    // Workload 1: Training Run claims exclusive access
    let req_training_1 = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "training_run_alpha".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: true,
        required_memory_mb: Some(20000),
    };

    let verdict1 = coord.admit_workload(&req_training_1).await.unwrap();
    let lease1 = match verdict1 {
        WorkloadAdmissionVerdict::Admitted { lease } => lease,
        other => panic!("Expected Admitted for first workload, got: {:?}", other),
    };
    assert_eq!(lease1.status, WorkloadLeaseStatus::Admitted);
    assert_eq!(lease1.device_id, "dev_cuda_0");
    assert!(lease1.is_exclusive);

    // Workload 2: Second Training Run attempts to claim same exclusive GPU
    let req_training_2 = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "training_run_beta".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: true,
        required_memory_mb: Some(20000),
    };

    let verdict2 = coord.admit_workload(&req_training_2).await.unwrap();
    match verdict2 {
        WorkloadAdmissionVerdict::Queued {
            queue_position,
            reason,
        } => {
            assert_eq!(queue_position, 1);
            assert!(
                reason.contains(
                    "exclusively held by workload 'TrainingRun' (owner: 'training_run_alpha')"
                ),
                "Must clearly state existing owner and exclusive reason"
            );
        }
        other => panic!(
            "Expected Queued verdict for conflicting exclusive launch, got: {:?}",
            other
        ),
    }

    // Verify active leases on dev_cuda_0 only contains exactly ONE active lease (NO DUAL LAUNCH)
    let active = coord.list_active_leases(Some("dev_cuda_0")).await.unwrap();
    assert_eq!(
        active.len(),
        1,
        "Only one workload may hold the exclusive GPU simultaneously"
    );
    assert_eq!(active[0].lease_id, lease1.lease_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 3: Inter-Workload Collision Avoidance (Training vs Notebook)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_3_training_holds_gpu_blocks_notebook_execution() {
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));

    // 1. Training workload occupies GPU
    let req_training = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::TrainingRun,
        owner_id: "unsloth_lora_run".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: true,
        required_memory_mb: Some(18000),
    };
    let v_train = admission.admit_workload(&req_training).await.unwrap();
    assert!(matches!(v_train, WorkloadAdmissionVerdict::Admitted { .. }));

    // 2. Notebook Coordinator wired to admission arbiter
    let driver = Arc::new(MockNotebookDriver);
    let nb_coord =
        PythonKernelCoordinator::new(driver).with_admission_coordinator(admission.clone());

    // 3. User attempts to execute notebook cell on the occupied GPU
    let cell_params = ExecuteCellParams {
        session_id: "session_lab_1".into(),
        cell_id: "cell_eval_tensor".into(),
        code: "import torch; x = torch.ones(10, device='cuda')".into(),
        workspace_id: None,
        cwd: None,
        task_id: None,
        notebook_id: Some("lab_nb".into()),
        kernel_id: None,
        cell_version: None,
        execution_request_id: Some("req_cell_1".into()),
        device_id: Some("dev_cuda_0".into()),
    };

    let result = nb_coord.execute_cell(cell_params, None).await;
    match result {
        Err(DomainError::Conflict(message)) => {
            assert!(
                message.contains("Compute device 'dev_cuda_0' busy"),
                "Cell execution must be cleanly blocked with Conflict error rather than crashing or dual launching"
            );
        }
        other => panic!(
            "Expected Conflict error due to exclusive training occupancy, got: {:?}",
            other
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 4: Multi-Notebook Shared Coexistence (GPU-B Multi-Notebook)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_4_multi_notebook_shared_coexistence() {
    let mps = ComputeDeviceRef::apple_mps(32768);
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        mps.clone()
    ]));

    let driver = Arc::new(MockNotebookDriver);
    let nb_coord =
        PythonKernelCoordinator::new(driver).with_admission_coordinator(admission.clone());

    // Execute Notebook A cell on MPS
    let cell_a = ExecuteCellParams {
        session_id: "session_nb_a".into(),
        cell_id: "cell_a_1".into(),
        code: "x = 100".into(),
        workspace_id: None,
        cwd: None,
        task_id: None,
        notebook_id: Some("nb_a".into()),
        kernel_id: None,
        cell_version: None,
        execution_request_id: Some("req_a_1".into()),
        device_id: Some("dev_mps_0".into()),
    };
    let res_a = nb_coord.execute_cell(cell_a, None).await.unwrap();
    assert_eq!(res_a.device_id, Some("dev_mps_0".into()));

    // Execute Notebook B cell on same MPS device concurrently
    let cell_b = ExecuteCellParams {
        session_id: "session_nb_b".into(),
        cell_id: "cell_b_1".into(),
        code: "y = 200".into(),
        workspace_id: None,
        cwd: None,
        task_id: None,
        notebook_id: Some("nb_b".into()),
        kernel_id: None,
        cell_version: None,
        execution_request_id: Some("req_b_1".into()),
        device_id: Some("dev_mps_0".into()),
    };
    let res_b = nb_coord.execute_cell(cell_b, None).await.unwrap();
    assert_eq!(res_b.device_id, Some("dev_mps_0".into()));

    // Active leases must record both kernels sharing dev_mps_0
    let active = admission
        .list_active_leases(Some("dev_mps_0"))
        .await
        .unwrap();
    assert_eq!(
        active.len(),
        2,
        "Both notebook kernels hold valid shared leases"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 5: Crash, Interrupt, and Kernel Reset Resource Recovery (Zero Zombie Leases)
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_5_crash_reset_recovery_zero_zombies() {
    let cuda = ComputeDeviceRef::cuda(0, "NVIDIA RTX 4090", 24576, Some("GPU-UUID-0".into()));
    let admission = Arc::new(WorkloadAdmissionCoordinator::with_custom_devices(vec![
        cuda.clone(),
    ]));

    let driver = Arc::new(MockNotebookDriver);
    let nb_coord =
        PythonKernelCoordinator::new(driver).with_admission_coordinator(admission.clone());

    // 1. Notebook acquires shared lease during cell execution
    let cell_params = ExecuteCellParams {
        session_id: "session_reset_test".into(),
        cell_id: "cell_mem_heavy".into(),
        code: "import torch; tensors = torch.randn(100, 100)".into(),
        workspace_id: None,
        cwd: None,
        task_id: None,
        notebook_id: Some("nb_reset".into()),
        kernel_id: None,
        cell_version: None,
        execution_request_id: Some("req_mem".into()),
        device_id: Some("dev_cuda_0".into()),
    };
    let _ = nb_coord.execute_cell(cell_params, None).await.unwrap();

    let active_before = admission
        .list_active_leases(Some("dev_cuda_0"))
        .await
        .unwrap();
    assert_eq!(active_before.len(), 1, "Notebook must hold 1 active lease");

    // 2. Kernel reset occurs (e.g. user restarts kernel or memory cleared)
    let reset_state = nb_coord
        .reset_scoped("session_reset_test", Some("nb_reset"), None)
        .await
        .unwrap();
    assert_eq!(reset_state.epoch, 2, "Epoch must increment to 2");

    // 3. Verify lease was automatically released (ZERO ZOMBIE LEASES)
    let active_after_reset = admission
        .list_active_leases(Some("dev_cuda_0"))
        .await
        .unwrap();
    assert_eq!(
        active_after_reset.len(),
        0,
        "Resetting kernel must immediately release device lease"
    );

    // 4. Orphan sweep check for simulated unexpected process crash
    // Re-admit a lease simulating an orphaned session
    let req_orphan = WorkloadAdmissionRequest {
        workload_kind: ComputeWorkloadKind::LocalInference,
        owner_id: "crashed_session_xyz".into(),
        preferred_device_id: Some("dev_cuda_0".into()),
        require_exclusive: false,
        required_memory_mb: None,
    };
    let _ = admission.admit_workload(&req_orphan).await.unwrap();
    assert_eq!(
        admission
            .list_active_leases(Some("dev_cuda_0"))
            .await
            .unwrap()
            .len(),
        1
    );

    let swept = admission.sweep_orphaned_leases("crashed_session_xyz").await;
    assert_eq!(
        swept.len(),
        1,
        "Orphan sweeper must reclaim abandoned lease"
    );
    assert_eq!(
        admission
            .list_active_leases(Some("dev_cuda_0"))
            .await
            .unwrap()
            .len(),
        0
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenario 6: Bounded Diagnostic Probe
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_scenario_6_bounded_diagnostic_probe() {
    let driver = Arc::new(MockNotebookDriver);
    let nb_coord = PythonKernelCoordinator::new(driver);

    // Runs bounded probe with 5-second cap
    let probe_result = nb_coord.probe_kernel_device_environment(None).await;
    assert!(
        probe_result.is_ok(),
        "Probe execution must succeed or return structured capability status without hanging"
    );

    let report = probe_result.unwrap();
    // Must return either torch info, torch_unavailable, probe_stderr, or python_probe_failed
    assert!(
        report.contains("torch:")
            || report.contains("torch_unavailable")
            || report.contains("probe_stderr")
            || report.contains("python_probe_failed"),
        "Report must contain structured diagnostic telemetry: got '{}'",
        report
    );
}
