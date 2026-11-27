use super::{ApiRequest, ApiResponse, LocalApiDispatcher};
use custos_contracts::ProposalStatus;
use custos_integrations::mcp::{
    CustosMcpClient, CustosMcpHostInfo, McpFederationHub,
};
use custos_storage::repositories::{
    McpServerConfigRecord, McpTransportKind, UpsertMcpServerParams,
};
use custos_storage::SqliteTaskStore;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Deserialize)]
struct ToggleMcpParams {
    prefix: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct DeleteMcpParams {
    prefix: String,
}

#[derive(Deserialize)]
struct ListProposalsParams {
    status: Option<ProposalStatus>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct ResolveProposalParams {
    proposal_id: String,
    accept: bool,
}

pub async fn register_enabled_mcp_servers(store: &SqliteTaskStore, hub: &McpFederationHub) {
    let records = match store.mcp_servers().list() {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!("Failed to load persisted MCP servers: {err}");
            return;
        }
    };

    for record in records {
        if record.enabled {
            if let Err(err) = sync_server_into_hub(hub, &record).await {
                tracing::warn!(
                    prefix = %record.prefix,
                    "Failed to register MCP server on startup: {err}"
                );
            }
        }
    }
}

pub async fn sync_server_into_hub(
    hub: &McpFederationHub,
    record: &McpServerConfigRecord,
) -> Result<(), String> {
    if !record.enabled {
        hub.unregister_server(&record.prefix).await;
        return Ok(());
    }

    let secret = record
        .auth_secret_ref
        .as_deref()
        .and_then(|key| std::env::var(key).ok())
        .filter(|v| !v.is_empty());

    let client = match record.transport_kind {
        McpTransportKind::Stdio => {
            let cmd = record
                .command
                .as_deref()
                .ok_or_else(|| "Stdio MCP server missing command".to_string())?;
            let mut env = record.env.clone();
            if let (Some(ref_key), Some(val)) = (&record.auth_secret_ref, &secret) {
                env.insert(ref_key.clone(), val.clone());
            }
            let cwd = record.cwd.as_deref().map(PathBuf::from);
            Arc::new(CustosMcpClient::new_stdio(
                &record.prefix,
                cmd,
                record.args.clone(),
                env,
                cwd,
                CustosMcpHostInfo {
                    explicit_extensions: true,
                    client_name: Some("custos-daemon".into()),
                    client_version: Some(env!("CARGO_PKG_VERSION").into()),
                },
            ))
        }
        McpTransportKind::Http => {
            let url = record
                .url
                .as_deref()
                .ok_or_else(|| "HTTP MCP server missing url".to_string())?;
            Arc::new(CustosMcpClient::new_http(
                &record.prefix,
                url,
                secret,
                record.headers.clone(),
            ))
        }
    };

    hub.register_server(&record.prefix, client)
        .await
        .map_err(|e| e.to_string())
}

impl LocalApiDispatcher {
    pub(crate) async fn handle_mcp(&self, req: ApiRequest) -> ApiResponse {
        let Some(store) = self.kernel.sqlite_store.as_ref() else {
            return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon");
        };
        let hub = self.workbenches.mcp_hub.as_ref();

        match req.method.as_str() {
            "v1.mcp.servers.list" => match store.mcp_servers().list() {
                Ok(servers) => {
                    let active = hub.list_registered_prefixes().await;
                    ApiResponse::success(
                        req.id,
                        serde_json::json!({ "servers": servers, "active_prefixes": active }),
                    )
                }
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "v1.mcp.servers.save" => {
                let params: UpsertMcpServerParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                match store.mcp_servers().upsert(params) {
                    Ok(record) => {
                        if let Err(e) = sync_server_into_hub(hub, &record).await {
                            return ApiResponse::error(req.id, e);
                        }
                        ApiResponse::success(
                            req.id,
                            serde_json::to_value(record).unwrap_or_default(),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.mcp.servers.toggle" => {
                let params: ToggleMcpParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                match store
                    .mcp_servers()
                    .set_enabled(&params.prefix, params.enabled)
                {
                    Ok(record) => {
                        if let Err(e) = sync_server_into_hub(hub, &record).await {
                            return ApiResponse::error(req.id, e);
                        }
                        ApiResponse::success(
                            req.id,
                            serde_json::to_value(record).unwrap_or_default(),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.mcp.servers.delete" => {
                let params: DeleteMcpParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                match store.mcp_servers().delete(&params.prefix) {
                    Ok(()) => {
                        hub.unregister_server(&params.prefix).await;
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({ "deleted": params.prefix }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.mcp.tools.list" => match hub.list_federated_tools().await {
                Ok(tools) => ApiResponse::success(req.id, serde_json::json!({ "tools": tools })),
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            _ => ApiResponse::error(req.id.clone(), format!("Unknown MCP method: {}", req.method)),
        }
    }

    pub(crate) async fn handle_memory(&self, req: ApiRequest) -> ApiResponse {
        let Some(store) = self.kernel.sqlite_store.as_ref() else {
            return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon");
        };

        match req.method.as_str() {
            "v1.memory.proposals.list" => {
                let params: ListProposalsParams = serde_json::from_value(req.params).unwrap_or(
                    ListProposalsParams {
                        status: None,
                        limit: Some(50),
                    },
                );
                match store
                    .memory()
                    .list_proposals(params.status, params.limit.unwrap_or(50))
                {
                    Ok(proposals) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "proposals": proposals }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.memory.proposals.resolve" => {
                let params: ResolveProposalParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid params: {e}")),
                };
                match store
                    .memory()
                    .resolve_proposal(&params.proposal_id, params.accept)
                {
                    Ok(receipt) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(receipt).unwrap_or_default(),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            _ => {
                ApiResponse::error(req.id.clone(), format!("Unknown memory method: {}", req.method))
            }
        }
    }
}

