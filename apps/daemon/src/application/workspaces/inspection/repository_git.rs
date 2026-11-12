use crate::api::*;
use custos_adapters::host::binaries::resolve_gh_binary;
use custos_core::contracts::terminal::TerminalPort;
use custos_domain::DomainError;

impl LocalApiDispatcher {
    pub(crate) async fn handle_repository_git(&self, req: ApiRequest) -> ApiResponse {
        let workspace_id = match req.params.get("workspace_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id,
            _ => return ApiResponse::error(req.id, "An exact workspace_id is required"),
        };
        let ws = match self
            .resolve_workspace_for_dispatch(&req.id, workspace_id)
            .await
        {
            Ok(ws) => ws,
            Err(response) => return response,
        };
        if !matches!(ws.kind, custos_domain::WorkspaceKind::Git { .. }) {
            return ApiResponse::error(
                req.id,
                "Local registered Git checkout required; remote-host Git routing is not available here",
            );
        }
        let _guard = self.env.candidate_mutation.lock().await;
        let path = std::path::Path::new(&ws.path);
        let gh = resolve_gh_binary();
        let result: Result<serde_json::Value, DomainError> = async {
            use custos_adapters::repository_git as git;
            tracing::debug!(
                request_id = %req.id,
                workspace_id,
                method = %req.method,
                "Repository Git operation"
            );
            match req.method.as_str() {
                METHOD_GIT_REPOSITORY => git::repository(path).await,
                METHOD_GIT_ACCOUNT => {
                    let binding = req
                        .params
                        .get("account")
                        .filter(|v| !v.is_null())
                        .map(|v| {
                            serde_json::from_value::<git::GitHubBinding>(v.clone()).map_err(|_| {
                                DomainError::Validation("Invalid account preference".into())
                            })
                        })
                        .transpose()?;
                    git::bind_account(path, gh.as_deref(), binding.as_ref()).await?;
                    git::repository(path).await
                }
                METHOD_GIT_GITHUB => {
                    let binding = git::account_binding(path).await?.ok_or_else(|| {
                        DomainError::Validation("Choose a repository GitHub account first".into())
                    })?;
                    git::github_viewer(
                        gh.as_deref().ok_or_else(|| {
                            DomainError::Validation("GitHub CLI is not installed".into())
                        })?,
                        &binding,
                    )
                    .await
                }
                METHOD_GIT_FETCH => {
                    let remote = req
                        .params
                        .get("remote")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DomainError::Validation("Explicit remote required".into()))?;
                    git::fetch_remote(path, gh.as_deref(), remote).await
                }
                METHOD_GIT_SWITCH => {
                    let branch = req
                        .params
                        .get("branch")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DomainError::Validation("Explicit branch required".into()))?;
                    let expected = req
                        .params
                        .get("expected_head")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| DomainError::Validation("Observed HEAD required".into()))?;
                    if self
                        .env.terminal
                        .list_sessions(Some(workspace_id))
                        .await?
                        .iter()
                        .any(|s| matches!(s.status, custos_domain::TerminalSessionStatus::Active))
                    {
                        return Err(DomainError::Conflict(
                            "Close active checkout terminals before switching, or use a new worktree"
                                .into(),
                        ));
                    }
                    let runs = self.kernel.usage_runs.as_ref().ok_or_else(|| {
                        DomainError::Validation(
                            "Cannot verify active runs; branch switch unavailable".into(),
                        )
                    })?;
                    for task in self.kernel.task_service.list_tasks().await? {
                        for run in runs.list_runs_for_task(&task.id).await? {
                            let target = run
                                .metadata
                                .get("execution_workspace_id")
                                .or_else(|| run.metadata.get("workspace_id"))
                                .and_then(|v| v.as_str());
                            if !run.status.is_terminal()
                                && (target.is_none() || target == Some(workspace_id))
                            {
                                return Err(DomainError::Conflict(
                                    "Active or unscoped run prevents branch switching; use a separate worktree"
                                        .into(),
                                ));
                            }
                        }
                    }
                    git::switch_branch(path, branch, expected).await
                }
                _ => Err(DomainError::Validation("Unsupported Git operation".into())),
            }
        }
        .await;
        match result {
            Ok(value) => ApiResponse::success(req.id, value),
            Err(error) => ApiResponse::error(req.id, error.to_string()),
        }
    }
}
