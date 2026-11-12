//! Hosted Compute & GPU Adapters (F10 / P3).
//!
//! Concrete adapters for:
//! 1. `KaggleTargetAdapter` — Kaggle Kernels via Kaggle CLI / REST API.
//! 2. `ColabTargetAdapter` — Google Colab runtimes via Colab CLI (`google-colab-cli`).
//! 3. `RentedGpuTargetAdapter` — External rented GPUs (RunPod, Vast.ai, Lambda Cloud, Dedicated SSH).

pub mod checkpoint;
pub mod checkpoint_store;
pub mod colab;
pub mod kaggle;
pub mod rented_gpu;

pub use checkpoint_store::CasCheckpointStoreAdapter;
pub use colab::ColabTargetAdapter;
pub use kaggle::KaggleTargetAdapter;
pub use rented_gpu::RentedGpuTargetAdapter;

use std::path::Path;
use std::sync::Arc;
use custos_core::contracts::compute::{ExecutionTargetPort, ExecutionTargetResolver};
use custos_domain::ComputeBackendKind;

/// Default production execution target resolver instantiating live adapters without mock bypasses.
#[derive(Debug, Default, Clone)]
pub struct DefaultExecutionTargetResolver;

impl ExecutionTargetResolver for DefaultExecutionTargetResolver {
    fn resolve(
        &self,
        backend: ComputeBackendKind,
        work_dir: &Path,
        ssh_host: Option<&str>,
        ssh_port: Option<u16>,
        ssh_user: Option<&str>,
        pod_id: Option<&str>,
        cloud_api_key: Option<&str>,
    ) -> Arc<dyn ExecutionTargetPort> {
        match backend {
            ComputeBackendKind::Colab => Arc::new(ColabTargetAdapter::new(work_dir)),
            ComputeBackendKind::Kaggle => Arc::new(KaggleTargetAdapter::new(work_dir)),
            ComputeBackendKind::RunPod
            | ComputeBackendKind::VastAi
            | ComputeBackendKind::DedicatedSsh => {
                let host = ssh_host.unwrap_or("127.0.0.1");
                let port = ssh_port.unwrap_or(22);
                let user = ssh_user.unwrap_or("root");
                let mut a = RentedGpuTargetAdapter::new(host, port, user, backend, work_dir);
                if let Some(p) = pod_id {
                    a = a.with_pod_lease(p, cloud_api_key.map(|s| s.to_string()));
                }
                Arc::new(a)
            }
            _ => Arc::new(ColabTargetAdapter::new(work_dir)),
        }
    }
}

/// Mock execution target resolver for isolated testing environments.
#[derive(Debug, Default, Clone)]
pub struct MockExecutionTargetResolver;

impl ExecutionTargetResolver for MockExecutionTargetResolver {
    fn resolve(
        &self,
        backend: ComputeBackendKind,
        work_dir: &Path,
        ssh_host: Option<&str>,
        ssh_port: Option<u16>,
        ssh_user: Option<&str>,
        pod_id: Option<&str>,
        cloud_api_key: Option<&str>,
    ) -> Arc<dyn ExecutionTargetPort> {
        match backend {
            ComputeBackendKind::Colab => Arc::new(ColabTargetAdapter::new(work_dir).with_mock_mode()),
            ComputeBackendKind::Kaggle => Arc::new(KaggleTargetAdapter::new(work_dir).with_mock_mode()),
            ComputeBackendKind::RunPod
            | ComputeBackendKind::VastAi
            | ComputeBackendKind::DedicatedSsh => {
                let host = ssh_host.unwrap_or("127.0.0.1");
                let port = ssh_port.unwrap_or(22);
                let user = ssh_user.unwrap_or("root");
                let mut a = RentedGpuTargetAdapter::new(host, port, user, backend, work_dir);
                if let Some(p) = pod_id {
                    a = a.with_pod_lease(p, cloud_api_key.map(|s| s.to_string()));
                }
                Arc::new(a.with_mock_mode())
            }
            _ => Arc::new(ColabTargetAdapter::new(work_dir).with_mock_mode()),
        }
    }
}

