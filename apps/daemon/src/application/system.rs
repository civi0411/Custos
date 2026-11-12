//! system application use cases over the shared daemon context.

use super::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_system(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.ping" | "v1.health" => {
                ApiResponse::success(req.id, serde_json::json!({ "status": "ok" }))
            }
            "v1.system.metrics" | "v1.system.telemetry" => {
                let telemetry = custos_adapters::host::telemetry::collect_system_telemetry();

                let mut active_runs: u64 = 0;
                if let Some(store) = &self.kernel.usage_runs {
                    if let Ok(tasks) = self.kernel.task_service.list_tasks().await {
                        for task in tasks {
                            if let Ok(runs) = store.list_runs_for_task(&task.id).await {
                                for run in runs {
                                    if !run.status.is_terminal() {
                                        active_runs += 1;
                                    }
                                }
                            }
                        }
                    }
                }

                let (caffeinate_mode, is_active) =
                    custos_adapters::host::CAFFEINATE_MANAGER.status();
                let caffeinate_status = serde_json::json!({
                    "mode": caffeinate_mode,
                    "active": is_active,
                });

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "memory_resident_mb": telemetry.memory_resident_mb,
                        "system_total_memory_mb": telemetry.system_total_memory_mb,
                        "process_groups": telemetry.process_groups,
                        "active_runs": active_runs,
                        "active_ports": telemetry.active_ports,
                        "listening_ports": telemetry.listening_ports,
                        "caffeinate": caffeinate_status
                    }),
                )
            }
            "v1.system.caffeinate" => {
                let mode = req
                    .params
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("off");
                let (current_mode, is_active) =
                    custos_adapters::host::CAFFEINATE_MANAGER.set_mode(mode);
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "mode": current_mode,
                        "active": is_active
                    }),
                )
            }
            METHOD_SETTINGS_GET => {
                let store = match self.kernel.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                match store.get_settings() {
                    Ok(settings) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&settings).unwrap_or_default(),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SETTINGS_SET => {
                let store = match self.kernel.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                if let Some(map) = req.params.as_object() {
                    let mut updates = std::collections::HashMap::with_capacity(map.len());
                    for (k, v) in map {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        updates.insert(k.clone(), val_str);
                    }
                    if let Err(e) = store.set_settings(&updates) {
                        return ApiResponse::error(req.id, e.to_string());
                    }
                    match store.get_settings() {
                        Ok(settings) => ApiResponse::success(
                            req.id,
                            serde_json::to_value(&settings).unwrap_or_default(),
                        ),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    ApiResponse::error(
                        req.id,
                        "Invalid params: expected JSON object of key-value settings",
                    )
                }
            }
            METHOD_CAPABILITIES_LIST => {
                let caps = self.list_capabilities();
                match serde_json::to_value(&caps) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_CAPABILITIES_GET => {
                let target_id = req
                    .params
                    .get("capability_id")
                    .or_else(|| req.params.get("id"))
                    .or_else(|| req.params.get("resource_id"))
                    .and_then(|v| v.as_str());

                match target_id {
                    Some(id) => {
                        let caps = self.list_capabilities();
                        if let Some(c) = caps
                            .into_iter()
                            .find(|cap| cap.id == id || cap.resource_id.as_deref() == Some(id))
                        {
                            match serde_json::to_value(&c) {
                                Ok(val) => ApiResponse::success(req.id, val),
                                Err(e) => ApiResponse::error(req.id, e.to_string()),
                            }
                        } else {
                            ApiResponse::error(req.id, format!("Capability '{id}' not found"))
                        }
                    }
                    None => ApiResponse::error(req.id, "Missing capability_id parameter"),
                }
            }
            METHOD_RESOURCES_CAPABILITIES => {
                let resource_id = match req.params.get("resource_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing resource_id parameter"),
                };
                let descriptor = match self
                    .list_capabilities()
                    .into_iter()
                    .find(|capability| capability.resource_id.as_deref() == Some(resource_id))
                {
                    Some(capability) => capability,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            format!("Unknown resource_id: {resource_id}"),
                        )
                    }
                };
                let actions: Vec<ResourceActionCapability> = descriptor
                    .supported_operations
                    .iter()
                    .enumerate()
                    .map(|(index, operation)| ResourceActionCapability {
                        id: operation.clone(),
                        label: operation.replace(['.', '_'], " "),
                        status: "available".into(),
                        reason: None,
                        primary: index == 0,
                        shortcut: None,
                    })
                    .collect();
                match serde_json::to_value(&actions) {
                    Ok(value) => ApiResponse::success(req.id, value),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
