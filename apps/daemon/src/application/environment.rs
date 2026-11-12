//! Environment application use cases (AEE lifecycle & verification).

use super::*;
use custos_core::contracts::EnvironmentPort;
use custos_domain::environment::{EnvironmentProfile, EnvironmentSpec};

impl LocalApiDispatcher {
    pub(crate) async fn handle_environment(&self, req: ApiRequest) -> ApiResponse {
        let env_coord = match &self.env.environment_coordinator {
            Some(coord) => coord.clone(),
            None => return ApiResponse::error(req.id, "Environment coordinator not initialized"),
        };

        match req.method.as_str() {
            "v1.environment.list" => {
                let handles = env_coord.list_handles().await;
                ApiResponse::success(req.id, serde_json::to_value(handles).unwrap_or_default())
            }
            "v1.environment.get" => {
                let env_id = match req.params.get("env_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'env_id' parameter"),
                };
                match env_coord.get_handle(env_id).await {
                    Some(handle) => {
                        ApiResponse::success(req.id, serde_json::to_value(handle).unwrap_or_default())
                    }
                    None => ApiResponse::error(req.id, format!("Environment '{env_id}' not found")),
                }
            }
            "v1.environment.provision" => {
                let spec: EnvironmentSpec = match serde_json::from_value(req.params) {
                    Ok(s) => s,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid EnvironmentSpec: {e}"))
                    }
                };
                match env_coord.provision(&spec).await {
                    Ok(handle) => {
                        ApiResponse::success(req.id, serde_json::to_value(handle).unwrap_or_default())
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to provision environment: {e}"))
                    }
                }
            }
            "v1.environment.verify" => {
                let env_id = match req.params.get("env_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'env_id' parameter"),
                };
                let handle = match env_coord.get_handle(env_id).await {
                    Some(h) => h,
                    None => return ApiResponse::error(req.id, format!("Environment '{env_id}' not found")),
                };
                let spec: EnvironmentSpec = match req.params.get("spec") {
                    Some(s) => match serde_json::from_value(s.clone()) {
                        Ok(spec) => spec,
                        Err(e) => return ApiResponse::error(req.id, format!("Invalid spec: {e}")),
                    },
                    None => EnvironmentSpec::new(
                        env_id,
                        EnvironmentProfile::Assistant {
                            project_id: None,
                        },
                    ),
                };
                match env_coord.verify(&handle, &spec).await {
                    Ok(report) => {
                        ApiResponse::success(req.id, serde_json::to_value(report).unwrap_or_default())
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Verification failed: {e}")),
                }
            }
            "v1.environment.teardown" => {
                let env_id = match req.params.get("env_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'env_id' parameter"),
                };
                let handle = match env_coord.get_handle(env_id).await {
                    Some(h) => h,
                    None => return ApiResponse::error(req.id, format!("Environment '{env_id}' not found")),
                };
                match env_coord.teardown(&handle).await {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "status": "torn_down", "env_id": env_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("Teardown failed: {e}")),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown environment method: {unknown}")),
        }
    }
}

