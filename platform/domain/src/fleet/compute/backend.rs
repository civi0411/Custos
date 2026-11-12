use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeBackendKind {
    Local,
    Colab,
    Kaggle,
    DedicatedSsh,
    RunPod,
    VastAi,
    LambdaCloud,
}

impl std::fmt::Display for ComputeBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Colab => write!(f, "colab"),
            Self::Kaggle => write!(f, "kaggle"),
            Self::DedicatedSsh => write!(f, "dedicated_ssh"),
            Self::RunPod => write!(f, "runpod"),
            Self::VastAi => write!(f, "vast_ai"),
            Self::LambdaCloud => write!(f, "lambda_cloud"),
        }
    }
}

/// Hardware accelerator profile observed or requested
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceleratorSpec {
    pub name: String,
    pub memory_mb: u64,
    pub compute_capability: Option<String>,
    pub supports_bfloat16: bool,
    pub driver_version: Option<String>,
    pub cuda_version: Option<String>,
}

impl AcceleratorSpec {
    pub fn new(name: impl Into<String>, memory_mb: u64, supports_bfloat16: bool) -> Self {
        Self {
            name: name.into(),
            memory_mb,
            compute_capability: None,
            supports_bfloat16,
            driver_version: None,
            cuda_version: None,
        }
    }

    pub fn rtx_4090() -> Self {
        Self {
            name: "NVIDIA GeForce RTX 4090".into(),
            memory_mb: 24576,
            compute_capability: Some("8.9".into()),
            supports_bfloat16: true,
            driver_version: None,
            cuda_version: Some("12.2".into()),
        }
    }

    pub fn a100_40gb() -> Self {
        Self {
            name: "NVIDIA A100-SXM4-40GB".into(),
            memory_mb: 40960,
            compute_capability: Some("8.0".into()),
            supports_bfloat16: true,
            driver_version: None,
            cuda_version: Some("12.2".into()),
        }
    }

    pub fn tesla_t4() -> Self {
        Self {
            name: "Tesla T4".into(),
            memory_mb: 15360,
            compute_capability: Some("7.5".into()),
            supports_bfloat16: false,
            driver_version: None,
            cuda_version: Some("12.0".into()),
        }
    }

    pub fn tesla_p100() -> Self {
        Self {
            name: "Tesla P100-PCIE-16GB".into(),
            memory_mb: 16280,
            compute_capability: Some("6.0".into()),
            supports_bfloat16: false,
            driver_version: None,
            cuda_version: Some("12.0".into()),
        }
    }
}

/// Paid on-demand cloud GPU pod lease record (RunPod / Vast.ai)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudPodLease {
    pub lease_id: String,
    pub provider: ComputeBackendKind,
    pub external_pod_id: String,
    pub host_address: Option<String>,
    pub ssh_port: Option<u16>,
    pub cost_per_hour_usd: f32,
    pub max_budget_usd: Option<f32>,
    pub auto_stop_minutes: u32,
    pub started_at: DateTime<Utc>,
    pub status: CloudPodStatus,
}

impl CloudPodLease {
    pub fn new(
        lease_id: impl Into<String>,
        provider: ComputeBackendKind,
        external_pod_id: impl Into<String>,
        cost_per_hour: f32,
    ) -> Self {
        Self {
            lease_id: lease_id.into(),
            provider,
            external_pod_id: external_pod_id.into(),
            host_address: None,
            ssh_port: None,
            cost_per_hour_usd: cost_per_hour,
            max_budget_usd: None,
            auto_stop_minutes: 30,
            started_at: Utc::now(),
            status: CloudPodStatus::Provisioning,
        }
    }

    pub fn with_budget(mut self, max_budget_usd: f32) -> Self {
        self.max_budget_usd = Some(max_budget_usd);
        self
    }

    pub fn with_auto_stop(mut self, minutes: u32) -> Self {
        self.auto_stop_minutes = minutes;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudPodStatus {
    Provisioning,
    Running,
    Computing,
    Stopping,
    Stopped,
    Terminated,
}

/// Device architecture or acceleration backend kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Cpu,
    Cuda,
    Mps,
    Rocm,
}

impl std::fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "cpu"),
            Self::Cuda => write!(f, "cuda"),
            Self::Mps => write!(f, "mps"),
            Self::Rocm => write!(f, "rocm"),
        }
    }
}

/// Logical descriptor of an inventoried compute device (CPU or accelerator)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeDeviceRef {
    pub device_id: String,
    pub backend: ComputeBackendKind,
    pub device_kind: DeviceKind,
    pub ordinal: u32,
    pub name: String,
    pub total_memory_mb: u64,
    pub available_memory_mb: Option<u64>,
    pub bus_id: Option<String>,
    pub uuid: Option<String>,
}

impl ComputeDeviceRef {
    pub fn cpu() -> Self {
        Self {
            device_id: "dev_cpu_0".into(),
            backend: ComputeBackendKind::Local,
            device_kind: DeviceKind::Cpu,
            ordinal: 0,
            name: "Host CPU".into(),
            total_memory_mb: 16384,
            available_memory_mb: Some(8192),
            bus_id: None,
            uuid: None,
        }
    }

    pub fn apple_mps(memory_mb: u64) -> Self {
        Self {
            device_id: "dev_mps_0".into(),
            backend: ComputeBackendKind::Local,
            device_kind: DeviceKind::Mps,
            ordinal: 0,
            name: "Apple Silicon Metal (MPS)".into(),
            total_memory_mb: memory_mb,
            available_memory_mb: Some(memory_mb * 3 / 4),
            bus_id: None,
            uuid: None,
        }
    }

    pub fn cuda(
        ordinal: u32,
        name: impl Into<String>,
        memory_mb: u64,
        uuid: Option<String>,
    ) -> Self {
        Self {
            device_id: format!("dev_cuda_{ordinal}"),
            backend: ComputeBackendKind::Local,
            device_kind: DeviceKind::Cuda,
            ordinal,
            name: name.into(),
            total_memory_mb: memory_mb,
            available_memory_mb: Some(memory_mb),
            bus_id: None,
            uuid,
        }
    }
}

