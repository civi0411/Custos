//! Environment Coordinator Subsystem (AEE)
//!
//! Orchestrates the end-to-end provisioning, invariant verification, and teardown
//! of sovereign Agent Execution Environments across all four Custos lenses.

use async_trait::async_trait;
use custos_core::authority::EnvironmentGate;
use custos_core::contracts::environment::{EnvironmentHandle, EnvironmentPort, GateReport};
use custos_core::contracts::terminal::TerminalPort;
use custos_domain::environment::{EnvironmentProfile, EnvironmentSpec, EnvironmentState};
use custos_domain::{DomainError, ExecutionWorkspace, WorkspaceKind, WorkspaceStatus};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, OwnedMutexGuard};
use tracing::{info, warn};

use crate::workspace::coordinator::{CreateWorkspaceRequest, WorkspaceCoordinator};

pub struct EnvironmentCoordinator {
    workspace_coordinator: Arc<WorkspaceCoordinator>,
    terminal_port: Option<Arc<dyn TerminalPort>>,
    handles: Mutex<HashMap<String, EnvironmentHandle>>,
    lifecycle_locks: Mutex<HashMap<String, Weak<Mutex<()>>>>,
}

impl EnvironmentCoordinator {
    pub fn new(
        workspace_coordinator: Arc<WorkspaceCoordinator>,
        terminal_port: Option<Arc<dyn TerminalPort>>,
    ) -> Self {
        Self {
            workspace_coordinator,
            terminal_port,
            handles: Mutex::new(HashMap::new()),
            lifecycle_locks: Mutex::new(HashMap::new()),
        }
    }

    async fn lock_lifecycle(&self, env_id: &str) -> OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.lifecycle_locks.lock().await;
            locks.retain(|_, lock| lock.strong_count() > 0);
            let lock = locks
                .get(env_id)
                .and_then(Weak::upgrade)
                .unwrap_or_default();
            locks.insert(env_id.to_owned(), Arc::downgrade(&lock));
            lock
        };
        lock.lock_owned().await
    }

    async fn workspace_for_handle(
        &self,
        handle: &EnvironmentHandle,
    ) -> Result<ExecutionWorkspace, DomainError> {
        let workspace = if let Some(id) = &handle.workspace_id {
            self.workspace_coordinator.get_workspace(id).await?
        } else {
            let mut matches = self
                .workspace_coordinator
                .list_workspaces()
                .await?
                .into_iter()
                .filter(|workspace| {
                    workspace.metadata.get("env_id").and_then(|v| v.as_str())
                        == Some(handle.env_id.as_str())
                });
            let workspace = matches.next();
            if matches.next().is_some() {
                return Err(DomainError::Conflict(format!(
                    "Multiple workspace bindings for environment '{}'",
                    handle.env_id
                )));
            }
            workspace
        }
        .ok_or_else(|| DomainError::NotFound {
            kind: "EnvironmentWorkspace".into(),
            id: handle.env_id.clone(),
        })?;
        if workspace.metadata.get("env_id").and_then(|v| v.as_str()) != Some(handle.env_id.as_str())
        {
            return Err(DomainError::Validation(
                "Environment workspace binding mismatch".into(),
            ));
        }
        if let Some(active) = self.get_handle(&handle.env_id).await {
            if active.workspace_id.as_ref() != Some(&workspace.id) {
                return Err(DomainError::Conflict(
                    "Environment handle refers to an earlier workspace binding".into(),
                ));
            }
        }
        Ok(workspace)
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

        let _lifecycle = self.lock_lifecycle(&spec.env_id).await;
        if self.get_handle(&spec.env_id).await.is_some() {
            return Err(DomainError::Conflict(format!(
                "Environment '{}' is already provisioned",
                spec.env_id
            )));
        }

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
                (
                    WorkspaceKind::Folder {
                        path: folder.clone(),
                    },
                    folder,
                )
            }
            EnvironmentProfile::Research { corpus_path, .. } => (
                WorkspaceKind::Folder {
                    path: corpus_path.clone(),
                },
                corpus_path.clone(),
            ),
            EnvironmentProfile::Quant { data_partition, .. } => {
                let folder = format!("data/quant/{}", data_partition);
                (
                    WorkspaceKind::Folder {
                        path: folder.clone(),
                    },
                    folder,
                )
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
                warn!(
                    "Workspace allocation failed for env '{}': {}",
                    spec.env_id, e
                );
                DomainError::Validation(format!("Failed to provision workspace: {e}"))
            })?;

        let assurance = match self
            .workspace_coordinator
            .inspect_environment_assurance(&ws.id)
            .await
        {
            Ok(assurance) => assurance,
            Err(error) => {
                warn!(env_id = %spec.env_id, %error, "Environment assurance unavailable");
                custos_domain::environment::EnvironmentAssurance::unconstrained()
            }
        };

        // Step 3: Run invariant gates
        let gate_report = EnvironmentGate::evaluate(spec, &EnvironmentState::Ready, &assurance);

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
            workspace_id: Some(ws.id),
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
        let _lifecycle = self.lock_lifecycle(&handle.env_id).await;
        if handle.env_id != spec.env_id {
            return Err(DomainError::Validation(
                "Environment specification identity mismatch".into(),
            ));
        }
        let workspace = self.workspace_for_handle(handle).await?;
        let assurance = self
            .workspace_coordinator
            .inspect_environment_assurance(&workspace.id)
            .await?;
        let state = if workspace.status == WorkspaceStatus::Archived {
            &EnvironmentState::TornDown
        } else {
            &handle.state
        };
        Ok(EnvironmentGate::evaluate(spec, state, &assurance))
    }

    async fn teardown(&self, handle: &EnvironmentHandle) -> Result<(), DomainError> {
        info!(
            "Tearing down Agent Execution Environment '{}'",
            handle.env_id
        );

        let _lifecycle = self.lock_lifecycle(&handle.env_id).await;
        let workspace = self.workspace_for_handle(handle).await?;
        // Environment release archives the binding. Physical deletion is an explicit workspace operation.
        if workspace.status != WorkspaceStatus::Archived {
            self.workspace_coordinator
                .archive_workspace(&workspace.id, false)
                .await?;
        }
        self.handles.lock().await.remove(&handle.env_id);

        Ok(())
    }
}

#[cfg(test)]
#[path = "coordinator_tests.rs"]
mod tests;
