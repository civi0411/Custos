use super::*;
use custos_core::contracts::workspace::{WorkspaceProvider, WorkspaceRepository};
use custos_domain::authority::action::Assurance;
use custos_domain::environment::EnvironmentAssurance;
use custos_domain::WorkspaceId;
use custos_domain::{ExecutionWorkspace, WorkspaceStatus};
use std::sync::atomic::{AtomicBool, Ordering};

struct MockWorkspaceRepo {
    workspaces: Mutex<HashMap<WorkspaceId, ExecutionWorkspace>>,
    fail_archive: AtomicBool,
}

impl MockWorkspaceRepo {
    fn new() -> Self {
        Self {
            workspaces: Mutex::new(HashMap::new()),
            fail_archive: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl WorkspaceRepository for MockWorkspaceRepo {
    async fn save_workspace(&self, ws: &ExecutionWorkspace) -> Result<(), DomainError> {
        let mut g = self.workspaces.lock().await;
        g.insert(ws.id.clone(), ws.clone());
        Ok(())
    }
    async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError> {
        let g = self.workspaces.lock().await;
        Ok(g.get(id).cloned())
    }
    async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        let g = self.workspaces.lock().await;
        Ok(g.values().cloned().collect())
    }
    async fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError> {
        if self.fail_archive.load(Ordering::SeqCst) && status == WorkspaceStatus::Archived {
            return Err(DomainError::Conflict(
                "Archive temporarily unavailable".into(),
            ));
        }
        let mut g = self.workspaces.lock().await;
        if let Some(ws) = g.get_mut(id) {
            ws.status = status;
        }
        Ok(())
    }
    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError> {
        let mut g = self.workspaces.lock().await;
        g.remove(id);
        Ok(())
    }
}

#[derive(Default)]
struct MockWorkspaceProv {
    assurance: Option<EnvironmentAssurance>,
}

#[async_trait]
impl WorkspaceProvider for MockWorkspaceProv {
    async fn validate(&self, _kind: &WorkspaceKind, _path: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn provision(&self, _ws: &ExecutionWorkspace) -> Result<(), DomainError> {
        Ok(())
    }
    async fn setup(
        &self,
        _ws: &ExecutionWorkspace,
        _script: Option<&str>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn inspect_environment_assurance(
        &self,
        _workspace: &ExecutionWorkspace,
    ) -> Result<EnvironmentAssurance, DomainError> {
        Ok(self
            .assurance
            .clone()
            .unwrap_or_else(EnvironmentAssurance::unconstrained))
    }
    async fn teardown(&self, _ws: &ExecutionWorkspace) -> Result<(), DomainError> {
        Err(DomainError::Conflict(
            "Environment release must retain workspace files".into(),
        ))
    }
}

#[tokio::test]
async fn test_environment_coordinator_provision_verify_teardown_flow() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let prov = Arc::new(MockWorkspaceProv::default());
    let ws_coord = Arc::new(WorkspaceCoordinator::new(repo, prov));
    let env_coord = EnvironmentCoordinator::new(ws_coord, None);

    let spec = EnvironmentSpec::new(
        "env_test_01",
        EnvironmentProfile::Assistant {
            project_id: Some("proj_1".into()),
        },
    );

    let handle = env_coord
        .provision(&spec)
        .await
        .expect("Provision must succeed");
    assert!(matches!(handle.state, EnvironmentState::Degraded { .. }));
    assert_eq!(handle.assurance.measured_assurance, Assurance::ObserveOnly);

    let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
    assert!(!verify_report.passed);

    env_coord
        .teardown(&handle)
        .await
        .expect("Teardown must succeed");
    assert!(env_coord.get_handle(&spec.env_id).await.is_none());
}

#[tokio::test]
async fn coding_worktree_does_not_imply_filesystem_confinement() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let prov = Arc::new(MockWorkspaceProv::default());
    let ws_coord = Arc::new(WorkspaceCoordinator::new(repo.clone(), prov));
    let env_coord = EnvironmentCoordinator::new(ws_coord, None);

    let spec = EnvironmentSpec::new(
        "env_coding_01",
        EnvironmentProfile::Coding {
            repo_path: "/repos/custos".into(),
            branch: "feat/sade-env".into(),
            worktree_path: "/worktrees/custos_sade".into(),
        },
    );

    let handle = env_coord
        .provision(&spec)
        .await
        .expect("Provision coding env");
    assert!(matches!(handle.state, EnvironmentState::Degraded { .. }));
    assert!(!handle.assurance.filesystem_confined);
    assert!(!handle.assurance.network_restricted);
    assert_eq!(handle.working_dir, "/worktrees/custos_sade");

    let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
    assert!(!verify_report.passed);
    assert!(verify_report
        .violations
        .iter()
        .any(|v| v.contains("ENV-03")));
}

#[tokio::test]
async fn quant_profile_does_not_imply_network_isolation() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let prov = Arc::new(MockWorkspaceProv::default());
    let ws_coord = Arc::new(WorkspaceCoordinator::new(repo, prov));
    let env_coord = EnvironmentCoordinator::new(ws_coord, None);

    let spec = EnvironmentSpec::new(
        "env_quant_backtest",
        EnvironmentProfile::Quant {
            data_partition: "2024-Q1".into(),
            is_live_gateway: false,
        },
    );

    let handle = env_coord
        .provision(&spec)
        .await
        .expect("Provision quant env");
    assert!(matches!(handle.state, EnvironmentState::Degraded { .. }));
    assert!(!handle.assurance.network_restricted);
    assert_eq!(handle.assurance.isolation_driver, "none");

    let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
    assert!(!verify_report.passed);
}

#[tokio::test]
async fn test_environment_coordinator_research_lens_corpus() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let prov = Arc::new(MockWorkspaceProv::default());
    let ws_coord = Arc::new(WorkspaceCoordinator::new(repo, prov));
    let env_coord = EnvironmentCoordinator::new(ws_coord, None);

    let spec = EnvironmentSpec::new(
        "env_research_corpus",
        EnvironmentProfile::Research {
            corpus_path: "/data/papers/attention".into(),
            kernel_spec: Some("python3".into()),
            dataset_hash: Some("sha256:abc123".into()),
        },
    );

    let handle = env_coord
        .provision(&spec)
        .await
        .expect("Provision research env");
    assert!(matches!(handle.state, EnvironmentState::Degraded { .. }));
    assert_eq!(handle.working_dir, "/data/papers/attention");

    let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
    assert!(!verify_report.passed);
}

#[tokio::test]
async fn test_environment_coordinator_degraded_on_invariant_violation() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let prov = Arc::new(MockWorkspaceProv::default());
    let ws_coord = Arc::new(WorkspaceCoordinator::new(repo, prov));
    let env_coord = EnvironmentCoordinator::new(ws_coord, None);

    // Coding profile with empty branch violates ENV-02
    let spec = EnvironmentSpec::new(
        "env_coding_invalid",
        EnvironmentProfile::Coding {
            repo_path: "/repos/custos".into(),
            branch: "".into(),
            worktree_path: "/worktrees/custos".into(),
        },
    );

    let handle = env_coord
        .provision(&spec)
        .await
        .expect("Provision must proceed");
    // State must be Degraded according to ENV-07
    match &handle.state {
        EnvironmentState::Degraded { reason } => {
            assert!(reason.contains("ENV-02"));
        }
        other => panic!("Expected Degraded state, got {:?}", other),
    }
}

#[tokio::test]
async fn provider_assurance_is_reported_without_profile_inference() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let mut assurance = EnvironmentAssurance::unconstrained();
    assurance.measured_assurance = Assurance::CustosMediated;
    assurance.isolation_driver = "tested-provider".into();
    let provider = Arc::new(MockWorkspaceProv {
        assurance: Some(assurance.clone()),
    });
    let workspace = Arc::new(WorkspaceCoordinator::new(repo, provider));
    let coordinator = EnvironmentCoordinator::new(workspace, None);
    let spec = EnvironmentSpec::new(
        "measured",
        EnvironmentProfile::Assistant { project_id: None },
    );
    let handle = coordinator.provision(&spec).await.unwrap();
    assert_eq!(handle.assurance, assurance);
    assert_eq!(handle.state, EnvironmentState::Ready);
    assert!(coordinator.verify(&handle, &spec).await.unwrap().passed);
}

#[tokio::test]
async fn teardown_archives_generated_workspace_and_is_repeatable() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let provider = Arc::new(MockWorkspaceProv::default());
    let workspace = Arc::new(WorkspaceCoordinator::new(repo.clone(), provider));
    let coordinator = EnvironmentCoordinator::new(workspace, None);
    let spec = EnvironmentSpec::new("bound", EnvironmentProfile::Assistant { project_id: None });
    let handle = coordinator.provision(&spec).await.unwrap();
    let id = handle.workspace_id.clone().unwrap();
    assert_ne!(id.to_string(), "env-ws-bound");
    coordinator.teardown(&handle).await.unwrap();
    coordinator.teardown(&handle).await.unwrap();
    assert_eq!(
        repo.get_workspace(&id).await.unwrap().unwrap().status,
        WorkspaceStatus::Archived
    );
    assert!(coordinator.get_handle("bound").await.is_none());
}

#[tokio::test]
async fn cleanup_failure_keeps_handle_for_retry() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let workspace = Arc::new(WorkspaceCoordinator::new(
        repo.clone(),
        Arc::new(MockWorkspaceProv::default()),
    ));
    let coordinator = EnvironmentCoordinator::new(workspace, None);
    let spec = EnvironmentSpec::new("retry", EnvironmentProfile::Assistant { project_id: None });
    let handle = coordinator.provision(&spec).await.unwrap();
    repo.fail_archive.store(true, Ordering::SeqCst);
    assert!(coordinator.teardown(&handle).await.is_err());
    assert!(coordinator.get_handle("retry").await.is_some());
    repo.fail_archive.store(false, Ordering::SeqCst);
    coordinator.teardown(&handle).await.unwrap();
    assert!(coordinator.get_handle("retry").await.is_none());
}

#[tokio::test]
async fn legacy_handle_resolves_persisted_metadata_after_coordinator_restart() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let workspace = Arc::new(WorkspaceCoordinator::new(
        repo.clone(),
        Arc::new(MockWorkspaceProv::default()),
    ));
    let coordinator = EnvironmentCoordinator::new(workspace.clone(), None);
    let spec = EnvironmentSpec::new("legacy", EnvironmentProfile::Assistant { project_id: None });
    let handle = coordinator.provision(&spec).await.unwrap();
    let id = handle.workspace_id.clone().unwrap();
    let mut json = serde_json::to_value(handle).unwrap();
    json.as_object_mut().unwrap().remove("workspace_id");
    let legacy: EnvironmentHandle = serde_json::from_value(json).unwrap();
    let restarted = EnvironmentCoordinator::new(workspace, None);
    restarted.teardown(&legacy).await.unwrap();
    assert_eq!(
        repo.get_workspace(&id).await.unwrap().unwrap().status,
        WorkspaceStatus::Archived
    );
}

#[tokio::test]
async fn concurrent_provision_cannot_overwrite_live_binding() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let workspace = Arc::new(WorkspaceCoordinator::new(
        repo.clone(),
        Arc::new(MockWorkspaceProv::default()),
    ));
    let coordinator = EnvironmentCoordinator::new(workspace, None);
    let spec = EnvironmentSpec::new(
        "concurrent",
        EnvironmentProfile::Assistant { project_id: None },
    );
    let (first, second) = tokio::join!(coordinator.provision(&spec), coordinator.provision(&spec));
    assert_ne!(first.is_ok(), second.is_ok());
    assert_eq!(repo.list_workspaces().await.unwrap().len(), 1);
}

#[tokio::test]
async fn stale_handle_cannot_release_replacement_environment() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let workspace = Arc::new(WorkspaceCoordinator::new(
        repo.clone(),
        Arc::new(MockWorkspaceProv::default()),
    ));
    let coordinator = EnvironmentCoordinator::new(workspace, None);
    let spec = EnvironmentSpec::new(
        "replacement",
        EnvironmentProfile::Assistant { project_id: None },
    );
    let old = coordinator.provision(&spec).await.unwrap();
    coordinator.teardown(&old).await.unwrap();
    let replacement = coordinator.provision(&spec).await.unwrap();
    assert!(coordinator.teardown(&old).await.is_err());
    assert_eq!(
        coordinator.get_handle(&spec.env_id).await.unwrap(),
        replacement
    );
    coordinator.teardown(&replacement).await.unwrap();
}

#[tokio::test]
async fn archived_workspace_cannot_verify_as_ready() {
    let repo = Arc::new(MockWorkspaceRepo::new());
    let workspace = Arc::new(WorkspaceCoordinator::new(
        repo,
        Arc::new(MockWorkspaceProv::default()),
    ));
    let coordinator = EnvironmentCoordinator::new(workspace, None);
    let mut spec = EnvironmentSpec::new(
        "archived",
        EnvironmentProfile::Assistant { project_id: None },
    );
    spec.target_assurance = Assurance::ObserveOnly;
    let handle = coordinator.provision(&spec).await.unwrap();
    coordinator.teardown(&handle).await.unwrap();
    let report = coordinator.verify(&handle, &spec).await.unwrap();
    assert!(!report.passed);
    assert_eq!(report.state, EnvironmentState::TornDown);
}
