//! Fleet host registration, ping, and remote execution handlers.

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_integrations_fleet(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_FLEET_HOSTS_LIST => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                match fleet.list_remote_hosts() {
                    Ok(hosts) => match serde_json::to_value(&hosts) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_REGISTER => {
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let params: RegisterHostParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid register host params: {e}"),
                        )
                    }
                };
                let auth_method = if let Some(key_path) = &params.private_key_path {
                    SshAuthMethod::KeyPair {
                        private_key_path: key_path.clone(),
                        passphrase: None,
                    }
                } else {
                    SshAuthMethod::Agent
                };
                let mut node = match RemoteHostNode::new(
                    params.name,
                    params.host,
                    params.port,
                    params.user,
                    auth_method,
                ) {
                    Ok(n) => n,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                node.labels = params.labels;
                match fleet.save_remote_host(&node) {
                    Ok(()) => match serde_json::to_value(&node) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_FLEET_HOSTS_PING => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; SSH probe is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let mut host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => {
                        return ApiResponse::error(req.id, format!("Host '{host_id}' not found"))
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin =
                    resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("ConnectTimeout=3")
                    .arg("-o")
                    .arg("StrictHostKeyChecking=yes")
                    .arg("-p")
                    .arg(host.port.to_string());

                if let SshAuthMethod::KeyPair {
                    private_key_path, ..
                } = &host.auth_method
                {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg("echo -n ping");

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        if out.status.success() {
                            host.status = RemoteHostStatus::Online;
                            host.last_ping_ms = Some(duration_ms);
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "host_id": host.id,
                                    "status": "online",
                                    "ping_ms": duration_ms
                                }),
                            )
                        } else {
                            let stderr = String::from_utf8_lossy(&out.stderr);
                            host.status = RemoteHostStatus::Offline;
                            host.updated_at = now;
                            let _ = fleet.save_remote_host(&host);
                            ApiResponse::success(
                                req.id,
                                serde_json::json!({
                                    "host_id": host.id,
                                    "status": "offline",
                                    "error": stderr.trim()
                                }),
                            )
                        }
                    }
                    Err(e) => {
                        host.status = RemoteHostStatus::Offline;
                        host.updated_at = now;
                        let _ = fleet.save_remote_host(&host);
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "host_id": host.id,
                                "status": "offline",
                                "error": e.to_string()
                            }),
                        )
                    }
                }
            }
            METHOD_FLEET_EXEC => {
                if !remote_execution_adapter_connected() {
                    return ApiResponse::error(
                        req.id,
                        "Governed remote execution adapter is not connected; remote command is fail-closed",
                    );
                }
                let fleet = match self.fleet_automation.as_ref() {
                    Some(f) => f,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "FleetAutomationRepository not configured",
                        )
                    }
                };
                let host_id = match req.params.get("host_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing host_id parameter"),
                };
                let command = match req.params.get("command").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return ApiResponse::error(req.id, "Missing command parameter"),
                };
                let host = match fleet.get_remote_host(host_id) {
                    Ok(Some(h)) => h,
                    Ok(None) => {
                        return ApiResponse::error(req.id, format!("Host '{host_id}' not found"))
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let now = chrono::Utc::now().timestamp_millis();
                let start = std::time::Instant::now();

                let ssh_bin =
                    resolve_ssh_binary().unwrap_or_else(|| std::path::PathBuf::from("ssh"));
                let mut cmd = tokio::process::Command::new(ssh_bin);
                cmd.arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("ConnectTimeout=10")
                    .arg("-o")
                    .arg("StrictHostKeyChecking=yes")
                    .arg("-p")
                    .arg(host.port.to_string());

                if let SshAuthMethod::KeyPair {
                    private_key_path, ..
                } = &host.auth_method
                {
                    cmd.arg("-i").arg(private_key_path);
                }

                let target = format!("{}@{}", host.user, host.host);
                cmd.arg(&target).arg(command);

                match cmd.output().await {
                    Ok(out) => {
                        let duration_ms = start.elapsed().as_millis() as u64;
                        let receipt = FleetExecReceipt {
                            execution_id: new_id("fexec"),
                            host_id: host.id.clone(),
                            command: command.to_string(),
                            exit_code: out.status.code(),
                            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                            duration_ms,
                            // Process exit is an operational observation, not
                            // independent verification of the remote effect.
                            verified: false,
                            timestamp: now,
                        };
                        let _ = fleet.save_fleet_execution(&receipt);
                        match serde_json::to_value(&receipt) {
                            Ok(val) => ApiResponse::success(req.id, val),
                            Err(e) => ApiResponse::error(req.id, e.to_string()),
                        }
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to spawn SSH process: {e}"))
                    }
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
