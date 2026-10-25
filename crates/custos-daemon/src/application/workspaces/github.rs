//! GitHub accounts, login, auth status, and viewer use cases.

use crate::api::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_workspaces_github(
        &self,
        req: ApiRequest,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_GITHUB_ACCOUNTS => {
                let gh = match resolve_gh_binary() {
                    Some(gh) => gh,
                    None => {
                        return ApiResponse::success(
                            req.id,
                            serde_json::json!({"installed":false,"accounts":[]}),
                        )
                    }
                };
                match custos_adapters::repository_git::github_accounts(&gh).await {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GITHUB_LOGIN => {
                let _login_guard = GITHUB_LOGIN_LOCK.lock().await;
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => return ApiResponse::error(
                        req.id,
                        "Install GitHub CLI (gh) to connect. Local Git/worktrees remain available.",
                    ),
                };
                // Reuse the bounded PTY lifecycle, never the model OAuth singleton.
                let existing = self.terminal.list_sessions(Some("github-auth")).await;
                if let Ok(sessions) = existing {
                    if let Some(session) = sessions.into_iter().find(|session| {
                        matches!(session.status, custos_domain::TerminalSessionStatus::Active)
                    }) {
                        return match serde_json::to_value(session) {
                            Ok(value) => ApiResponse::success(req.id, value),
                            Err(error) => ApiResponse::error(req.id, error.to_string()),
                        };
                    }
                }
                let command = github_login_command(&gh);
                tracing::info!(request_id = %req.id, "Starting user-requested GitHub CLI login");
                match self
                    .terminal
                    .spawn(
                        "github-auth",
                        &std::env::temp_dir(),
                        Some(&command),
                        100,
                        28,
                    )
                    .await
                {
                    Ok(session) => match serde_json::to_value(session) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            METHOD_GITHUB_AUTH_STATUS => {
                let gh = match resolve_gh_binary() {
                    Some(path) => path,
                    None => {
                        return ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "authenticated": false, "source": "gh_cli", "installed": false,
                                "error": "GitHub CLI (gh) is not installed. Local Git/worktrees remain available."
                            }),
                        )
                    }
                };
                // The authenticated viewer is authoritative. Never export gh's token or
                // trust a cached profile / a fragment of multi-account auth-status text.
                let mut command = tokio::process::Command::new(gh);
                command.args(["api", "--hostname", "github.com", "user"]);
                command.kill_on_drop(true);
                let result =
                    tokio::time::timeout(std::time::Duration::from_secs(10), command.output())
                        .await;
                let value = match result {
                    Ok(Ok(output)) => github_auth_metadata(output.status.success(), &output.stdout),
                    Ok(Err(_)) => {
                        serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "Could not launch GitHub CLI."})
                    }
                    Err(_) => {
                        serde_json::json!({"authenticated": false, "source": "gh_cli", "installed": true, "error": "GitHub verification timed out. Check the network and retry."})
                    }
                };
                ApiResponse::success(req.id, value)
            }
            METHOD_GITHUB_VIEWER => {
                let gh_bin = match resolve_gh_binary() {
                    Some(b) => b,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "GitHub CLI (`gh`) is not installed or not in PATH",
                        );
                    }
                };

                match tokio::process::Command::new(&gh_bin)
                    .args(["api", "user"])
                    .output()
                    .await
                {
                    Ok(out) if out.status.success() => {
                        match serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(
                                req.id,
                                format!("Invalid JSON from gh api user: {e}"),
                            ),
                        }
                    }
                    Ok(out) => {
                        let err_msg = String::from_utf8_lossy(&out.stderr);
                        ApiResponse::error(
                            req.id,
                            format!("gh api user failed: {}", err_msg.trim()),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to run gh api user: {e}")),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown github method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_github_auth_metadata_offline() {
        let meta = github_auth_metadata(false, b"");
        assert_eq!(
            meta.get("authenticated").and_then(|v| v.as_bool()),
            Some(false)
        );
    }
}
