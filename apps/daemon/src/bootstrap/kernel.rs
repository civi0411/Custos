//! Kernel and Isolation Services Bootstrap (SADE R8 §6.8)

use std::path::Path;
use std::sync::Arc;

use custos_engine::bridge::BridgeService;
use custos_engine::contracts::kernel::TrustedKernel;
use custos_engine::session::SessionManager;
use custos_engine::TaskService;
use custos_integrations::documents::SourceFetchAdapter;
use custos_integrations::sandbox::SovereignDeveloperAdapter;
use custos_storage::SqliteTaskStore;

pub struct KernelServices {
    pub task_service: Arc<TaskService>,
    pub session_manager: Arc<SessionManager>,
    pub bridge_service: Arc<BridgeService>,
    pub kernel: Arc<TrustedKernel>,
    pub sandbox: Arc<SovereignDeveloperAdapter>,
    pub live_runs: custos_engine::workflow::live::LiveRuns,
    pub supervision: Arc<custos_engine::workflow::SupervisionCoordinator>,
}

pub fn init_kernel_services(store: Arc<SqliteTaskStore>, workspace_root: &Path) -> KernelServices {
    let task_service = Arc::new(TaskService::new(store.clone()));
    let session_manager = Arc::new(SessionManager::with_store(store.clone()));
    let bridge_service = Arc::new(BridgeService::new(
        session_manager.clone(),
        task_service.clone(),
    ));
    let live_runs = custos_engine::workflow::live::LiveRuns::default();
    let supervision = Arc::new(custos_engine::workflow::SupervisionCoordinator::new(
        Some(store.clone()),
        live_runs.clone(),
    ));
    supervision.ensure_started();

    let source_acq = Arc::new(SourceFetchAdapter::with_base_dir(workspace_root));
    let kernel = Arc::new(TrustedKernel::with_source_acquisition(store, source_acq));
    let sandbox = Arc::new(SovereignDeveloperAdapter::new(workspace_root.to_path_buf()));

    KernelServices {
        task_service,
        session_manager,
        bridge_service,
        kernel,
        sandbox,
        live_runs,
        supervision,
    }
}
