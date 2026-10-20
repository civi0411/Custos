//! system application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_system(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.ping" | "v1.health" => {
                ApiResponse::success(req.id, serde_json::json!({ "status": "ok" }))
            }
            "v1.system.metrics" | "v1.system.telemetry" => {
                let mut process_groups: Vec<serde_json::Value> = Vec::new();
                let mut total_kb: f64 = 0.0;

                if let Some(out) = std::process::Command::new("ps")
                    .args(["-A", "-o", "pid=,rss=,%cpu=,comm="])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                {
                    for line in out.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let pid_str = parts[0];
                            let rss_str = parts[1];
                            let cpu_str = parts[2];
                            let comm = parts[3..].join(" ");
                            let lower_comm = comm.to_lowercase();

                            if lower_comm.contains("custos") {
                                if let (Ok(pid), Ok(rss), Ok(cpu)) = (
                                    pid_str.parse::<u32>(),
                                    rss_str.parse::<f64>(),
                                    cpu_str.parse::<f64>(),
                                ) {
                                    total_kb += rss;
                                    let mb = ((rss / 1024.0) * 10.0).round() / 10.0;
                                    let (id, name, kind) = if lower_comm.contains("custos-daemon") {
                                        ("daemon", "Custos Daemon (Core Engine)", "daemon")
                                    } else if lower_comm.contains("custos-app-desktop") {
                                        ("desktop", "Custos Desktop (GUI Studio)", "app")
                                    } else {
                                        ("runner", "Custos Worker Process", "worker")
                                    };

                                    process_groups.push(serde_json::json!({
                                        "id": id,
                                        "name": name,
                                        "kind": kind,
                                        "pid": pid,
                                        "memory_mb": mb,
                                        "cpu_percent": cpu,
                                    }));
                                }
                            }
                        }
                    }
                }

                let memory_resident_mb = if total_kb == 0.0 {
                    let pid = std::process::id();
                    let rss_kb: f64 = std::process::Command::new("ps")
                        .args(["-o", "rss=", "-p", &pid.to_string()])
                        .output()
                        .ok()
                        .and_then(|out| String::from_utf8(out.stdout).ok())
                        .and_then(|s| s.trim().parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let mb = ((rss_kb / 1024.0) * 10.0).round() / 10.0;
                    process_groups.push(serde_json::json!({
                        "id": "daemon",
                        "name": "Custos Daemon (Core Engine)",
                        "kind": "daemon",
                        "pid": pid,
                        "memory_mb": mb,
                        "cpu_percent": 0.0,
                    }));
                    mb
                } else {
                    ((total_kb / 1024.0) * 10.0).round() / 10.0
                };

                process_groups.sort_by(|a, b| {
                    let mb_b = b["memory_mb"].as_f64().unwrap_or(0.0);
                    let mb_a = a["memory_mb"].as_f64().unwrap_or(0.0);
                    mb_b.partial_cmp(&mb_a).unwrap_or(std::cmp::Ordering::Equal)
                });

                let system_total_memory_mb: Option<u64> = {
                    #[cfg(target_os = "macos")]
                    {
                        std::process::Command::new("sysctl")
                            .args(["-n", "hw.memsize"])
                            .output()
                            .ok()
                            .and_then(|out| String::from_utf8(out.stdout).ok())
                            .and_then(|s| s.trim().parse::<u64>().ok())
                            .map(|bytes| bytes / (1024 * 1024))
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        None
                    }
                };

                let mut active_runs: u64 = 0;
                if let Some(store) = &self.usage_runs {
                    if let Ok(tasks) = self.task_service.list_tasks().await {
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

                let active_ports: Vec<u16> = std::process::Command::new("lsof")
                    .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| {
                        let mut ports = std::collections::BTreeSet::new();
                        for line in s.lines().skip(1) {
                            if let Some(part) = line.split_whitespace().nth(8) {
                                if let Some(port_str) = part.rsplit(':').next() {
                                    if let Ok(p) = port_str.parse::<u16>() {
                                        if (1024..=49151).contains(&p) {
                                            ports.insert(p);
                                        }
                                    }
                                }
                            }
                        }
                        ports.into_iter().collect()
                    })
                    .unwrap_or_default();

                let caffeinate_status = {
                    let mut lock = CAFFEINATE_STATE.lock().unwrap();
                    let (mode, child) = &mut *lock;
                    if mode.is_empty() {
                        *mode = "off".to_string();
                    }
                    let is_active = child
                        .as_mut()
                        .map(|c| c.try_wait().ok().flatten().is_none())
                        .unwrap_or(false);
                    serde_json::json!({
                        "mode": mode,
                        "active": is_active
                    })
                };

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "memory_resident_mb": memory_resident_mb,
                        "system_total_memory_mb": system_total_memory_mb,
                        "process_groups": process_groups,
                        "active_runs": active_runs,
                        "active_ports": active_ports,
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
                let mut lock = CAFFEINATE_STATE.lock().unwrap();
                let (current_mode, child) = &mut *lock;
                *current_mode = mode.to_string();

                let should_run = mode == "always";
                if should_run {
                    if child.is_none() {
                        *child = std::process::Command::new("caffeinate")
                            .args(["-i", "-s"])
                            .spawn()
                            .ok();
                    }
                } else if let Some(mut c) = child.take() {
                    let _ = c.kill();
                }

                let is_active = child
                    .as_mut()
                    .map(|c| c.try_wait().ok().flatten().is_none())
                    .unwrap_or(false);
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "mode": mode,
                        "active": is_active
                    }),
                )
            }
            METHOD_SETTINGS_GET => {
                let store = match self.sqlite_store.as_ref() {
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
                let store = match self.sqlite_store.as_ref() {
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
                        label: operation.replace('.', " ").replace('_', " "),
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
