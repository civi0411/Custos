use serde_json::json;
use std::path::Path;

use custos_core::contracts::compute::PreflightReport;
use custos_domain::{ComputeBackendKind, DomainError};

use super::RentedGpuTargetAdapter;

pub(super) async fn run_gpu_preflight(
    adapter: &RentedGpuTargetAdapter,
) -> Result<PreflightReport, DomainError> {
    if !adapter.is_safe_host() || !adapter.is_safe_user() {
        return Ok(PreflightReport {
            allowed: false,
            blockers: vec!["Invalid characters detected in remote SSH host or username".into()],
            warnings: Vec::new(),
        });
    }

    let mut cmd = adapter.build_ssh_command(
        "nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader",
    );

    let output = match cmd.output().await {
        Ok(o) => o,
        Err(e) => {
            return Ok(PreflightReport {
                allowed: false,
                blockers: vec![format!("Failed to execute SSH to {}: {e}", adapter.host)],
                warnings: Vec::new(),
            });
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(PreflightReport {
            allowed: false,
            blockers: vec![format!(
                "GPU check failed on {}:{}. nvidia-smi error: {stderr}",
                adapter.host, adapter.port
            )],
            warnings: Vec::new(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let warnings = if stdout.is_empty() {
        vec!["No GPU devices detected via nvidia-smi".to_string()]
    } else {
        vec![format!("Detected GPU: {stdout}")]
    };

    Ok(PreflightReport {
        allowed: true,
        blockers: Vec::new(),
        warnings,
    })
}

pub(super) async fn deallocate_cloud_pod(
    backend_kind: ComputeBackendKind,
    pod: &str,
    api_key: &str,
) -> Result<(), DomainError> {
    match backend_kind {
        ComputeBackendKind::RunPod => {
            let client = reqwest::Client::new();
            let query = json!({
                "query": format!("mutation {{ podStop(input: {{ podId: \"{pod}\" }}) {{ id }} }}")
            });
            let resp = client
                .post("https://api.runpod.io/graphql")
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&query)
                .send()
                .await
                .map_err(|e| {
                    DomainError::Validation(format!("Failed to deallocate RunPod instance: {e}"))
                })?;
            if !resp.status().is_success() {
                return Err(DomainError::Validation(format!(
                    "RunPod deallocation returned HTTP status {}",
                    resp.status()
                )));
            }
        }
        ComputeBackendKind::VastAi => {
            let mut vast_cmd = tokio::process::Command::new("vastai");
            vast_cmd.arg("destroy").arg("instance").arg(pod);
            vast_cmd.env("VAST_API_KEY", api_key);
            let out = vast_cmd.output().await.map_err(|e| {
                DomainError::Validation(format!("Failed to invoke vastai destroy: {e}"))
            })?;
            if !out.status.success() {
                return Err(DomainError::Validation(format!(
                    "vastai destroy failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) async fn fetch_remote_train_log(
    adapter: &RentedGpuTargetAdapter,
    job_id: &str,
    dest: &Path,
) {
    let remote_src = format!(
        "{}@{}:/tmp/custos/runs/{}/train.log",
        adapter.user, adapter.host, job_id
    );
    let mut scp_cmd = tokio::process::Command::new("scp");
    scp_cmd.arg("-P").arg(adapter.port.to_string());
    scp_cmd.arg("-o").arg("StrictHostKeyChecking=no");
    if let Some(ref key) = adapter.ssh_key_path {
        scp_cmd.arg("-i").arg(key);
    }
    scp_cmd.arg(&remote_src).arg(dest);
    let _ = scp_cmd.output().await;
}

