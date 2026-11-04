//! Workload Resource Admission Coordinator (SADE GPU-A / GPU-B)
//!
//! Enforces hardware resource arbitration, device inventory discovery,
//! and exclusive mutual exclusion (preventing dual launch on exclusive accelerators).

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use custos_core::contracts::compute::WorkloadAdmissionPort;
use custos_domain::{
    new_id, ComputeDeviceRef, DomainError, WorkloadAdmissionRequest, WorkloadAdmissionVerdict,
    WorkloadLease, WorkloadLeaseStatus,
};

#[derive(Debug, Clone)]
struct DeviceSlot {
    device: ComputeDeviceRef,
    active_exclusive_lease: Option<WorkloadLease>,
    active_shared_leases: HashMap<String, WorkloadLease>,
    queue: VecDeque<WorkloadAdmissionRequest>,
}

impl DeviceSlot {
    fn new(device: ComputeDeviceRef) -> Self {
        Self {
            device,
            active_exclusive_lease: None,
            active_shared_leases: HashMap::new(),
            queue: VecDeque::new(),
        }
    }

    fn active_shared_count(&self) -> usize {
        self.active_shared_leases.len()
    }
}

/// Runtime coordinator responsible for device inventory and workload admission arbitration
#[derive(Clone)]
pub struct WorkloadAdmissionCoordinator {
    devices: Arc<RwLock<HashMap<String, DeviceSlot>>>,
    leases: Arc<RwLock<HashMap<String, WorkloadLease>>>,
}

impl Default for WorkloadAdmissionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkloadAdmissionCoordinator {
    /// Creates a new coordinator populated with detected local host devices (CPU + MPS if on macOS)
    pub fn new() -> Self {
        let mut initial_devices = HashMap::new();

        // 1. Host CPU is always available
        let cpu = ComputeDeviceRef::cpu();
        initial_devices.insert(cpu.device_id.clone(), DeviceSlot::new(cpu));

        // 2. Detect Apple Silicon MPS on macOS
        #[cfg(target_os = "macos")]
        {
            let mps = ComputeDeviceRef::apple_mps(32768);
            initial_devices.insert(mps.device_id.clone(), DeviceSlot::new(mps));
        }

        Self {
            devices: Arc::new(RwLock::new(initial_devices)),
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a coordinator initialized with an explicit list of devices (ideal for deterministic test fixtures)
    pub fn with_custom_devices(devices: Vec<ComputeDeviceRef>) -> Self {
        let mut map = HashMap::new();
        for dev in devices {
            map.insert(dev.device_id.clone(), DeviceSlot::new(dev));
        }
        Self {
            devices: Arc::new(RwLock::new(map)),
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Dynamically registers an inventoried compute device
    pub async fn register_device(&self, device: ComputeDeviceRef) {
        let mut devs = self.devices.write().await;
        devs.insert(device.device_id.clone(), DeviceSlot::new(device));
    }

    /// Resolves the target device ID for a request
    async fn resolve_device_id(&self, preferred: Option<&str>) -> Result<String, DomainError> {
        let devs = self.devices.read().await;
        if let Some(id) = preferred {
            if devs.contains_key(id) {
                return Ok(id.to_string());
            }
            return Err(DomainError::NotFound {
                kind: "device".into(),
                id: id.to_string(),
            });
        }
        // Default to first available accelerator, or fallback to CPU
        for (id, slot) in devs.iter() {
            if slot.device.device_kind != custos_domain::DeviceKind::Cpu {
                return Ok(id.clone());
            }
        }
        devs.keys()
            .next()
            .cloned()
            .ok_or_else(|| DomainError::NotFound {
                kind: "device".into(),
                id: "none".into(),
            })
    }

    /// Sweeps and releases all active leases associated with an owner (e.g. on session crash or kernel reset)
    pub async fn sweep_orphaned_leases(&self, owner_id: &str) -> Vec<String> {
        let mut released_ids = Vec::new();
        let mut leases = self.leases.write().await;
        let mut devs = self.devices.write().await;

        for (lease_id, lease) in leases.iter_mut() {
            if lease.owner_id == owner_id && lease.status == WorkloadLeaseStatus::Admitted {
                lease.status = WorkloadLeaseStatus::Released;
                lease.released_at = Some(Utc::now().timestamp());
                released_ids.push(lease_id.clone());

                if let Some(slot) = devs.get_mut(&lease.device_id) {
                    if let Some(ref exc) = slot.active_exclusive_lease {
                        if exc.lease_id == *lease_id {
                            slot.active_exclusive_lease = None;
                        }
                    }
                    slot.active_shared_leases.remove(lease_id);
                }
            }
        }
        released_ids
    }
}

#[async_trait]
impl WorkloadAdmissionPort for WorkloadAdmissionCoordinator {
    async fn discover_devices(&self) -> Result<Vec<ComputeDeviceRef>, DomainError> {
        let devs = self.devices.read().await;
        Ok(devs.values().map(|slot| slot.device.clone()).collect())
    }

    async fn admit_workload(
        &self,
        request: &WorkloadAdmissionRequest,
    ) -> Result<WorkloadAdmissionVerdict, DomainError> {
        let target_device_id = self.resolve_device_id(request.preferred_device_id.as_deref()).await?;
        let mut devs = self.devices.write().await;
        let mut leases = self.leases.write().await;

        let slot = devs
            .get_mut(&target_device_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "device".into(),
                id: target_device_id.clone(),
            })?;

        // Invariant 1: If device is currently held by an exclusive workload, reject or queue new requests
        if let Some(ref exclusive_lease) = slot.active_exclusive_lease {
            let reason = format!(
                "Device '{}' is exclusively held by workload '{:?}' (owner: '{}')",
                target_device_id, exclusive_lease.workload_kind, exclusive_lease.owner_id
            );
            slot.queue.push_back(request.clone());
            return Ok(WorkloadAdmissionVerdict::Queued {
                queue_position: slot.queue.len(),
                reason,
            });
        }

        // Invariant 2: If the new request requires exclusive access, but the device currently has active shared workloads
        if request.require_exclusive && slot.active_shared_count() > 0 {
            let reason = format!(
                "Device '{}' currently running {} shared workloads; exclusive request queued",
                target_device_id,
                slot.active_shared_count()
            );
            slot.queue.push_back(request.clone());
            return Ok(WorkloadAdmissionVerdict::Queued {
                queue_position: slot.queue.len(),
                reason,
            });
        }

        // Invariant 3: Memory capacity preflight check
        if let Some(req_mb) = request.required_memory_mb {
            if req_mb > slot.device.total_memory_mb {
                return Ok(WorkloadAdmissionVerdict::Rejected {
                    reason: format!(
                        "Requested memory ({} MB) exceeds device total capacity ({} MB)",
                        req_mb, slot.device.total_memory_mb
                    ),
                });
            }
        }

        // Admit the lease
        let lease_id = new_id("lease");
        let lease = WorkloadLease {
            lease_id: lease_id.clone(),
            device_id: target_device_id.clone(),
            workload_kind: request.workload_kind,
            owner_id: request.owner_id.clone(),
            is_exclusive: request.require_exclusive,
            status: WorkloadLeaseStatus::Admitted,
            acquired_at: Utc::now().timestamp(),
            released_at: None,
        };

        if request.require_exclusive {
            slot.active_exclusive_lease = Some(lease.clone());
        } else {
            slot.active_shared_leases.insert(lease_id.clone(), lease.clone());
        }

        leases.insert(lease_id, lease.clone());
        Ok(WorkloadAdmissionVerdict::Admitted { lease })
    }

    async fn release_lease(&self, lease_id: &str) -> Result<(), DomainError> {
        let mut leases = self.leases.write().await;
        let mut devs = self.devices.write().await;

        let lease = leases
            .get_mut(lease_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "lease".into(),
                id: lease_id.to_string(),
            })?;

        if lease.status == WorkloadLeaseStatus::Released {
            return Ok(());
        }

        lease.status = WorkloadLeaseStatus::Released;
        lease.released_at = Some(Utc::now().timestamp());

        if let Some(slot) = devs.get_mut(&lease.device_id) {
            if let Some(ref exc) = slot.active_exclusive_lease {
                if exc.lease_id == lease_id {
                    slot.active_exclusive_lease = None;
                }
            }
            slot.active_shared_leases.remove(lease_id);
        }

        Ok(())
    }

    async fn list_active_leases(
        &self,
        device_id: Option<&str>,
    ) -> Result<Vec<WorkloadLease>, DomainError> {
        let leases = self.leases.read().await;
        let list = leases
            .values()
            .filter(|lease| {
                lease.status == WorkloadLeaseStatus::Admitted
                    && device_id.is_none_or(|d| d == lease.device_id)
            })
            .cloned()
            .collect();
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
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
        };
        let verdict1 = coord.admit_workload(&req_training).await.unwrap();
        let lease1 = match verdict1 {
            WorkloadAdmissionVerdict::Admitted { lease } => lease,
            _ => panic!("Expected admitted verdict"),
        };
        assert_eq!(lease1.workload_kind, ComputeWorkloadKind::TrainingRun);

        // 2. Second exclusive workload attempts to claim the same device -> must be queued (no double launch!)
        let req_training_2 = WorkloadAdmissionRequest {
            workload_kind: ComputeWorkloadKind::TrainingRun,
            owner_id: "train_job_2".into(),
            preferred_device_id: Some("dev_cuda_0".into()),
            require_exclusive: true,
            required_memory_mb: Some(16000),
        };
        let verdict2 = coord.admit_workload(&req_training_2).await.unwrap();
        match verdict2 {
            WorkloadAdmissionVerdict::Queued { queue_position, reason } => {
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
        };
        let req2 = WorkloadAdmissionRequest {
            workload_kind: ComputeWorkloadKind::NotebookKernel,
            owner_id: "session_b".into(),
            preferred_device_id: Some("dev_cpu_0".into()),
            require_exclusive: false,
            required_memory_mb: None,
        };

        let v1 = coord.admit_workload(&req1).await.unwrap();
        let v2 = coord.admit_workload(&req2).await.unwrap();

        assert!(matches!(v1, WorkloadAdmissionVerdict::Admitted { .. }));
        assert!(matches!(v2, WorkloadAdmissionVerdict::Admitted { .. }));

        let active = coord.list_active_leases(Some("dev_cpu_0")).await.unwrap();
        assert_eq!(active.len(), 2);

        // Orphan sweep for session_a cleanly releases its lease
        let swept = coord.sweep_orphaned_leases("session_a").await;
        assert_eq!(swept.len(), 1);

        let active_remaining = coord.list_active_leases(Some("dev_cpu_0")).await.unwrap();
        assert_eq!(active_remaining.len(), 1);
        assert_eq!(active_remaining[0].owner_id, "session_b");
    }
}
