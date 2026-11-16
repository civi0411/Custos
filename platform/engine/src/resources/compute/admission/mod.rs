//! Workload Resource Admission Coordinator (SADE GPU-A / GPU-B)
//!
//! Enforces hardware resource arbitration, device inventory discovery,
//! and exclusive mutual exclusion (preventing dual launch on exclusive accelerators).

use async_trait::async_trait;
use chrono::Utc;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use custos_core::contracts::compute::WorkloadAdmissionPort;
use custos_domain::{
    new_id, ComputeDeviceRef, DomainError, ExecutionLane, LanePolicy,
    WorkloadAdmissionRequest, WorkloadAdmissionVerdict, WorkloadLease, WorkloadLeaseStatus,
};

#[cfg(test)]
mod tests;

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
    lane_policy: Arc<RwLock<LanePolicy>>,
    workbench_policies: Arc<RwLock<HashMap<String, LanePolicy>>>,
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
            lane_policy: Arc::new(RwLock::new(LanePolicy::default())),
            workbench_policies: Arc::new(RwLock::new(HashMap::new())),
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
            lane_policy: Arc::new(RwLock::new(LanePolicy::default())),
            workbench_policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configures the coordinator with an explicit global lane policy
    pub fn with_policy(mut self, policy: LanePolicy) -> Self {
        self.lane_policy = Arc::new(RwLock::new(policy));
        self
    }

    /// Dynamically registers or updates the global execution lane policy
    pub async fn set_policy(&self, policy: LanePolicy) {
        let mut p = self.lane_policy.write().await;
        *p = policy;
    }

    /// Dynamically registers a workbench-specific lane policy
    pub async fn set_workbench_policy(&self, workbench_id: impl Into<String>, policy: LanePolicy) {
        let mut map = self.workbench_policies.write().await;
        map.insert(workbench_id.into(), policy);
    }

    /// Returns the number of currently admitted workloads in the given execution lane
    pub async fn active_lane_count(&self, lane: ExecutionLane) -> usize {
        let leases = self.leases.read().await;
        leases
            .values()
            .filter(|l| l.status == WorkloadLeaseStatus::Admitted && l.lane == lane)
            .count()
    }

    /// Returns a telemetry snapshot of active workload counts by execution lane
    pub async fn active_lane_counts(&self) -> HashMap<ExecutionLane, usize> {
        let leases = self.leases.read().await;
        let mut counts = HashMap::new();
        for l in leases.values() {
            if l.status == WorkloadLeaseStatus::Admitted {
                *counts.entry(l.lane).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Resolves the effective lane policy for an owner / workbench
    async fn resolve_policy(&self, owner_id: &str) -> LanePolicy {
        let wb = self.workbench_policies.read().await;
        for (prefix, pol) in wb.iter() {
            if owner_id.contains(prefix) {
                return pol.clone();
            }
        }
        self.lane_policy.read().await.clone()
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
        let target_device_id = self
            .resolve_device_id(request.preferred_device_id.as_deref())
            .await?;
        let lane = request.effective_lane();
        let policy = self.resolve_policy(&request.owner_id).await;

        let mut devs = self.devices.write().await;
        let mut leases = self.leases.write().await;

        // 1. Lane concurrency quota check & arbitration
        let current_lane_count = leases
            .values()
            .filter(|l| l.status == WorkloadLeaseStatus::Admitted && l.lane == lane)
            .count();
        let max_for_lane = policy.max_concurrency_for_lane(lane);

        if current_lane_count >= max_for_lane {
            let mut preempted = false;
            if policy.allow_preemption && lane == ExecutionLane::Interactive {
                // Find lowest-priority preemptible lease across system to free slot
                if let Some(target_id) = leases
                    .iter()
                    .filter(|(_, l)| l.status == WorkloadLeaseStatus::Admitted && l.lane.is_preemptible())
                    .min_by_key(|(_, l)| l.lane.priority_rank())
                    .map(|(k, _)| k.clone())
                {
                    if let Some(l) = leases.get_mut(&target_id) {
                        l.status = WorkloadLeaseStatus::Preempted;
                        l.released_at = Some(Utc::now().timestamp());
                        let dev_id = l.device_id.clone();
                        if let Some(s) = devs.get_mut(&dev_id) {
                            if let Some(ref exc) = s.active_exclusive_lease {
                                if exc.lease_id == target_id {
                                    s.active_exclusive_lease = None;
                                }
                            }
                            s.active_shared_leases.remove(&target_id);
                        }
                        tracing::warn!(
                            preempted_id = %target_id,
                            admitted_lane = ?lane,
                            "Preempted ambient/batch lease to admit higher-priority interactive workload"
                        );
                        preempted = true;
                    }
                }
            }

            if !preempted {
                let reason = format!(
                    "Lane {:?} concurrency quota ({}/{}) saturated; request queued",
                    lane, current_lane_count, max_for_lane
                );
                let q_pos = if let Some(slot) = devs.get_mut(&target_device_id) {
                    slot.queue.push_back(request.clone());
                    slot.queue.len()
                } else {
                    1
                };
                return Ok(WorkloadAdmissionVerdict::Queued {
                    queue_position: q_pos,
                    reason,
                });
            }
        }

        let slot = devs
            .get_mut(&target_device_id)
            .ok_or_else(|| DomainError::NotFound {
                kind: "device".into(),
                id: target_device_id.clone(),
            })?;

        // 2. Exclusive mutual exclusion with preemption
        if let Some(ref exclusive_lease) = slot.active_exclusive_lease {
            let can_preempt = policy.allow_preemption
                && (exclusive_lease.lane == ExecutionLane::Ambient
                    || (request.require_exclusive && lane.can_preempt(&exclusive_lease.lane)));

            if can_preempt {
                let preempted_id = exclusive_lease.lease_id.clone();
                let preempted_lane = exclusive_lease.lane;
                if let Some(l) = leases.get_mut(&preempted_id) {
                    l.status = WorkloadLeaseStatus::Preempted;
                    l.released_at = Some(Utc::now().timestamp());
                }
                slot.active_exclusive_lease = None;
                tracing::warn!(
                    preempted_id = %preempted_id,
                    preempted_lane = ?preempted_lane,
                    admitted_lane = ?lane,
                    "Preempted exclusive lease on device '{}' for high-priority workload",
                    target_device_id
                );
            } else {
                let reason = format!(
                    "Device '{}' is exclusively held by workload '{:?}' (owner: '{}', lane: '{:?}')",
                    target_device_id, exclusive_lease.workload_kind, exclusive_lease.owner_id, exclusive_lease.lane
                );
                slot.queue.push_back(request.clone());
                return Ok(WorkloadAdmissionVerdict::Queued {
                    queue_position: slot.queue.len(),
                    reason,
                });
            }
        }

        // 3. Shared workloads conflict with exclusive request
        if request.require_exclusive && slot.active_shared_count() > 0 {
            let can_preempt_all = policy.allow_preemption
                && slot.active_shared_leases.values().all(|l| lane.can_preempt(&l.lane));
            if can_preempt_all && !slot.active_shared_leases.is_empty() {
                for (shared_id, shared_lease) in slot.active_shared_leases.drain() {
                    if let Some(l) = leases.get_mut(&shared_id) {
                        l.status = WorkloadLeaseStatus::Preempted;
                        l.released_at = Some(Utc::now().timestamp());
                    }
                    tracing::warn!(
                        preempted_id = %shared_id,
                        preempted_lane = ?shared_lease.lane,
                        "Preempted shared ambient lease for exclusive high-priority workload"
                    );
                }
            } else {
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
        }

        // 4. Memory capacity preflight check
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

        // 5. Admit the lease
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
            lane,
        };

        if request.require_exclusive {
            slot.active_exclusive_lease = Some(lease.clone());
        } else {
            slot.active_shared_leases
                .insert(lease_id.clone(), lease.clone());
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
