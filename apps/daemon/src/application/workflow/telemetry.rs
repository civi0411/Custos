//! Workflow execution telemetry, waterfall inspection, logs, and watchdog handlers.

use super::super::*;
use custos_core::contracts::workflow::{
    SupervisionFindingFilter, SupervisionIngestPort, SupervisionLogFilter, SupervisionQueryPort,
    SupervisionRunFilter,
};
use custos_domain::{LogSeverity, SupervisionSeverity};

impl LocalApiDispatcher {
    pub(crate) async fn handle_workflow_telemetry(&self, req: ApiRequest, submethod: &str) -> ApiResponse {
        match submethod {
            "overview" | "metrics" | "runs.metrics" => match self.kernel.supervision.overview().await {
                Ok(data) => ApiResponse::success(req.id, serde_json::to_value(data).unwrap_or_default()),
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "runs" | "runs.list" => {
                let filter = SupervisionRunFilter {
                    workbench_id: req
                        .params
                        .get("workbench_id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    status: req
                        .params
                        .get("status")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    limit: req
                        .params
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                };
                match self.kernel.supervision.list_runs(filter).await {
                    Ok(runs) => ApiResponse::success(req.id, serde_json::json!({ "runs": runs })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "waterfall" | "inspect" | "runs.inspect" => {
                let Some(run_id) = req
                    .params
                    .get("run_id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                else {
                    return ApiResponse::error(req.id, "Missing required 'run_id' parameter");
                };
                match self.kernel.supervision.get_run_waterfall(run_id).await {
                    Ok(Some(view)) => {
                        ApiResponse::success(req.id, serde_json::to_value(view).unwrap_or_default())
                    }
                    Ok(None) => ApiResponse::error(req.id, format!("Workflow run '{run_id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "logs" | "runs.logs" => {
                let filter = SupervisionLogFilter {
                    run_id: req
                        .params
                        .get("run_id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    level: req
                        .params
                        .get("level")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(LogSeverity::parse),
                    span_id: req
                        .params
                        .get("span_id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    limit: req
                        .params
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                };
                match self.kernel.supervision.list_logs(filter).await {
                    Ok(logs) => ApiResponse::success(req.id, serde_json::json!({ "logs": logs })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "findings" | "runs.findings" => {
                let filter = SupervisionFindingFilter {
                    run_id: req
                        .params
                        .get("run_id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    workbench_id: req
                        .params
                        .get("workbench_id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                    severity: req
                        .params
                        .get("severity")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(SupervisionSeverity::parse),
                    unresolved_only: req
                        .params
                        .get("unresolved_only")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    limit: req
                        .params
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                };
                match self.kernel.supervision.list_findings(filter).await {
                    Ok(findings) => {
                        ApiResponse::success(req.id, serde_json::json!({ "findings": findings }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "errors" | "runs.errors" => {
                let workbench_id = req
                    .params
                    .get("workbench_id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty());
                let limit = req
                    .params
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(500);
                match self
                    .kernel
                    .supervision
                    .list_error_groups(workbench_id, limit)
                    .await
                {
                    Ok(groups) => {
                        ApiResponse::success(req.id, serde_json::json!({ "error_groups": groups }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "profiles" | "watchdog.profiles" => match self.kernel.supervision.list_profiles().await {
                Ok(profiles) => {
                    ApiResponse::success(req.id, serde_json::json!({ "profiles": profiles }))
                }
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "ingest" | "telemetry.ingest" | "ingest_telemetry" => {
                let Some(batch_val) = req.params.get("batch").cloned() else {
                    return ApiResponse::error(req.id, "Missing required 'batch' parameter");
                };
                let batch = match serde_json::from_value(batch_val) {
                    Ok(b) => b,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid telemetry batch: {e}")),
                };
                match self.kernel.supervision.ingest_batch(batch).await {
                    Ok(count) => ApiResponse::success(req.id, serde_json::json!({ "ingested_spans": count })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id.clone(), format!("Unknown workflow method: {unknown}")),
        }
    }

    pub(crate) async fn ingest_otlp_traces_bytes(&self, bytes: &[u8]) -> Result<usize, DomainError> {
        let batch = custos_adapters::host::telemetry::normalize_otlp_traces_json(bytes)?;
        self.kernel.supervision.ingest_batch(batch).await
    }

    pub(crate) async fn ingest_otlp_logs_bytes(&self, bytes: &[u8]) -> Result<usize, DomainError> {
        let batch = custos_adapters::host::telemetry::normalize_otlp_logs_json(bytes)?;
        self.kernel.supervision.ingest_batch(batch).await
    }
}
