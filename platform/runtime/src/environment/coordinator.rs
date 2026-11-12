//! Environment Coordinator Subsystem (AEE)
//!
//! Orchestrates the end-to-end provisioning, invariant verification, and teardown
//! of sovereign Agent Execution Environments across all four Custos lenses.

use async_trait::async_trait;
use custos_core::contracts::environment::{EnvironmentHandle, EnvironmentPort, GateReport};
use custos_core::contracts::terminal::TerminalPort;
use custos_core::authority::EnvironmentGate;
use custos_domain::authority::action::Assurance;
use custos_domain::environment::{
    EnvironmentAssurance, EnvironmentProfile, EnvironmentSpec, EnvironmentState,
};
use custos_domain::{DomainError, WorkspaceId, WorkspaceKind};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::workspace::coordinator::{CreateWorkspaceRequest, WorkspaceCoordinator};

pub struct EnvironmentCoordinator {
    workspace_coordinator: Arc<WorkspaceCoordinator>,
    terminal_port: Option<Arc<dyn TerminalPort>>,
    handles: Arc<Mutex<HashMap<String, EnvironmentHandle>>>,
}

impl EnvironmentCoordinator {
    pub fn new(
        workspace_coordinator: Arc<WorkspaceCoordinator>,
        terminal_port: Option<Arc<dyn TerminalPort>>,
    ) -> Self {
        Self {
            workspace_coordinator,
            terminal_port,
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Helper to get an active environment handle
    pub async fn get_handle(&self, env_id: &str) -> Option<EnvironmentHandle> {
        let guard = self.handles.lock().await;
        guard.get(env_id).cloned()
    }

    /// Lists all active environment handles
    pub async fn list_handles(&self) -> Vec<EnvironmentHandle> {
        let guard = self.handles.lock().await;
        guard.values().cloned().collect()
    }

    /// Access the bound terminal port driver
    pub fn terminal_port(&self) -> Option<Arc<dyn TerminalPort>> {
        self.terminal_port.clone()
    }
}

#[async_trait]
impl EnvironmentPort for EnvironmentCoordinator {
    async fn provision(&self, spec: &EnvironmentSpec) -> Result<EnvironmentHandle, DomainError> {
        info!("Provisioning Agent Execution Environment '{}'", spec.env_id);

        let (kind, ws_path) = match &spec.profile {
            EnvironmentProfile::Coding {
                repo_path,
                branch,
                worktree_path,
            } => {
                let kind = WorkspaceKind::Git {
                    repo_path: repo_path.clone(),
                    branch: branch.clone(),
                    base_commit: None,
                };
                (kind, worktree_path.clone())
            }
            EnvironmentProfile::Assistant { .. } => {
                let folder = format!("/tmp/custos_assistant_{}", spec.env_id);
                (WorkspaceKind::Folder { path: folder.clone() }, folder)
            }
            EnvironmentProfile::Research { corpus_path, .. } => (
                WorkspaceKind::Folder {
                    path: corpus_path.clone(),
                },
                corpus_path.clone(),
            ),
            EnvironmentProfile::Quant { data_partition, .. } => {
                let folder = format!("data/quant/{}", data_partition);
                (WorkspaceKind::Folder { path: folder.clone() }, folder)
            }
        };

        // Step 1: Provision physical/logical workspace via WorkspaceCoordinator
        let ws_req = CreateWorkspaceRequest {
            name: format!("env-ws-{}", spec.env_id),
            kind,
            path: ws_path.clone(),
            lineage: None,
            owner_task_id: spec.owner_task_id.clone(),
            metadata: Some(serde_json::json!({ "env_id": spec.env_id })),
            setup_script: None,
        };

        let ws = self
            .workspace_coordinator
            .create_workspace(ws_req)
            .await
            .map_err(|e| {
                warn!("Workspace allocation failed for env '{}': {}", spec.env_id, e);
                DomainError::Validation(format!("Failed to provision workspace: {e}"))
            })?;

        // Step 2: Determine measured assurance
        let (network_restricted, isolation_driver) = match &spec.profile {
            EnvironmentProfile::Quant {
                is_live_gateway: false,
                ..
            } => (true, "quant-airgap".to_string()),
            EnvironmentProfile::Coding { .. } => (false, "worktree-seatbelt".to_string()),
            _ => (false, "standard".to_string()),
        };

        let assurance = EnvironmentAssurance {
            measured_assurance: Assurance::CustosMediated,
            filesystem_confined: spec.profile.requires_worktree(),
            network_restricted,
            process_isolated: true,
            isolation_driver,
        };

        // Step 3: Run invariant gates
        let gate_report =
            EnvironmentGate::evaluate(spec, &EnvironmentState::Ready, &assurance);

        let final_state = if gate_report.passed {
            EnvironmentState::Ready
        } else {
            warn!(
                "Environment '{}' degraded due to invariant violations: {:?}",
                spec.env_id, gate_report.violations
            );
            EnvironmentState::Degraded {
                reason: gate_report.violations.join("; "),
            }
        };

        let handle = EnvironmentHandle {
            env_id: spec.env_id.clone(),
            working_dir: ws.path,
            state: final_state,
            assurance,
        };

        let mut guard = self.handles.lock().await;
        guard.insert(spec.env_id.clone(), handle.clone());

        Ok(handle)
    }

    async fn verify(
        &self,
        handle: &EnvironmentHandle,
        spec: &EnvironmentSpec,
    ) -> Result<GateReport, DomainError> {
        Ok(EnvironmentGate::evaluate(spec, &handle.state, &handle.assurance))
    }

    async fn teardown(&self, handle: &EnvironmentHandle) -> Result<(), DomainError> {
        info!("Tearing down Agent Execution Environment '{}'", handle.env_id);

        let mut guard = self.handles.lock().await;
        if let Some(mut existing) = guard.remove(&handle.env_id) {
            existing.state = EnvironmentState::TornDown;
        }

        // Archive / release associated workspace
        let _ = self
            .workspace_coordinator
            .archive_workspace(&WorkspaceId(format!("env-ws-{}", handle.env_id)), true)
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core::contracts::workspace::{WorkspaceProvider, WorkspaceRepository};
    use custos_domain::{ExecutionWorkspace, WorkspaceStatus};

    struct MockWorkspaceRepo {
        workspaces: Mutex<HashMap<WorkspaceId, ExecutionWorkspace>>,
    }

    impl MockWorkspaceRepo {
        fn new() -> Self {
            Self {
                workspaces: Mutex::new(HashMap::new()),
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

    struct MockWorkspaceProv;

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
        async fn teardown(&self, _ws: &ExecutionWorkspace) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_environment_coordinator_provision_verify_teardown_flow() {
        let repo = Arc::new(MockWorkspaceRepo::new());
        let prov = Arc::new(MockWorkspaceProv);
        let ws_coord = Arc::new(WorkspaceCoordinator::new(repo, prov));
        let env_coord = EnvironmentCoordinator::new(ws_coord, None);

        let spec = EnvironmentSpec::new(
            "env_test_01",
            EnvironmentProfile::Assistant { project_id: Some("proj_1".into()) },
        );

        let handle = env_coord.provision(&spec).await.expect("Provision must succeed");
        assert_eq!(handle.state, EnvironmentState::Ready);
        assert_eq!(handle.assurance.measured_assurance, Assurance::CustosMediated);

        let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
        assert!(verify_report.passed);

        env_coord.teardown(&handle).await.expect("Teardown must succeed");
        assert!(env_coord.get_handle(&spec.env_id).await.is_none());
    }

    #[tokio::test]
    async fn test_environment_coordinator_coding_lens_worktree_isolation() {
        let repo = Arc::new(MockWorkspaceRepo::new());
        let prov = Arc::new(MockWorkspaceProv);
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

        let handle = env_coord.provision(&spec).await.expect("Provision coding env");
        assert_eq!(handle.state, EnvironmentState::Ready);
        assert!(handle.assurance.filesystem_confined);
        assert!(!handle.assurance.network_restricted);
        assert_eq!(handle.working_dir, "/worktrees/custos_sade");

        let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
        assert!(verify_report.passed);
        assert!(verify_report.violations.is_empty());
    }

    #[tokio::test]
    async fn test_environment_coordinator_quant_lens_airgap() {
        let repo = Arc::new(MockWorkspaceRepo::new());
        let prov = Arc::new(MockWorkspaceProv);
        let ws_coord = Arc::new(WorkspaceCoordinator::new(repo, prov));
        let env_coord = EnvironmentCoordinator::new(ws_coord, None);

        let spec = EnvironmentSpec::new(
            "env_quant_backtest",
            EnvironmentProfile::Quant {
                data_partition: "2024-Q1".into(),
                is_live_gateway: false,
            },
        );

        let handle = env_coord.provision(&spec).await.expect("Provision quant env");
        assert_eq!(handle.state, EnvironmentState::Ready);
        assert!(handle.assurance.network_restricted);
        assert_eq!(handle.assurance.isolation_driver, "quant-airgap");

        let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
        assert!(verify_report.passed);
    }

    #[tokio::test]
    async fn test_environment_coordinator_research_lens_corpus() {
        let repo = Arc::new(MockWorkspaceRepo::new());
        let prov = Arc::new(MockWorkspaceProv);
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

        let handle = env_coord.provision(&spec).await.expect("Provision research env");
        assert_eq!(handle.state, EnvironmentState::Ready);
        assert_eq!(handle.working_dir, "/data/papers/attention");

        let verify_report = env_coord.verify(&handle, &spec).await.unwrap();
        assert!(verify_report.passed);
    }

    #[tokio::test]
    async fn test_environment_coordinator_degraded_on_invariant_violation() {
        let repo = Arc::new(MockWorkspaceRepo::new());
        let prov = Arc::new(MockWorkspaceProv);
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

        let handle = env_coord.provision(&spec).await.expect("Provision must proceed");
        // State must be Degraded according to ENV-07
        match &handle.state {
            EnvironmentState::Degraded { reason } => {
                assert!(reason.contains("ENV-02"));
            }
            other => panic!("Expected Degraded state, got {:?}", other),
        }
    }
}

