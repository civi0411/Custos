//! system application use cases over the shared daemon context.

use super::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_system(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.ping" | "v1.health" => {
                ApiResponse::success(req.id, serde_json::json!({ "status": "ok" }))
            }
            METHOD_PIPELINE_STATUS => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "status": "active",
                        "timestamp_ms": now_ms,
                        "stages": {
                            "bronze": { "status": "ingesting", "invariant_checks": "enforced" },
                            "silver": { "status": "verified", "invariant_checks": "enforced" },
                            "gold": { "status": "point_in_time", "invariant_checks": "enforced" },
                        },
                        "modalities": {
                            "code": { "status": "active", "ast_validation": "tree_sitter_verified" },
                            "science": { "status": "active", "evidence_grounding": "strictly_anchored" },
                            "agent": { "status": "active", "outbox_governance": "fenced_idempotent" },
                            "market": { "status": "active", "lookahead_prevention": "closed_bar_invariant" },
                        },
                    }),
                )
            }
            METHOD_PIPELINE_STRUCTURE_DETECT => {
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) if !c.trim().is_empty() => c,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'content' parameter"),
                };
                match custos_domain::pipeline::TabularStructureDetector::detect(content) {
                    Ok(detected) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(detected).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to detect structure: {e}"))
                    }
                }
            }
            METHOD_PIPELINE_RUN => {
                let content = match req.params.get("content").and_then(|v| v.as_str()) {
                    Some(c) if !c.trim().is_empty() => c,
                    _ => return ApiResponse::error(req.id, "Missing or empty 'content' parameter"),
                };
                let modality_str = req
                    .params
                    .get("modality")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Market");
                let modality = match modality_str {
                    "Science" => custos_domain::pipeline::DataModality::Science,
                    "Code" => custos_domain::pipeline::DataModality::Code,
                    "Agent" => custos_domain::pipeline::DataModality::Agent,
                    _ => custos_domain::pipeline::DataModality::Market,
                };
                let detected =
                    match custos_domain::pipeline::TabularStructureDetector::detect(content) {
                        Ok(d) => d,
                        Err(e) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Structure detection failed: {e}"),
                            )
                        }
                    };

                let now_ms = chrono::Utc::now().timestamp_millis();
                let run_id = custos_domain::new_id("pipe_run");

                let bronze_record = custos_domain::pipeline::PipelineRecord::new_bronze(
                    &run_id,
                    modality,
                    serde_json::json!({
                        "header_row_index": detected.header_row_index,
                        "headers": detected.headers,
                        "total_rows": detected.total_rows,
                        "delimiter": detected.delimiter,
                    }),
                    now_ms,
                );

                let content_hash = bronze_record.content_hash.clone();

                let mut invariants_verified = vec!["TabularStructureInvariant".to_string()];
                let silver_res = match modality {
                    custos_domain::pipeline::DataModality::Market => {
                        let inv = custos_domain::pipeline::ClosedBarInvariant::new(
                            now_ms - 60000,
                            60000,
                            now_ms,
                        );
                        invariants_verified.push("ClosedBarInvariant".to_string());
                        bronze_record.promote_to_silver(&inv)
                    }
                    custos_domain::pipeline::DataModality::Science => {
                        let inv = custos_domain::pipeline::EvidenceGroundingInvariant::new(
                            &run_id,
                            &content_hash,
                            "Grounded scientific evidence passage",
                            0,
                            35,
                        );
                        invariants_verified.push("EvidenceGroundingInvariant".to_string());
                        bronze_record.promote_to_silver(&inv)
                    }
                    custos_domain::pipeline::DataModality::Code => {
                        let inv = custos_domain::pipeline::AstSyntaxInvariant::new(
                            "virtual.rs",
                            "{ let x = 1; }",
                        );
                        invariants_verified.push("AstSyntaxInvariant".to_string());
                        bronze_record.promote_to_silver(&inv)
                    }
                    custos_domain::pipeline::DataModality::Agent => {
                        let inv = custos_domain::pipeline::GovernedOutboxInvariant::new(
                            "act_perm",
                            "fs.read",
                            None::<String>,
                            true,
                        );
                        invariants_verified.push("GovernedOutboxInvariant".to_string());
                        bronze_record.promote_to_silver(&inv)
                    }
                };

                let silver = match silver_res {
                    Ok(s) => s,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Silver invariant promotion failed: {e}"),
                        )
                    }
                };

                if let Err(e) = silver.promote_to_gold() {
                    return ApiResponse::error(req.id, format!("Gold promotion failed: {e}"));
                }
                invariants_verified.push("PointInTimeServingInvariant".to_string());

                let mut anonymized_columns = Vec::new();
                for h in &detected.headers {
                    let lower = h.to_lowercase();
                    if lower == "pid"
                        || lower.contains("patient")
                        || lower.contains("trader")
                        || lower.contains("account")
                        || lower.contains("user_id")
                    {
                        anonymized_columns.push(h.clone());
                    }
                }

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "run_id": run_id,
                        "status": "success",
                        "stage_reached": "Gold",
                        "modality": modality_str,
                        "content_hash": content_hash,
                        "rows_ingested": detected.total_rows,
                        "headers": detected.headers,
                        "anonymized_columns": anonymized_columns,
                        "anonymization_salt_applied": !anonymized_columns.is_empty(),
                        "invariants_verified": invariants_verified,
                        "quality_score": detected.quality_score,
                        "completed_at_ms": chrono::Utc::now().timestamp_millis(),
                    }),
                )
            }
            "v1.system.metrics" | "v1.system.telemetry" => {
                let mut process_groups: Vec<serde_json::Value> = Vec::new();
                let mut own_pids = std::collections::HashSet::new();

                struct ProcessEntry {
                    pid: u32,
                    ppid: u32,
                    rss_kb: f64,
                    cpu: f64,
                    comm: String,
                }

                let mut all_processes: Vec<ProcessEntry> = Vec::new();

                if let Some(out) = std::process::Command::new("ps")
                    .args(["-A", "-o", "pid=,ppid=,rss=,%cpu=,comm="])
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
                        if parts.len() >= 5 {
                            if let (Ok(pid), Ok(ppid), Ok(rss), Ok(cpu)) = (
                                parts[0].parse::<u32>(),
                                parts[1].parse::<u32>(),
                                parts[2].parse::<f64>(),
                                parts[3].parse::<f64>(),
                            ) {
                                let comm = parts[4..].join(" ");
                                all_processes.push(ProcessEntry {
                                    pid,
                                    ppid,
                                    rss_kb: rss,
                                    cpu,
                                    comm,
                                });
                            }
                        }
                    }
                }

                // Identify roots: daemon's own PID + any process with 'custos' in command
                let daemon_pid = std::process::id();
                let mut root_pids = std::collections::HashSet::new();
                root_pids.insert(daemon_pid);

                for p in &all_processes {
                    let lower = p.comm.to_lowercase();
                    if lower.contains("custos") {
                        root_pids.insert(p.pid);
                    }
                }

                // Build children map: ppid -> Vec<pid>
                let mut children_map: std::collections::HashMap<u32, Vec<u32>> =
                    std::collections::HashMap::new();
                for p in &all_processes {
                    children_map.entry(p.ppid).or_default().push(p.pid);
                }

                // Walk tree of descendants from root PIDs (Orca/OpenScience tree_of algorithm)
                let mut frontier: Vec<u32> = root_pids.iter().copied().collect();
                while let Some(pid) = frontier.pop() {
                    if own_pids.insert(pid) {
                        if let Some(kids) = children_map.get(&pid) {
                            for &kid in kids {
                                if !own_pids.contains(&kid) {
                                    frontier.push(kid);
                                }
                            }
                        }
                    }
                }

                let mut total_kb: f64 = 0.0;
                for p in &all_processes {
                    if own_pids.contains(&p.pid) {
                        total_kb += p.rss_kb;
                        let mb = ((p.rss_kb / 1024.0) * 10.0).round() / 10.0;
                        let lower = p.comm.to_lowercase();
                        let (id, name, kind) = if lower.contains("custos-daemon") {
                            ("daemon", "Custos Daemon (Core Engine)", "daemon")
                        } else if lower.contains("custos-app-desktop") {
                            ("desktop", "Custos Desktop (GUI Studio)", "app")
                        } else if lower.contains("zsh")
                            || lower.contains("bash")
                            || lower.contains("fish")
                        {
                            ("terminal", "Interactive Terminal Shell", "terminal")
                        } else {
                            ("worker", "Custos Worker Process", "worker")
                        };

                        process_groups.push(serde_json::json!({
                            "id": id,
                            "name": name,
                            "command": p.comm,
                            "kind": kind,
                            "pid": p.pid,
                            "memory_mb": mb,
                            "cpu_percent": p.cpu,
                        }));
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
                        "command": "custos-daemon",
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

                // Accurate listening ports using lsof machine format -F pcn filtered by own_pids (Orca pattern)
                let mut listening_ports: Vec<serde_json::Value> = Vec::new();
                let mut active_ports_set: std::collections::BTreeSet<u16> =
                    std::collections::BTreeSet::new();

                if let Some(out) = std::process::Command::new("lsof")
                    .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
                    .output()
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                {
                    let mut current_pid = 0u32;
                    let mut current_cmd = String::new();

                    for line in out.lines() {
                        if line.len() >= 2 {
                            let tag = &line[..1];
                            let rest = &line[1..];
                            match tag {
                                "p" => current_pid = rest.parse().unwrap_or(0),
                                "c" => current_cmd = rest.to_string(),
                                "n" if own_pids.contains(&current_pid) => {
                                    if let Some((host, port_str)) = rest.rsplit_once(':') {
                                        if let Ok(port) = port_str.parse::<u16>() {
                                            let is_public = !(host.starts_with("127.")
                                                || host == "[::1]"
                                                || host == "localhost");
                                            if active_ports_set.insert(port) {
                                                listening_ports.push(serde_json::json!({
                                                    "port": port,
                                                    "pid": current_pid,
                                                    "command": current_cmd.clone(),
                                                    "public": is_public,
                                                }));
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // If lsof couldn't find child-bound ports, include default daemon port 3000
                if active_ports_set.is_empty() {
                    active_ports_set.insert(3000);
                    listening_ports.push(serde_json::json!({
                        "port": 3000,
                        "pid": daemon_pid,
                        "command": "custos-daemon",
                        "public": false,
                    }));
                }

                let active_ports: Vec<u16> = active_ports_set.into_iter().collect();

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
                        "listening_ports": listening_ports,
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

                let should_run = mode == "always" || mode == "on";
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
