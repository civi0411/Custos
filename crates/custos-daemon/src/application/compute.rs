//! compute application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_compute(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_COMPUTE_TARGETS_LIST => {
                let targets = serde_json::json!([
                    {
                        "backend": "colab",
                        "name": "Google Colab",
                        "description": "Hosted Colab runtime via google-colab-cli. Decoupled stop vs release prevents burning compute units.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    },
                    {
                        "backend": "kaggle",
                        "name": "Kaggle Kernels",
                        "description": "Batch Kaggle GPU kernels (30h/week free T4/P100) with pinned versions, log streaming & auto-teardown.",
                        "supports_interactive": false,
                        "decoupled_release": false
                    },
                    {
                        "backend": "runpod",
                        "name": "Rented GPU (RunPod)",
                        "description": "On-demand cloud GPU with nvidia-smi preflight, PID group tracking & automatic pod destruction.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    },
                    {
                        "backend": "vast_ai",
                        "name": "Rented GPU (Vast.ai)",
                        "description": "Vast.ai GPU instance with hardware verification and automated CLI instance teardown.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    },
                    {
                        "backend": "dedicated_ssh",
                        "name": "Dedicated SSH GPU",
                        "description": "Custom remote SSH GPU server with hardware probing, PID supervision & checkpoint triggers.",
                        "supports_interactive": true,
                        "decoupled_release": true
                    }
                ]);
                ApiResponse::success(req.id, targets)
            }
            METHOD_EXPERIMENT_PREFLIGHT => {
                let backend_str = req
                    .params
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .unwrap_or("colab");
                let recipe_id = req
                    .params
                    .get("recipe_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let work_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let backend_kind = match backend_str {
                    "colab" => ComputeBackendKind::Colab,
                    "kaggle" => ComputeBackendKind::Kaggle,
                    "runpod" => ComputeBackendKind::RunPod,
                    "vast_ai" => ComputeBackendKind::VastAi,
                    "dedicated_ssh" | "ssh" => ComputeBackendKind::DedicatedSsh,
                    _ => ComputeBackendKind::Local,
                };
                let force_mock = req
                    .params
                    .get("mock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let ssh_host = req.params.get("ssh_host").and_then(|v| v.as_str());
                let ssh_port = req
                    .params
                    .get("ssh_port")
                    .and_then(|v| v.as_u64())
                    .map(|p| p as u16);
                let ssh_user = req.params.get("ssh_user").and_then(|v| v.as_str());
                let pod_id = req.params.get("pod_id").and_then(|v| v.as_str());
                let cloud_api_key = req.params.get("cloud_api_key").and_then(|v| v.as_str());

                let adapter = self.resolve_target_adapter(
                    backend_kind,
                    &work_dir,
                    ssh_host,
                    ssh_port,
                    ssh_user,
                    pod_id,
                    cloud_api_key,
                    force_mock,
                );

                match adapter.preflight(recipe_id).await {
                    Ok(report) => match serde_json::to_value(&report) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_EXPERIMENT_SUBMIT => {
                let experiment_id = match req.params.get("experiment_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing experiment_id parameter"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                let backend_str = req
                    .params
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .unwrap_or("colab");
                let backend_kind = match backend_str {
                    "colab" => ComputeBackendKind::Colab,
                    "kaggle" => ComputeBackendKind::Kaggle,
                    "runpod" => ComputeBackendKind::RunPod,
                    "vast_ai" => ComputeBackendKind::VastAi,
                    "dedicated_ssh" | "ssh" => ComputeBackendKind::DedicatedSsh,
                    _ => ComputeBackendKind::Local,
                };
                let parent_checkpoint_id = req
                    .params
                    .get("parent_checkpoint_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let force_mock = req
                    .params
                    .get("mock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let ssh_host = req.params.get("ssh_host").and_then(|v| v.as_str());
                let ssh_port = req
                    .params
                    .get("ssh_port")
                    .and_then(|v| v.as_u64())
                    .map(|p| p as u16);
                let ssh_user = req.params.get("ssh_user").and_then(|v| v.as_str());
                let pod_id = req.params.get("pod_id").and_then(|v| v.as_str());
                let cloud_api_key = req.params.get("cloud_api_key").and_then(|v| v.as_str());

                let mut attempt = ExecutionAttempt::new(
                    experiment_id,
                    recipe_id,
                    backend_kind,
                    parent_checkpoint_id,
                );
                let work_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));

                let adapter = self.resolve_target_adapter(
                    backend_kind,
                    &work_dir,
                    ssh_host,
                    ssh_port,
                    ssh_user,
                    pod_id,
                    cloud_api_key,
                    force_mock,
                );

                let idempotency_key = format!("submit_{}", attempt.id);
                match adapter.submit(&attempt, &idempotency_key).await {
                    Ok(external_ref) => {
                        let _ = attempt.apply(ExecEvent::PreflightOk);
                        let _ = attempt.apply(ExecEvent::SubmitStarted);
                        let _ = attempt.apply(ExecEvent::SubmitAcked(external_ref.clone()));

                        if let Some(ref store) = self.sqlite_store {
                            let _ = store.experiment_attempts().save_attempt(&attempt);
                        }

                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "attempt": attempt,
                                "external_ref": external_ref,
                            }),
                        )
                    }
                    Err(e) => {
                        let _ = attempt.apply(ExecEvent::SubmitRejected);
                        if let Some(ref store) = self.sqlite_store {
                            let _ = store.experiment_attempts().save_attempt(&attempt);
                        }
                        ApiResponse::error(
                            req.id,
                            format!("Submission to {} failed: {}", backend_str, e),
                        )
                    }
                }
            }
            METHOD_EXPERIMENT_OBSERVE => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Attempt {attempt_id} not found"),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "Attempt does not have an external job reference",
                        )
                    }
                };

                let work_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req
                    .params
                    .get("mock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                match adapter.observe(&ext_ref).await {
                    Ok(obs) => {
                        attempt.exec_state = obs.exec_state;
                        attempt.resource_state = obs.resource_state;
                        let _ = store.experiment_attempts().save_attempt(&attempt);
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "attempt": attempt,
                                "observation": obs,
                            }),
                        )
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to observe {attempt_id}: {e}"))
                    }
                }
            }
            METHOD_EXPERIMENT_STOP => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Attempt {attempt_id} not found"),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => {
                        return ApiResponse::error(req.id, "Attempt has no external job reference")
                    }
                };

                let work_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req
                    .params
                    .get("mock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                if let Err(e) = adapter.stop_job(&ext_ref).await {
                    let _ = attempt.apply(ExecEvent::StopRequested);
                    let _ = store.experiment_attempts().save_attempt(&attempt);
                    return ApiResponse::error(req.id, format!("Failed to stop remote job: {e}"));
                }
                let _ = attempt.apply(ExecEvent::StopRequested);
                let _ = attempt.apply(ExecEvent::StopConfirmed);
                let _ = store.experiment_attempts().save_attempt(&attempt);

                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "stopped": true, "attempt": attempt }),
                )
            }
            METHOD_EXPERIMENT_RELEASE => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Attempt {attempt_id} not found"),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => {
                        return ApiResponse::error(req.id, "Attempt has no external job reference")
                    }
                };

                let work_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req
                    .params
                    .get("mock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                if let Err(e) = adapter.release_compute(&ext_ref).await {
                    let _ = attempt.apply_resource(ResourceEvent::ReleaseRequested);
                    let _ = store.experiment_attempts().save_attempt(&attempt);
                    return ApiResponse::error(req.id, format!("Failed to release compute: {e}"));
                }
                let _ = attempt.apply_resource(ResourceEvent::ReleaseRequested);
                let _ = attempt.apply_resource(ResourceEvent::ReleaseConfirmed);
                let _ = store.experiment_attempts().save_attempt(&attempt);

                ApiResponse::success(
                    req.id,
                    serde_json::json!({ "released": true, "attempt": attempt }),
                )
            }
            METHOD_EXPERIMENT_COLLECT => {
                let attempt_id = match req.params.get("attempt_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing attempt_id parameter"),
                };
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => return ApiResponse::error(req.id, "SqliteTaskStore not configured"),
                };
                let mut attempt = match store.experiment_attempts().get_attempt(attempt_id) {
                    Ok(Some(a)) => a,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Attempt {attempt_id} not found"),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let ext_ref = match attempt.external_ref.as_ref() {
                    Some(r) => r.clone(),
                    None => {
                        return ApiResponse::error(req.id, "Attempt has no external job reference")
                    }
                };

                let work_dir = dirs::home_dir()
                    .map(|h| h.join(".custos"))
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp/custos"));
                let force_mock = req
                    .params
                    .get("mock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let adapter = self.resolve_target_adapter(
                    attempt.backend,
                    &work_dir,
                    Some(&ext_ref.account_ref),
                    None,
                    None,
                    None,
                    None,
                    force_mock,
                );

                match adapter.collect_artifacts(&ext_ref).await {
                    Ok(artifacts) => {
                        let _ = attempt.apply(ExecEvent::Collected);
                        let _ = store.experiment_attempts().save_attempt(&attempt);
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({ "artifacts": artifacts, "attempt": attempt }),
                        )
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to collect artifacts: {e}"))
                    }
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
